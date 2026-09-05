//! Versioned wire decoding for the Claude hook envelope — `docs/specs/dashboard/
//! claude.md` R15-R15.1, [`crate::claude::DESIGN.md`].
//!
//! `hook`'s module writes one bounded, newline-delimited, versioned JSON
//! envelope per accepted activity event (R13/R14's fifteen-event allowlist).
//! This module is the receiving half of that contract: the listener reads one
//! line from the Unix socket and calls [`decode_envelope`], which validates
//! the protocol version, the exact `hook`-allowlisted record fields for
//! whichever of the fifteen event kinds the line carries, and every value
//! bound, before constructing the typed [`ClaudeIpcEnvelope`] that
//! [`crate::claude::state`] and [`crate::claude::ClaudeAdapter`] consume. Raw
//! JSON lives only inside a transient `serde_json::Value` scoped to one call
//! and is dropped before anything else runs (`claude.md` R14: never retain
//! the raw value).
//!
//! The decoder never reads an unallowlisted key or value, so an envelope that
//! somehow carries extra fields (which `hook`'s serializer itself never emits)
//! decodes to the same typed record — the extras are ignored, not retained.
//!
//! CONTRACT: ClaudeIpcWireDecoder (`docs/specs/dashboard/claude.md`
//! R13-R15.1; `crates/dashboard/src/claude/DESIGN.md`)
//!
//! GUARANTEES:
//!   - [`decode_envelope`] turns exactly one bounded, newline-delimited
//!     `hook`-produced envelope into a typed `ClaudeIpcEnvelope` when the wire
//!     carries protocol version 1 and only the exact allowlisted fields for
//!     one of the fifteen R13 event kinds (`session_id`, `cwd`, event kind,
//!     the per-kind R14 fields, `received_at`). Every other key and value is
//!     never read.
//!   - Malformed JSON, a missing/wrong/unknown protocol version, an unknown
//!     event kind, and out-of-bounds values (empty/oversized session id or
//!     cwd, an over-length label, a line over the envelope bound, an embedded
//!     newline, a `received_at` outside the shared `Timestamp` range) are
//!     rejected with a category-only [`DecodeError`] that never carries the
//!     rejected value or raw JSON.
//!   - The transient `serde_json::Value` never escapes this module and never
//!     appears in state or logs.
//!
//! EXPECTS:
//!   - The listener to hand [`decode_envelope`] exactly one line (optionally
//!     newline-terminated) produced by `hook`'s `serialize_envelope`/
//!     `deliver_to`.
//!
//! FAILURE BEHAVIOR:
//!   - Every rejection returns `Err(DecodeError)` with only the category;
//!     callers log the category, never the payload. Neither `state` nor the
//!     adapter ever sees a partial or unvalidated envelope.
//!
//! DOES NOT:
//!   - Open sockets, touch Claude configuration or transcripts, retain raw
//!     JSON, or accept any protocol other than version 1.

use serde_json::{Map, Value};

use super::hook::{
    ClaudeEvent, ClaudeHookRecord, ClaudeIpcEnvelope, ReceivedAt, SessionEndReason,
    SessionStartSource, ENVELOPE_PROTOCOL_VERSION, MAX_CWD_LEN, MAX_ENVELOPE_BYTES,
    MAX_LABEL_LEN, MAX_SESSION_ID_LEN,
};

/// Why a wire line was rejected. Category-only on purpose: the rejected
/// input's content must never appear in the error, in logs, or in state
/// (`claude.md` R14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Not valid JSON, a required allowlisted field is missing or mistyped,
    /// or an allowlisted metadata/label value is unverified (outside a
    /// closed set, or a label over its length bound).
    Malformed,
    /// `protocol_version` is present but not the sole supported version 1.
    UnknownVersion,
    /// The event `kind` is not one of the fifteen R13 allowlisted kinds.
    UnknownEvent,
    /// A value violates a hard identity bound: an empty session id or cwd,
    /// an oversized UTF-8 byte value, a line longer than `MAX_ENVELOPE_BYTES`
    /// or containing more than one newline, or a `received_at` outside the
    /// shared `Timestamp` range.
    OutOfBounds,
}

impl DecodeError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Malformed => "malformed envelope",
            Self::UnknownVersion => "unknown protocol version",
            Self::UnknownEvent => "unknown event kind",
            Self::OutOfBounds => "out of bounds",
        }
    }
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Decode one bounded, newline-delimited `hook`-produced envelope into its
/// typed form (`claude.md` R15/R15.1). This is the public decoder the
/// listener calls after reading one line from the socket.
///
/// The input is deserialized into a transient `serde_json::Value` strictly
/// for extraction — the same never-retain pattern `hook`'s own
/// `parse_hook_input` uses — and only allowlisted keys are ever read. The
/// original JSON and every rejected value are dropped before the typed
/// envelope is returned.
pub fn decode_envelope(line: &str) -> Result<ClaudeIpcEnvelope, DecodeError> {
    // Hard frame bound first: a line over the serialized-envelope cap is
    // rejected before any parsing, matching `hook`'s own MAX_ENVELOPE_BYTES
    // serializer bound (which includes the trailing newline).
    if line.len() > MAX_ENVELOPE_BYTES {
        return Err(DecodeError::OutOfBounds);
    }
    // One optional trailing newline; anything else must be a single line.
    let body = line.strip_suffix('\n').unwrap_or(line);
    if body.as_bytes().contains(&b'\n') {
        return Err(DecodeError::OutOfBounds);
    }

    let raw: Value = serde_json::from_str(body).map_err(|_| DecodeError::Malformed)?;

    let protocol_version = raw
        .get("protocol_version")
        .and_then(Value::as_u64)
        .ok_or(DecodeError::Malformed)?;
    if protocol_version != u64::from(ENVELOPE_PROTOCOL_VERSION) {
        return Err(DecodeError::UnknownVersion);
    }

    let record = raw
        .get("record")
        .and_then(Value::as_object)
        .ok_or(DecodeError::Malformed)?;

    let session_id = record
        .get("session_id")
        .and_then(Value::as_str)
        .ok_or(DecodeError::Malformed)?;
    if !valid_session_id(session_id) {
        return Err(DecodeError::OutOfBounds);
    }

    let cwd = record
        .get("cwd")
        .and_then(Value::as_str)
        .ok_or(DecodeError::Malformed)?;
    if !valid_cwd(cwd) {
        return Err(DecodeError::OutOfBounds);
    }

    // received_at must fit the shared snapshot `Timestamp` (i64 epoch millis);
    // a value beyond that range cannot be represented at the provider-neutral
    // boundary, so it is an out-of-bounds rejection rather than a silent wrap.
    let received_at = record
        .get("received_at")
        .and_then(Value::as_u64)
        .ok_or(DecodeError::Malformed)?;
    if received_at > i64::MAX as u64 {
        return Err(DecodeError::OutOfBounds);
    }

    let event = record
        .get("event")
        .and_then(Value::as_object)
        .ok_or(DecodeError::Malformed)?;
    let kind = event
        .get("kind")
        .and_then(Value::as_str)
        .ok_or(DecodeError::Malformed)?;
    let event = decode_event(kind, event)?;

    Ok(ClaudeIpcEnvelope {
        protocol_version: ENVELOPE_PROTOCOL_VERSION,
        record: ClaudeHookRecord {
            session_id: session_id.to_owned(),
            cwd: cwd.to_owned(),
            event,
            received_at: ReceivedAt(received_at),
        },
    })
}

fn decode_event(kind: &str, event: &Map<String, Value>) -> Result<ClaudeEvent, DecodeError> {
    match kind {
        "session_start" => {
            let source = match event.get("source") {
                None | Some(Value::Null) => None,
                Some(Value::String(label)) => {
                    Some(SessionStartSource::parse(label).ok_or(DecodeError::Malformed)?)
                }
                Some(_) => return Err(DecodeError::Malformed),
            };
            let model = read_optional_label(event, "model")?;
            Ok(ClaudeEvent::SessionStart { source, model })
        }
        "user_prompt_submit" => {
            let prompt = read_required_text(event, "prompt")?;
            Ok(ClaudeEvent::UserPromptSubmit { prompt })
        }
        "pre_tool_use" => {
            let tool_name = read_required_label(event, "tool_name")?;
            let tool_use_id = read_required_label(event, "tool_use_id")?;
            let tool_input = read_required_text(event, "tool_input")?;
            let agent_id = read_optional_label(event, "agent_id")?;
            let agent_type = read_optional_label(event, "agent_type")?;
            Ok(ClaudeEvent::PreToolUse {
                tool_name,
                tool_use_id,
                tool_input,
                agent_id,
                agent_type,
            })
        }
        "post_tool_use" => {
            let tool_name = read_required_label(event, "tool_name")?;
            let tool_use_id = read_required_label(event, "tool_use_id")?;
            let tool_input = read_required_text(event, "tool_input")?;
            let tool_response = read_required_text(event, "tool_response")?;
            let agent_id = read_optional_label(event, "agent_id")?;
            let agent_type = read_optional_label(event, "agent_type")?;
            Ok(ClaudeEvent::PostToolUse {
                tool_name,
                tool_use_id,
                tool_input,
                tool_response,
                agent_id,
                agent_type,
            })
        }
        "post_tool_use_failure" => {
            let tool_name = read_required_label(event, "tool_name")?;
            let tool_use_id = read_required_label(event, "tool_use_id")?;
            let tool_input = read_required_text(event, "tool_input")?;
            let error = read_required_text(event, "error")?;
            let error_type = read_optional_label(event, "error_type")?;
            let agent_id = read_optional_label(event, "agent_id")?;
            let agent_type = read_optional_label(event, "agent_type")?;
            Ok(ClaudeEvent::PostToolUseFailure {
                tool_name,
                tool_use_id,
                tool_input,
                error,
                error_type,
                agent_id,
                agent_type,
            })
        }
        "permission_request" => {
            let tool_name = read_required_label(event, "tool_name")?;
            let tool_use_id = read_required_label(event, "tool_use_id")?;
            let tool_input = read_required_text(event, "tool_input")?;
            Ok(ClaudeEvent::PermissionRequest {
                tool_name,
                tool_use_id,
                tool_input,
            })
        }
        "permission_denied" => {
            let tool_name = read_required_label(event, "tool_name")?;
            let tool_use_id = read_required_label(event, "tool_use_id")?;
            let denial_reason = read_optional_text(event, "denial_reason")?;
            Ok(ClaudeEvent::PermissionDenied {
                tool_name,
                tool_use_id,
                denial_reason,
            })
        }
        "elicitation" => {
            let tool_use_id = read_required_label(event, "tool_use_id")?;
            let server_name = read_required_label(event, "server_name")?;
            let elicitation_request = read_required_text(event, "elicitation_request")?;
            Ok(ClaudeEvent::Elicitation {
                tool_use_id,
                server_name,
                elicitation_request,
            })
        }
        "elicitation_result" => {
            let tool_use_id = read_required_label(event, "tool_use_id")?;
            let server_name = read_required_label(event, "server_name")?;
            let user_response = read_required_text(event, "user_response")?;
            Ok(ClaudeEvent::ElicitationResult {
                tool_use_id,
                server_name,
                user_response,
            })
        }
        "notification" => {
            let notification_type = read_optional_label(event, "notification_type")?;
            let notification_message = read_required_text(event, "notification_message")?;
            Ok(ClaudeEvent::Notification {
                notification_type,
                notification_message,
            })
        }
        "stop" => {
            let last_assistant_message = read_required_text(event, "last_assistant_message")?;
            let agent_id = read_optional_label(event, "agent_id")?;
            let agent_type = read_optional_label(event, "agent_type")?;
            Ok(ClaudeEvent::Stop {
                last_assistant_message,
                agent_id,
                agent_type,
            })
        }
        "stop_failure" => {
            let error_type = read_optional_label(event, "error_type")?;
            Ok(ClaudeEvent::StopFailure { error_type })
        }
        "subagent_start" => {
            let agent_id = read_required_label(event, "agent_id")?;
            let agent_type = read_optional_label(event, "agent_type")?;
            let agent_prompt = read_required_text(event, "agent_prompt")?;
            Ok(ClaudeEvent::SubagentStart {
                agent_id,
                agent_type,
                agent_prompt,
            })
        }
        "subagent_stop" => {
            let agent_id = read_required_label(event, "agent_id")?;
            let agent_type = read_optional_label(event, "agent_type")?;
            let last_assistant_message = read_required_text(event, "last_assistant_message")?;
            let stop_reason = read_optional_label(event, "stop_reason")?;
            Ok(ClaudeEvent::SubagentStop {
                agent_id,
                agent_type,
                last_assistant_message,
                stop_reason,
            })
        }
        "session_end" => {
            let reason = match event.get("reason") {
                None | Some(Value::Null) => None,
                Some(Value::String(label)) => {
                    Some(SessionEndReason::parse(label).ok_or(DecodeError::Malformed)?)
                }
                Some(_) => return Err(DecodeError::Malformed),
            };
            Ok(ClaudeEvent::SessionEnd { reason })
        }
        _ => Err(DecodeError::UnknownEvent),
    }
}

/// Reads a required "(bounded)" text field: present and a JSON string. No
/// additional length re-check — the overall envelope size bound already
/// gates total wire size, and `hook` truncates before it ever writes.
fn read_required_text(event: &Map<String, Value>, key: &str) -> Result<String, DecodeError> {
    event
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(DecodeError::Malformed)
}

/// Reads an optional "(bounded)" text field: absent/null is `None`; present
/// must be a JSON string. `denial_reason` is the only field of this shape
/// (`claude.md` R14). No length re-check, matching `read_required_text`.
fn read_optional_text(
    event: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, DecodeError> {
    match event.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(DecodeError::Malformed),
    }
}

fn valid_label(value: &str) -> bool {
    value.len() <= MAX_LABEL_LEN
}

fn read_required_label(event: &Map<String, Value>, key: &str) -> Result<String, DecodeError> {
    match event.get(key) {
        Some(Value::String(value)) if valid_label(value) => Ok(value.clone()),
        _ => Err(DecodeError::Malformed),
    }
}

fn read_optional_label(
    event: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, DecodeError> {
    match event.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if valid_label(value) => Ok(Some(value.clone())),
        Some(_) => Err(DecodeError::Malformed),
    }
}

fn valid_session_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_SESSION_ID_LEN
}

fn valid_cwd(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_CWD_LEN
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::hook::{
        parse_hook_input, serialize_envelope, ParseOutcome, MAX_HOOK_INPUT_BYTES,
    };

    const RECEIVED: u64 = 1_700_000_000_000;

    fn accepted(input: &str) -> ClaudeHookRecord {
        match parse_hook_input(input, ReceivedAt(RECEIVED)) {
            ParseOutcome::Accepted(record) => record,
            ParseOutcome::Dropped(reason) => panic!("expected accepted hook input, got {reason:?}"),
        }
    }

    fn wire(source: Option<&str>) -> String {
        let json = format!(
            "{{\"hook_event_name\":\"SessionStart\",\"session_id\":\"sess-1\",\"cwd\":\"/work/proj\"{}}}",
            match source {
                Some(source) => format!(",\"source\":\"{source}\""),
                None => String::new(),
            }
        );
        serialize_envelope(&accepted(&json)).expect("ordinary envelope must fit")
    }

    fn assert_rejected(line: &str, expected: DecodeError) {
        assert_eq!(decode_envelope(line), Err(expected), "line {line:?}");
    }

    #[test]
    fn decodes_a_hook_serialized_start_envelope_round_trip() {
        let line = wire(Some("startup"));
        let decoded = decode_envelope(&line).expect("decode round trip");
        assert_eq!(decoded.protocol_version, ENVELOPE_PROTOCOL_VERSION);
        assert_eq!(decoded.record.session_id, "sess-1");
        assert_eq!(decoded.record.cwd, "/work/proj");
        assert_eq!(decoded.record.received_at, ReceivedAt(RECEIVED));
        assert_eq!(
            decoded.record.event,
            ClaudeEvent::SessionStart {
                source: Some(SessionStartSource::Startup),
                model: None,
            }
        );
    }

    #[test]
    fn trailing_newline_is_accepted_and_embedded_newline_is_not() {
        let line = wire(None);
        assert!(decode_envelope(&line).is_ok());
        assert!(decode_envelope(line.trim_end()).is_ok());
        assert_rejected(&format!("{line}\n"), DecodeError::OutOfBounds);
    }

    /// Every one of the fifteen allowlisted events round-trips through
    /// `hook::parse_hook_input` -> `serialize_envelope` -> `decode_envelope`
    /// with every R14 field it carries surviving intact.
    #[test]
    fn every_allowlisted_event_round_trips_with_its_r14_fields() {
        let cases: Vec<(&str, ClaudeEvent)> = vec![
            (
                r#"{"hook_event_name":"SessionStart","session_id":"s","cwd":"/w","source":"resume","model":"claude-sonnet"}"#,
                ClaudeEvent::SessionStart {
                    source: Some(SessionStartSource::Resume),
                    model: Some("claude-sonnet".to_string()),
                },
            ),
            (
                r#"{"hook_event_name":"SessionStart","session_id":"s","cwd":"/w"}"#,
                ClaudeEvent::SessionStart {
                    source: None,
                    model: None,
                },
            ),
            (
                r#"{"hook_event_name":"UserPromptSubmit","session_id":"s","cwd":"/w","prompt":"fix the bug"}"#,
                ClaudeEvent::UserPromptSubmit {
                    prompt: "fix the bug".to_string(),
                },
            ),
            (
                r#"{"hook_event_name":"PreToolUse","session_id":"s","cwd":"/w","tool_name":"Edit","tool_use_id":"c1","tool_input":{"path":"a"},"agent_id":"a1","agent_type":"general-purpose"}"#,
                ClaudeEvent::PreToolUse {
                    tool_name: "Edit".to_string(),
                    tool_use_id: "c1".to_string(),
                    tool_input: "{\"path\":\"a\"}".to_string(),
                    agent_id: Some("a1".to_string()),
                    agent_type: Some("general-purpose".to_string()),
                },
            ),
            (
                r#"{"hook_event_name":"PostToolUse","session_id":"s","cwd":"/w","tool_name":"Bash","tool_use_id":"c2","tool_input":{"cmd":"ls"},"tool_response":"ok"}"#,
                ClaudeEvent::PostToolUse {
                    tool_name: "Bash".to_string(),
                    tool_use_id: "c2".to_string(),
                    tool_input: "{\"cmd\":\"ls\"}".to_string(),
                    tool_response: "ok".to_string(),
                    agent_id: None,
                    agent_type: None,
                },
            ),
            (
                r#"{"hook_event_name":"PostToolUseFailure","session_id":"s","cwd":"/w","tool_name":"Bash","tool_use_id":"c3","tool_input":{},"error":"boom","error_type":"nonzero_exit"}"#,
                ClaudeEvent::PostToolUseFailure {
                    tool_name: "Bash".to_string(),
                    tool_use_id: "c3".to_string(),
                    tool_input: "{}".to_string(),
                    error: "boom".to_string(),
                    error_type: Some("nonzero_exit".to_string()),
                    agent_id: None,
                    agent_type: None,
                },
            ),
            (
                r#"{"hook_event_name":"PermissionRequest","session_id":"s","cwd":"/w","tool_name":"Bash","tool_use_id":"c9","tool_input":{"cmd":"rm"}}"#,
                ClaudeEvent::PermissionRequest {
                    tool_name: "Bash".to_string(),
                    tool_use_id: "c9".to_string(),
                    tool_input: "{\"cmd\":\"rm\"}".to_string(),
                },
            ),
            (
                r#"{"hook_event_name":"PermissionDenied","session_id":"s","cwd":"/w","tool_name":"Bash","tool_use_id":"c9","denial_reason":"policy"}"#,
                ClaudeEvent::PermissionDenied {
                    tool_name: "Bash".to_string(),
                    tool_use_id: "c9".to_string(),
                    denial_reason: Some("policy".to_string()),
                },
            ),
            (
                r#"{"hook_event_name":"PermissionDenied","session_id":"s","cwd":"/w","tool_name":"Bash","tool_use_id":"c9"}"#,
                ClaudeEvent::PermissionDenied {
                    tool_name: "Bash".to_string(),
                    tool_use_id: "c9".to_string(),
                    denial_reason: None,
                },
            ),
            (
                r#"{"hook_event_name":"Elicitation","session_id":"s","cwd":"/w","tool_use_id":"c10","server_name":"mcp","elicitation_request":"confirm?"}"#,
                ClaudeEvent::Elicitation {
                    tool_use_id: "c10".to_string(),
                    server_name: "mcp".to_string(),
                    elicitation_request: "confirm?".to_string(),
                },
            ),
            (
                r#"{"hook_event_name":"ElicitationResult","session_id":"s","cwd":"/w","tool_use_id":"c10","server_name":"mcp","user_response":"yes"}"#,
                ClaudeEvent::ElicitationResult {
                    tool_use_id: "c10".to_string(),
                    server_name: "mcp".to_string(),
                    user_response: "yes".to_string(),
                },
            ),
            (
                r#"{"hook_event_name":"Notification","session_id":"s","cwd":"/w","notification_type":"permission","notification_message":"waiting"}"#,
                ClaudeEvent::Notification {
                    notification_type: Some("permission".to_string()),
                    notification_message: "waiting".to_string(),
                },
            ),
            (
                r#"{"hook_event_name":"Stop","session_id":"s","cwd":"/w","last_assistant_message":"done","agent_id":"a9"}"#,
                ClaudeEvent::Stop {
                    last_assistant_message: "done".to_string(),
                    agent_id: Some("a9".to_string()),
                    agent_type: None,
                },
            ),
            (
                r#"{"hook_event_name":"StopFailure","session_id":"s","cwd":"/w","error_type":"timeout"}"#,
                ClaudeEvent::StopFailure {
                    error_type: Some("timeout".to_string()),
                },
            ),
            (
                r#"{"hook_event_name":"SubagentStart","session_id":"s","cwd":"/w","agent_id":"a1","agent_type":"general-purpose","agent_prompt":"investigate"}"#,
                ClaudeEvent::SubagentStart {
                    agent_id: "a1".to_string(),
                    agent_type: Some("general-purpose".to_string()),
                    agent_prompt: "investigate".to_string(),
                },
            ),
            (
                r#"{"hook_event_name":"SubagentStop","session_id":"s","cwd":"/w","agent_id":"a1","last_assistant_message":"found it","stop_reason":"completed"}"#,
                ClaudeEvent::SubagentStop {
                    agent_id: "a1".to_string(),
                    agent_type: None,
                    last_assistant_message: "found it".to_string(),
                    stop_reason: Some("completed".to_string()),
                },
            ),
            (
                r#"{"hook_event_name":"SessionEnd","session_id":"s","cwd":"/w","reason":"other"}"#,
                ClaudeEvent::SessionEnd {
                    reason: Some(SessionEndReason::Other),
                },
            ),
            (
                r#"{"hook_event_name":"SessionEnd","session_id":"s","cwd":"/w"}"#,
                ClaudeEvent::SessionEnd { reason: None },
            ),
        ];
        for (input, expected) in cases {
            let line = serialize_envelope(&accepted(input)).expect("ordinary envelope must fit");
            let decoded = decode_envelope(&line).expect("decode");
            assert_eq!(decoded.record.event, expected, "input {input}");
        }
    }

    #[test]
    fn malformed_json_and_shapes_are_rejected() {
        for line in [
            "",
            "   ",
            "not json",
            "{\"protocol_version\":",
            "[]",
            "null",
            "42",
            "\"a string\"",
            // Missing protocol_version.
            "{\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"stop_failure\"},\"received_at\":1}}",
            // protocol_version of the wrong type.
            "{\"protocol_version\":\"1\",\"record\":{}}",
            "{\"protocol_version\":1.0,\"record\":{}}",
            // record not an object.
            "{\"protocol_version\":1,\"record\":[1]}",
            // session_id wrong type.
            "{\"protocol_version\":1,\"record\":{\"session_id\":5,\"cwd\":\"/w\",\"event\":{\"kind\":\"session_start\"},\"received_at\":1}}",
            // cwd missing.
            "{\"protocol_version\":1,\"record\":{\"session_id\":\"s\",\"event\":{\"kind\":\"session_start\"},\"received_at\":1}}",
            // event not an object.
            "{\"protocol_version\":1,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":\"session_start\",\"received_at\":1}}",
            // received_at wrong type / missing.
            "{\"protocol_version\":1,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"session_start\"},\"received_at\":\"now\"}}",
            "{\"protocol_version\":1,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"session_start\"}}}",
            // PreToolUse missing a required field.
            "{\"protocol_version\":1,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"pre_tool_use\",\"tool_name\":\"Edit\"},\"received_at\":1}}",
        ] {
            assert_rejected(line, DecodeError::Malformed);
        }
    }

    #[test]
    fn unknown_or_missing_protocol_versions_are_rejected() {
        assert_rejected(
            "{\"protocol_version\":2,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"session_start\"},\"received_at\":1}}",
            DecodeError::UnknownVersion,
        );
        assert_rejected(
            "{\"protocol_version\":99,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"session_end\"},\"received_at\":1}}",
            DecodeError::UnknownVersion,
        );
    }

    #[test]
    fn unknown_event_kinds_are_rejected() {
        for kind in [
            "session_stop",
            "SessionStart", // the hook-event name, not the envelope kind
            "user_prompt_submit_v2",
            "worktree_create",
            "",
        ] {
            assert_rejected(
                &format!("{{\"protocol_version\":1,\"record\":{{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{{\"kind\":\"{kind}\"}},\"received_at\":1}}}}"),
                DecodeError::UnknownEvent,
            );
        }
    }

    #[test]
    fn unverified_metadata_and_oversized_labels_are_rejected() {
        // source outside the closed SessionStartSource set.
        assert_rejected(
            "{\"protocol_version\":1,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"session_start\",\"source\":\"forked-over-ssh\"},\"received_at\":1}}",
            DecodeError::Malformed,
        );
        // source present but not a string.
        assert_rejected(
            "{\"protocol_version\":1,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"session_start\",\"source\":5},\"received_at\":1}}",
            DecodeError::Malformed,
        );
        // reason outside the closed SessionEndReason set.
        assert_rejected(
            "{\"protocol_version\":1,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"session_end\",\"reason\":\"user_abandoned\"},\"received_at\":1}}",
            DecodeError::Malformed,
        );
        // a required label over MAX_LABEL_LEN.
        let long_label = "i".repeat(MAX_LABEL_LEN + 1);
        assert_rejected(
            &format!(
                "{{\"protocol_version\":1,\"record\":{{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{{\"kind\":\"pre_tool_use\",\"tool_name\":\"{long_label}\",\"tool_use_id\":\"c\",\"tool_input\":\"{{}}\"}},\"received_at\":1}}}}"
            ),
            DecodeError::Malformed,
        );
        // stop_failure's sensitive label (hook never emits it) is never read:
        // the kind still decodes, and the label exists nowhere on the result.
        let line = "{\"protocol_version\":1,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"stop_failure\",\"error\":\"SENTINEL_ERROR\"},\"received_at\":1}}";
        let decoded = decode_envelope(line).expect("stop_failure decodes");
        assert_eq!(
            decoded.record.event,
            ClaudeEvent::StopFailure { error_type: None }
        );
        assert!(!format!("{:?}", decoded).contains("SENTINEL_ERROR"));
    }

    #[test]
    fn out_of_bounds_ids_paths_and_received_at_are_rejected() {
        for session_id in ["", "   ", &"i".repeat(MAX_SESSION_ID_LEN + 1)] {
            assert_rejected(
                &format!("{{\"protocol_version\":1,\"record\":{{\"session_id\":\"{session_id}\",\"cwd\":\"/w\",\"event\":{{\"kind\":\"session_start\"}},\"received_at\":1}}}}"),
                DecodeError::OutOfBounds,
            );
        }
        for cwd in ["", "   ", &"p".repeat(MAX_CWD_LEN + 1)] {
            assert_rejected(
                &format!("{{\"protocol_version\":1,\"record\":{{\"session_id\":\"s\",\"cwd\":\"{cwd}\",\"event\":{{\"kind\":\"session_start\"}},\"received_at\":1}}}}"),
                DecodeError::OutOfBounds,
            );
        }

        let bounded_id = "é".repeat(MAX_SESSION_ID_LEN / 2);
        let bounded_id_line = format!(
            "{{\"protocol_version\":1,\"record\":{{\"session_id\":\"{bounded_id}\",\"cwd\":\"/w\",\"event\":{{\"kind\":\"session_start\"}},\"received_at\":1}}}}"
        );
        assert!(decode_envelope(&bounded_id_line).is_ok());

        let oversized_id = "é".repeat(MAX_SESSION_ID_LEN / 2 + 1);
        let oversized_id_line = format!(
            "{{\"protocol_version\":1,\"record\":{{\"session_id\":\"{oversized_id}\",\"cwd\":\"/w\",\"event\":{{\"kind\":\"session_start\"}},\"received_at\":1}}}}"
        );
        assert_rejected(&oversized_id_line, DecodeError::OutOfBounds);

        let bounded_cwd = "é".repeat(MAX_CWD_LEN / 2);
        let bounded_cwd_line = format!(
            "{{\"protocol_version\":1,\"record\":{{\"session_id\":\"s\",\"cwd\":\"{bounded_cwd}\",\"event\":{{\"kind\":\"session_start\"}},\"received_at\":1}}}}"
        );
        assert!(decode_envelope(&bounded_cwd_line).is_ok());

        let oversized_cwd = "é".repeat(MAX_CWD_LEN / 2 + 1);
        let oversized_cwd_line = format!(
            "{{\"protocol_version\":1,\"record\":{{\"session_id\":\"s\",\"cwd\":\"{oversized_cwd}\",\"event\":{{\"kind\":\"session_start\"}},\"received_at\":1}}}}"
        );
        assert_rejected(&oversized_cwd_line, DecodeError::OutOfBounds);

        // received_at beyond the shared i64 Timestamp range.
        assert_rejected(
            "{\"protocol_version\":1,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"session_start\"},\"received_at\":18446744073709551615}}",
            DecodeError::OutOfBounds,
        );
    }

    #[test]
    fn oversized_lines_are_rejected_before_parsing() {
        let pad = "x".repeat(MAX_HOOK_INPUT_BYTES + 1);
        let line = format!(
            "{{\"protocol_version\":1,\"record\":{{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{{\"kind\":\"session_start\"}},\"received_at\":1,\"pad\":\"{pad}\"}}}}"
        );
        assert!(line.len() > MAX_ENVELOPE_BYTES);
        assert_rejected(&line, DecodeError::OutOfBounds);
        // A hook-produced max-bounded record must still fit (see hook tests
        // for the serializer side); here just prove the bound is the
        // encoder's own.
        let max = accepted(&format!(
            "{{\"hook_event_name\":\"SessionStart\",\"session_id\":\"{}\",\"cwd\":\"{}\"}}",
            "i".repeat(MAX_SESSION_ID_LEN),
            "/".repeat(MAX_CWD_LEN),
        ));
        let line = serialize_envelope(&max).expect("maximum ASCII envelope must fit");
        assert!(line.len() <= MAX_ENVELOPE_BYTES);
        assert!(decode_envelope(&line).is_ok());
    }

    #[test]
    fn extra_unallowlisted_keys_are_never_read() {
        let line = "{\"protocol_version\":1,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"session_start\"},\"received_at\":7,\"transcript_path\":\"SENTINEL_TRANSCRIPT\",\"payload\":{\"prompt\":\"SENTINEL_PROMPT\"}}}";
        let decoded = decode_envelope(line).expect("extra keys must not invalidate the record");
        assert_eq!(decoded.record.session_id, "s");
        assert_eq!(decoded.record.cwd, "/w");
        assert_eq!(decoded.protocol_version, ENVELOPE_PROTOCOL_VERSION);
        // The unallowlisted values exist nowhere on the typed record.
        assert!(!format!("{:?}", decoded).contains("SENTINEL"));
    }

    #[test]
    fn rejected_inputs_never_expose_values_in_the_error() {
        assert_rejected("SENTINEL_MALFORMED{", DecodeError::Malformed);
        let line = format!(
            "{{\"protocol_version\":1,\"record\":{{\"session_id\":\"{}\",\"cwd\":\"/w\",\"event\":{{\"kind\":\"unknown\"}},\"received_at\":1}}}}",
            "SENTINEL_ID"        );
        let err = decode_envelope(&line).unwrap_err();
        assert_eq!(err, DecodeError::UnknownEvent);
        assert!(!format!("{err:?}").contains("SENTINEL"));
    }
}
