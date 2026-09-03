//! Versioned wire decoding for the Claude hook envelope — `docs/specs/dashboard/
//! claude.md` R15-R15.1, T03 [`crate::claude::DESIGN.md`].
//!
//! T02's `hook` module writes one bounded, newline-delimited, versioned JSON
//! envelope per accepted lifecycle event. This module is the receiving half of
//! that contract: the T04 listener reads one line from the Unix socket and
//! calls [`decode_envelope`], which validates the protocol version, the exact
//! T02 allowlisted record fields, and every value bound before constructing the
//! typed T02 [`ClaudeIpcEnvelope`] that [`crate::claude::state`] and
//! [`crate::claude::ClaudeAdapter`] consume. Raw JSON lives only inside a
//! transient `serde_json::Value` scoped to one call and is dropped before
//! anything else runs (`claude.md` R14: never retain the raw value).
//!
//! The decoder never reads an unallowlisted key or value, so an envelope that
//! somehow carries extra fields (which T02's serializer itself never emits)
//! decodes to the same typed record — the extras are ignored, not retained.
//!
//! CONTRACT: ClaudeIpcWireDecoder (T03; `docs/specs/dashboard/claude.md`
//! R13-R15.1; `crates/dashboard/src/claude/DESIGN.md`)
//!
//! GUARANTEES:
//!   - [`decode_envelope`] turns exactly one bounded, newline-delimited T02
//!     envelope into a typed `ClaudeIpcEnvelope` when the wire carries protocol
//!     version 1 and only the exact T02 allowlisted fields (`session_id`, `cwd`,
//!     event kind, the allowlisted `source`/`reason` labels, `received_at`).
//!     Every other key and value is never read.
//!   - Malformed JSON, a missing/wrong/unknown protocol version, an unknown
//!     event kind, and out-of-bounds values (empty/oversized session id or cwd,
//!     a line over the envelope bound, an embedded newline, a `received_at`
//!     outside the shared `Timestamp` range) are rejected with a category-only
//!     [`DecodeError`] that never carries the rejected value or raw JSON.
//!   - The transient `serde_json::Value` never escapes this module and never
//!     appears in state or logs.
//!
//! EXPECTS:
//!   - T04 to hand [`decode_envelope`] exactly one line (optionally
//!     newline-terminated) produced by T02's `serialize_envelope`/`deliver_to`.
//!
//! FAILURE BEHAVIOR:
//!   - Every rejection returns `Err(DecodeError)` with only the category;
//!     callers log the category, never the payload. Neither `state` nor the
//!     adapter ever sees a partial or unvalidated envelope.
//!
//! DOES NOT:
//!   - Open sockets, touch Claude configuration or transcripts, retain raw JSON,
//!     or accept any protocol other than version 1.

use serde_json::Value;

use super::hook::{
    ClaudeEvent, ClaudeHookRecord, ClaudeIpcEnvelope, ReceivedAt, SessionEndReason,
    SessionStartSource, ENVELOPE_PROTOCOL_VERSION, MAX_CWD_LEN, MAX_ENVELOPE_BYTES,
    MAX_SESSION_ID_LEN,
};

/// Why a wire line was rejected. Category-only on purpose: the rejected
/// input's content must never appear in the error, in logs, or in state
/// (`claude.md` R14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Not valid JSON, a required allowlisted field is missing or mistyped,
    /// or an allowlisted metadata label carries an unverified value.
    Malformed,
    /// `protocol_version` is present but not the sole supported version 1.
    UnknownVersion,
    /// The event `kind` is not one of T02's allowlisted three.
    UnknownEvent,
    /// A value violates a hard bound: an empty/oversized session id or cwd, a
    /// line longer than `MAX_ENVELOPE_BYTES` or containing more than one
    /// newline, or a `received_at` outside the shared `Timestamp` range.
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

/// Decode one bounded, newline-delimited T02 envelope into its typed form
/// (`claude.md` R15/R15.1). This is the public decoder T04 calls after reading
/// one line from the listener socket.
///
/// The input is deserialized into a transient `serde_json::Value` strictly for
/// extraction — the same never-retain pattern T02's own `parse_hook_input`
/// uses — and only allowlisted keys are ever read. The original JSON and every
/// rejected value are dropped before the typed envelope is returned.
pub fn decode_envelope(line: &str) -> Result<ClaudeIpcEnvelope, DecodeError> {
    // Hard frame bound first: a line over the serialized-envelope cap is
    // rejected before any parsing, matching T02's own MAX_ENVELOPE_BYTES
    // serializer assertion (which includes the trailing newline).
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
    let event = match kind {
        "session_start" => {
            let source = match event.get("source") {
                None | Some(Value::Null) => None,
                Some(Value::String(label)) => {
                    Some(SessionStartSource::parse(label).ok_or(DecodeError::Malformed)?)
                }
                Some(_) => return Err(DecodeError::Malformed),
            };
            ClaudeEvent::SessionStart { source }
        }
        "stop_failure" => ClaudeEvent::StopFailure,
        "session_end" => {
            let reason = match event.get("reason") {
                None | Some(Value::Null) => None,
                Some(Value::String(label)) => {
                    Some(SessionEndReason::parse(label).ok_or(DecodeError::Malformed)?)
                }
                Some(_) => return Err(DecodeError::Malformed),
            };
            ClaudeEvent::SessionEnd { reason }
        }
        _ => return Err(DecodeError::UnknownEvent),
    };

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
        serialize_envelope(&accepted(&json))
    }

    fn assert_rejected(line: &str, expected: DecodeError) {
        assert_eq!(decode_envelope(line), Err(expected), "line {line:?}");
    }

    #[test]
    fn decodes_a_t02_serialized_start_envelope_round_trip() {
        let line = wire(Some("startup"));
        let decoded = decode_envelope(&line).expect("decode round trip");
        assert_eq!(decoded.protocol_version, ENVELOPE_PROTOCOL_VERSION);
        assert_eq!(decoded.record.session_id, "sess-1");
        assert_eq!(decoded.record.cwd, "/work/proj");
        assert_eq!(decoded.record.received_at, ReceivedAt(RECEIVED));
        assert_eq!(
            decoded.record.event,
            ClaudeEvent::SessionStart {
                source: Some(SessionStartSource::Startup)
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

    #[test]
    fn every_event_kind_round_trips_with_and_without_metadata() {
        for (input, expected) in [
            (
                "{\"hook_event_name\":\"SessionStart\",\"session_id\":\"s\",\"cwd\":\"/w\",\"source\":\"resume\"}",
                ClaudeEvent::SessionStart { source: Some(SessionStartSource::Resume) },
            ),
            (
                "{\"hook_event_name\":\"SessionStart\",\"session_id\":\"s\",\"cwd\":\"/w\"}",
                ClaudeEvent::SessionStart { source: None },
            ),
            (
                "{\"hook_event_name\":\"StopFailure\",\"session_id\":\"s\",\"cwd\":\"/w\"}",
                ClaudeEvent::StopFailure,
            ),
            (
                "{\"hook_event_name\":\"SessionEnd\",\"session_id\":\"s\",\"cwd\":\"/w\",\"reason\":\"other\"}",
                ClaudeEvent::SessionEnd { reason: Some(SessionEndReason::Other) },
            ),
            (
                "{\"hook_event_name\":\"SessionEnd\",\"session_id\":\"s\",\"cwd\":\"/w\"}",
                ClaudeEvent::SessionEnd { reason: None },
            ),
        ] {
            let line = serialize_envelope(&accepted(input));
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
            "user_prompt_submit",
            "pre_tool_use",
            "",
        ] {
            assert_rejected(
                &format!("{{\"protocol_version\":1,\"record\":{{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{{\"kind\":\"{kind}\"}},\"received_at\":1}}}}"),
                DecodeError::UnknownEvent,
            );
        }
    }

    #[test]
    fn unverified_metadata_labels_are_rejected() {
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
        // stop_failure's sensitive label (T02 never emits it) is never read:
        // the kind still decodes, and the label exists nowhere on the result.
        let line = "{\"protocol_version\":1,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"stop_failure\",\"error\":\"SENTINEL_ERROR\"},\"received_at\":1}}";
        let decoded = decode_envelope(line).expect("stop_failure decodes");
        assert_eq!(decoded.record.event, ClaudeEvent::StopFailure);
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
        // A T02-produced max-bounded record must still fit (see hook tests for
        // the serializer side); here just prove the bound is the encoder's own.
        let max = accepted(&format!(
            "{{\"hook_event_name\":\"SessionStart\",\"session_id\":\"{}\",\"cwd\":\"{}\"}}",
            "i".repeat(MAX_SESSION_ID_LEN),
            "/".repeat(MAX_CWD_LEN),
        ));
        let line = serialize_envelope(&max);
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
