//! Claude hook ingress — `docs/specs/dashboard/claude.md` R13-R16.
//!
//! The privacy-critical front door for Claude monitoring. A user-configured
//! Claude Code command hook (R11-R12) launches this helper, which receives
//! one hook event as JSON, parses it against a strict evidence-backed
//! allowlist, discards every non-allowlisted field, and writes one bounded
//! versioned newline-delimited envelope to a user-scoped Unix socket.
//! Delivery is best-effort: any listener problem (absent, stale, restarting,
//! full) is a logged drop, never a failure that could affect Claude (R16).
//!
//! The allowlist is the fifteen-event activity contract sealed 2026-09-05 and
//! widened in its advisor-reviewed round 2 against the published Claude Code
//! hooks reference (<https://code.claude.com/docs/en/hooks>) and a live
//! capture from this repository's own session (`tasks/2026-09-05-claude-
//! dashboard-activity-capture.spec-delta.md`): an event is accepted only if
//! it answers one of three questions (`claude.md` R13) — is it alive and
//! what is it doing (**SessionStart**, **UserPromptSubmit**, **PreToolUse**,
//! **PostToolUse**, **SubagentStart**), does it need me and has that cleared
//! (**PermissionRequest**, **PermissionDenied**, **Elicitation**,
//! **ElicitationResult**, **Notification**), or did it finish and how
//! (**Stop**, **StopFailure**, **SessionEnd**, **SubagentStop**,
//! **PostToolUseFailure**). Every other event (`TaskCreated`, `ConfigChange`,
//! `WorktreeCreate`, `PreCompact`, ...) is a silent no-op here (`claude.md`
//! R13).
//!
//! This module is self-contained on purpose: `tests/claude_ingress.rs`
//! compiles it directly via `#[path]` so the ingress boundary is
//! actually-compiled, actually-executed Cargo test coverage independent of
//! the rest of the crate. It must not depend on any other dashboard module
//! (`code-quality`'s encapsulation rule); `claude::state` maps these records
//! into the shared snapshot types.
//!
//! CONTRACT: ClaudeHookIngress (see docs/specs/dashboard/claude.md R13-R16)
//!
//! GUARANTEES:
//!   - Only the fifteen R13 events are accepted; any other hook event name is
//!     dropped with no output (R13).
//!   - An accepted record contains only `session_id`, `cwd`, the event name,
//!     local receipt time, and exactly the R14 fields documented for that
//!     event's own hook payload — nothing else ever enters it and nothing
//!     rejected ever appears in logs (R14).
//!   - A field marked "(bounded)" in R14 (`prompt`, `tool_input`,
//!     `tool_response`, `error`, `notification_message`, `agent_prompt`,
//!     `last_assistant_message`, `denial_reason`, `elicitation_request`,
//!     `user_response`) is truncated at a valid UTF-8 boundary at
//!     `MAX_FIELD_BYTES` with a trailing truncation marker when it is
//!     longer — truncated, never dropped whole (R14/R15). `tool_input` is
//!     always the tool's argument object serialized to compact JSON text
//!     before that same bound applies; it is never parsed back into
//!     structured data anywhere downstream.
//!   - A short opaque label (`tool_name`, `tool_use_id`, `agent_id`,
//!     `agent_type`, `notification_type`, `error_type`, `stop_reason`,
//!     `server_name`, `model`) longer than `MAX_LABEL_LEN`, or missing when
//!     its event requires it, drops the whole event (R14). `source`/`reason`
//!     remain closed enums validated against their documented value sets.
//!   - Every value is bounded; out-of-bound or malformed input is dropped
//!     before anything is serialized or sent (R15).
//!   - Delivery never blocks longer than the single R16 deadline, which
//!     covers socket-path resolution, filesystem metadata, connect, and
//!     write together, and never returns a failure a hook should
//!     propagate (R16).
//!
//! EXPECTS:
//!   - `parse_hook_input` receives the full hook payload as UTF-8 text.
//!   - `deliver` / `deliver_to` are only called with records produced by
//!     `parse_hook_input`.
//!
//! FAILURE BEHAVIOR:
//!   - Malformed JSON, unknown events, oversized input, or invalid
//!     IDs/paths/labels -> `Dropped(reason)`; nothing is sent and only the
//!     category is logged, never payload content.
//!   - Missing/unavailable/full listener -> a non-`Sent` `DeliveryOutcome`,
//!     all of which the helper treats as success.
//!
//! DOES NOT:
//!   - Read, write, or consult Claude configuration or transcripts.
//!   - Ever read or forward `transcript_path`/`agent_transcript_path` under
//!     any event, even though every hook payload carries them (R14/R17).
//!   - Forward any field not named in R14's per-event table, or parse
//!     `tool_input`/`tool_response` back into structured data.
//!   - Implement a listener, retry, persistent state, or exit-code policy
//!     (those belong to the runtime/adapter wiring).

use std::env;
use std::io::ErrorKind;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::time::timeout;

/// Wire protocol version carried by every envelope (`claude.md` R15).
pub const ENVELOPE_PROTOCOL_VERSION: u32 = 1;

/// Maximum size of a hook payload this parser will consider (bytes).
/// Larger inputs are dropped as oversized — before any parsing — so a
/// bloated or hostile payload can never make us work on it. Raised from
/// 64 KiB, then 256 KiB, now 2 MiB (advisor review round 2): a bounded
/// field's *raw*, untruncated size can legitimately be several hundred KiB
/// (e.g. a `Read` of a large file), and the parser must be able to read a
/// payload that big before it can truncate anything out of it — the old
/// 256 KiB cap could still whole-drop a payload R14's own truncation was
/// designed to handle (`claude.md` R15).
pub const MAX_HOOK_INPUT_BYTES: usize = 2 * 1024 * 1024;

/// Maximum size of a serialized envelope (UTF-8 bytes, including its newline).
/// Field bounds are checked before serialization, but JSON escaping can make
/// a field-bounded record exceed this separate wire bound. Raised from
/// 8 KiB: the largest event, `PostToolUse`, carries two bounded fields plus
/// labels (`claude.md` R15).
pub const MAX_ENVELOPE_BYTES: usize = 24 * 1024;

/// Maximum UTF-8 byte length of a session id (`claude.md` R15). Observed ids
/// are UUIDs (36 characters); the bound is deliberately looser so a future Claude id
/// format does not break ingress, but it is still hard.
pub const MAX_SESSION_ID_LEN: usize = 128;

/// Maximum UTF-8 byte length of a working-directory path (`claude.md` R15).
pub const MAX_CWD_LEN: usize = 4096;

/// Truncation cap for every R14 field marked "(bounded)" — `prompt`,
/// `tool_input` (after JSON serialization), `tool_response`, `error`,
/// `notification_message`, `agent_prompt`, `last_assistant_message`
/// (`claude.md` R14/R15). Content past this cap is cut, never dropped whole.
pub const MAX_FIELD_BYTES: usize = 4096;

/// Length bound for an opaque short label — `tool_name`, `tool_use_id`,
/// `agent_id`, `agent_type`, `notification_type`, `error_type`,
/// `stop_reason`, `model` (`claude.md` R14). These are documented with
/// "e.g." (open-ended) rather than a closed value set, unlike
/// `source`/`reason`, so they are validated by length alone. A label beyond
/// this bound, or missing where its event requires it, drops the whole
/// event — never truncated.
pub const MAX_LABEL_LEN: usize = 256;

/// Trailing marker appended to a "(bounded)" field cut at [`MAX_FIELD_BYTES`]
/// (`claude.md` R14).
pub const TRUNCATION_MARKER: &str = "…[truncated]";

/// Best-effort delivery: the single deadline for one entire hook delivery
/// attempt (`claude.md` R16). Socket-path resolution, filesystem metadata
/// (including `symlink_metadata`), connect, and write all share this one
/// budget, so blocking work can never exceed it. Chosen as the former
/// connect+write allowances combined, preserving the "a hook never waits
/// half a second for delivery" promise.
pub const DELIVERY_TIMEOUT: Duration = Duration::from_millis(500);

/// Environment variable that overrides the user-scoped socket path
/// (`claude.md` R15). The only configuration this module ever reads.
pub const SOCKET_ENV_VAR: &str = "DASHBOARD_CLAUDE_SOCKET";

/// Local time a hook payload was received, Unix epoch milliseconds. The
/// record's only timestamp — never a Claude transcript timestamp
/// (`claude.md` R14: only local receipt time crosses the boundary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReceivedAt(pub u64);

impl ReceivedAt {
    pub fn now() -> Self {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before the Unix epoch")
            .as_millis() as u64;
        ReceivedAt(millis)
    }

    pub fn epoch_millis(self) -> u64 {
        self.0
    }
}

/// Session-start source (`claude.md` R13/R14). Values from the T01c
/// evidence baseline (`redacted-schemas.md`): `startup` was directly
/// observed; the rest are documented values of the same closed enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStartSource {
    Startup,
    Resume,
    Clear,
    Compact,
    Fork,
}

impl SessionStartSource {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "startup" => Some(Self::Startup),
            "resume" => Some(Self::Resume),
            "clear" => Some(Self::Clear),
            "compact" => Some(Self::Compact),
            "fork" => Some(Self::Fork),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Resume => "resume",
            Self::Clear => "clear",
            Self::Compact => "compact",
            Self::Fork => "fork",
        }
    }
}

/// Session-end reason (`claude.md` R13/R14). Values from the T01c evidence
/// baseline (EVIDENCE.md S4): `other` was observed; `clear`, `resume`,
/// `logout`, `prompt_input_exit` are the documented siblings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndReason {
    Clear,
    Resume,
    Logout,
    PromptInputExit,
    Other,
}

impl SessionEndReason {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "clear" => Some(Self::Clear),
            "resume" => Some(Self::Resume),
            "logout" => Some(Self::Logout),
            "prompt_input_exit" => Some(Self::PromptInputExit),
            "other" => Some(Self::Other),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::Resume => "resume",
            Self::Logout => "logout",
            Self::PromptInputExit => "prompt_input_exit",
            Self::Other => "other",
        }
    }
}

/// An allowlisted Claude hook event (`claude.md` R13/R14). Every field on
/// every variant is exactly one named in R14's per-event table — nothing
/// else ever gets this far. Fields marked "(bounded)" in R14 are already
/// truncated by the time they reach here; short labels are already
/// length-validated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClaudeEvent {
    SessionStart {
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<SessionStartSource>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    UserPromptSubmit {
        prompt: String,
    },
    PreToolUse {
        tool_name: String,
        tool_use_id: String,
        tool_input: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
    },
    PostToolUse {
        tool_name: String,
        tool_use_id: String,
        tool_input: String,
        tool_response: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
    },
    PostToolUseFailure {
        tool_name: String,
        tool_use_id: String,
        tool_input: String,
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
    },
    PermissionRequest {
        tool_name: String,
        tool_use_id: String,
        tool_input: String,
    },
    PermissionDenied {
        tool_name: String,
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        denial_reason: Option<String>,
    },
    Elicitation {
        tool_use_id: String,
        server_name: String,
        elicitation_request: String,
    },
    ElicitationResult {
        tool_use_id: String,
        server_name: String,
        user_response: String,
    },
    Notification {
        #[serde(skip_serializing_if = "Option::is_none")]
        notification_type: Option<String>,
        notification_message: String,
    },
    Stop {
        last_assistant_message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
    },
    StopFailure {
        #[serde(skip_serializing_if = "Option::is_none")]
        error_type: Option<String>,
    },
    SubagentStart {
        agent_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        agent_prompt: String,
    },
    SubagentStop {
        agent_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        last_assistant_message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        stop_reason: Option<String>,
    },
    SessionEnd {
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<SessionEndReason>,
    },
}

impl ClaudeEvent {
    /// Stable wire label for this event kind (`claude.md` R15 envelope).
    #[allow(dead_code)]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::SessionStart { .. } => "session_start",
            Self::UserPromptSubmit { .. } => "user_prompt_submit",
            Self::PreToolUse { .. } => "pre_tool_use",
            Self::PostToolUse { .. } => "post_tool_use",
            Self::PostToolUseFailure { .. } => "post_tool_use_failure",
            Self::PermissionRequest { .. } => "permission_request",
            Self::PermissionDenied { .. } => "permission_denied",
            Self::Elicitation { .. } => "elicitation",
            Self::ElicitationResult { .. } => "elicitation_result",
            Self::Notification { .. } => "notification",
            Self::Stop { .. } => "stop",
            Self::StopFailure { .. } => "stop_failure",
            Self::SubagentStart { .. } => "subagent_start",
            Self::SubagentStop { .. } => "subagent_stop",
            Self::SessionEnd { .. } => "session_end",
        }
    }

    /// The subagent identity this event carries, when any (`claude.md`
    /// R14). `None` means the event targets the top-level session — the
    /// same rule `claude::state` uses to route between a top-level session
    /// and a tracked subagent session.
    pub fn agent_id(&self) -> Option<&str> {
        match self {
            Self::PreToolUse { agent_id, .. }
            | Self::PostToolUse { agent_id, .. }
            | Self::PostToolUseFailure { agent_id, .. }
            | Self::Stop { agent_id, .. } => agent_id.as_deref(),
            Self::SubagentStart { agent_id, .. } | Self::SubagentStop { agent_id, .. } => {
                Some(agent_id.as_str())
            }
            _ => None,
        }
    }

    /// The `tool_use_id` this event carries, when any (`claude.md` R14).
    /// Drives the generic exit-path rule in `claude::state`: any accepted
    /// event whose `tool_use_id` matches a tracked session's pending
    /// permission/elicitation clears that pending state and returns
    /// attention to `Running`, regardless of which specific event kind
    /// carries it — not a hardcoded "the next PreToolUse clears it"
    /// assumption, which is wrong (a tool's own `PreToolUse` for a given
    /// `tool_use_id` fires *before* the permission check, never after).
    pub fn tool_use_id(&self) -> Option<&str> {
        match self {
            Self::PreToolUse { tool_use_id, .. }
            | Self::PostToolUse { tool_use_id, .. }
            | Self::PostToolUseFailure { tool_use_id, .. }
            | Self::PermissionRequest { tool_use_id, .. }
            | Self::PermissionDenied { tool_use_id, .. }
            | Self::Elicitation { tool_use_id, .. }
            | Self::ElicitationResult { tool_use_id, .. } => Some(tool_use_id.as_str()),
            _ => None,
        }
    }
}

/// The internal allowlisted record (`claude.md` R13-R14). Built only from
/// `parse_hook_input`; contains no `serde_json::Value` and no rejected
/// field. This is what `claude::state` maps into snapshot types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeHookRecord {
    pub session_id: String,
    pub cwd: String,
    pub event: ClaudeEvent,
    pub received_at: ReceivedAt,
}

/// Versioned local IPC envelope (`claude.md` R15): one bounded JSON object,
/// newline-delimited on the wire, carrying a protocol version and one
/// record. No raw hook JSON is ever inside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeIpcEnvelope {
    pub protocol_version: u32,
    pub record: ClaudeHookRecord,
}

impl ClaudeIpcEnvelope {
    pub fn new(record: ClaudeHookRecord) -> Self {
        Self {
            protocol_version: ENVELOPE_PROTOCOL_VERSION,
            record,
        }
    }

    /// The wire form: one JSON object plus a trailing newline, when it fits
    /// the complete serialized-envelope bound.
    pub fn to_wire(&self) -> Result<String, EnvelopeSerializeError> {
        let mut out = serde_json::to_string(self)
            .expect("envelope serialization cannot fail: all fields are plain JSON values");
        out.push('\n');
        if out.len() > MAX_ENVELOPE_BYTES {
            return Err(EnvelopeSerializeError::Oversized);
        }
        Ok(out)
    }
}

/// Why a typed envelope could not be serialized for delivery. The error is
/// category-only so no rejected value can cross the hook boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeSerializeError {
    /// The complete escaped JSON frame, including its newline, exceeds R15.
    Oversized,
}

/// Serialize one record as its full envelope wire frame (JSON + newline).
/// Convenience for callers and tests that need the wire form directly.
pub fn serialize_envelope(record: &ClaudeHookRecord) -> Result<String, EnvelopeSerializeError> {
    ClaudeIpcEnvelope::new(record.clone()).to_wire()
}

/// Why a payload was dropped. Logging is category-only on purpose: the
/// rejected input's content must never appear in logs (`claude.md` R14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropReason {
    /// Not valid JSON, or missing a required allowlisted field.
    MalformedInput,
    /// Larger than `MAX_HOOK_INPUT_BYTES`, rejected before parsing.
    OversizedInput,
    /// A hook event name that is not in the R13 allowlist.
    UnknownEvent,
    /// Empty, whitespace-only, or longer than `MAX_SESSION_ID_LEN` UTF-8 bytes.
    InvalidSessionId,
    /// Empty, whitespace-only, or longer than `MAX_CWD_LEN` UTF-8 bytes.
    InvalidCwd,
    /// A closed-set metadata field (`source`, `reason`) carried a value
    /// outside its documented set, or the wrong JSON type.
    InvalidMetadata,
    /// An opaque short label (see `MAX_LABEL_LEN`'s doc comment) was longer
    /// than the bound, or the wrong JSON type.
    InvalidLabel,
    /// The complete escaped envelope exceeds `MAX_ENVELOPE_BYTES`.
    OversizedEnvelope,
}

impl DropReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MalformedInput => "malformed input",
            Self::OversizedInput => "oversized input",
            Self::UnknownEvent => "unknown event",
            Self::InvalidSessionId => "invalid session id",
            Self::InvalidCwd => "invalid cwd",
            Self::InvalidMetadata => "invalid metadata",
            Self::InvalidLabel => "invalid label",
            Self::OversizedEnvelope => "oversized envelope",
        }
    }
}

/// Result of parsing one hook payload: an accepted allowlisted record, or a
/// dropped category. Dropped payloads must never reach `deliver*`.
///
/// `ClaudeHookRecord` grew considerably with the R14 widening (bounded
/// content fields, multiple optional labels per event), so clippy flags the
/// size gap against `Dropped`. Not boxed: exactly one `ParseOutcome` exists
/// per hook invocation in this short-lived CLI process — there is no
/// collection of these and no hot loop — so the allocation clippy is
/// steering toward would cost more than the size gap it avoids.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum ParseOutcome {
    Accepted(ClaudeHookRecord),
    Dropped(DropReason),
}

/// Parse one Claude command-hook payload into an allowlisted record
/// (`claude.md` R13-R15).
///
/// Only allowlisted keys are ever read. The input is deserialized into a
/// transient `serde_json::Value` strictly for extraction and dropped before
/// the record is built — the original JSON is never retained, stored, or
/// forwarded (R14: "never retain the original `serde_json::Value`"). Values
/// in rejected fields are never extracted at all.
pub fn parse_hook_input(input: &str, received_at: ReceivedAt) -> ParseOutcome {
    if input.len() > MAX_HOOK_INPUT_BYTES {
        return dropped(DropReason::OversizedInput);
    }

    let raw: Value = match serde_json::from_str(input) {
        Ok(value) => value,
        Err(_) => return dropped(DropReason::MalformedInput),
    };

    // Classify by event name first: an unknown event is dropped without
    // ever parsing its other fields (which may hold sensitive content).
    let event_name = match raw.get("hook_event_name").and_then(Value::as_str) {
        Some(name) => name,
        None => return dropped(DropReason::MalformedInput),
    };

    let event = match event_name {
        "SessionStart" => parse_session_start(&raw),
        "UserPromptSubmit" => parse_user_prompt_submit(&raw),
        "PreToolUse" => parse_pre_tool_use(&raw),
        "PostToolUse" => parse_post_tool_use(&raw),
        "PostToolUseFailure" => parse_post_tool_use_failure(&raw),
        "PermissionRequest" => parse_permission_request(&raw),
        "PermissionDenied" => parse_permission_denied(&raw),
        "Elicitation" => parse_elicitation(&raw),
        "ElicitationResult" => parse_elicitation_result(&raw),
        "Notification" => parse_notification(&raw),
        "Stop" => parse_stop(&raw),
        "StopFailure" => parse_stop_failure(&raw),
        "SubagentStart" => parse_subagent_start(&raw),
        "SubagentStop" => parse_subagent_stop(&raw),
        "SessionEnd" => parse_session_end(&raw),
        _ => return dropped(DropReason::UnknownEvent),
    };
    let event = match event {
        Ok(event) => event,
        Err(reason) => return dropped(reason),
    };

    let session_id = match raw.get("session_id").and_then(Value::as_str) {
        Some(value) if valid_session_id(value) => value.to_owned(),
        Some(_) => return dropped(DropReason::InvalidSessionId),
        None => return dropped(DropReason::MalformedInput),
    };

    let cwd = match raw.get("cwd").and_then(Value::as_str) {
        Some(value) if valid_cwd(value) => value.to_owned(),
        Some(_) => return dropped(DropReason::InvalidCwd),
        None => return dropped(DropReason::MalformedInput),
    };

    let record = ClaudeHookRecord {
        session_id,
        cwd,
        event,
        received_at,
    };

    match serialize_envelope(&record) {
        Ok(_) => ParseOutcome::Accepted(record),
        Err(EnvelopeSerializeError::Oversized) => dropped(DropReason::OversizedEnvelope),
    }
}

/// Truncates `value` at a valid UTF-8 char boundary at or before
/// [`MAX_FIELD_BYTES`], appending [`TRUNCATION_MARKER`] when it was cut
/// (`claude.md` R14: a "(bounded)" field is truncated, never dropped whole).
fn bounded_text(value: &str) -> String {
    if value.len() <= MAX_FIELD_BYTES {
        return value.to_owned();
    }
    let mut cut = MAX_FIELD_BYTES;
    while cut > 0 && !value.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut out = String::with_capacity(cut + TRUNCATION_MARKER.len());
    out.push_str(&value[..cut]);
    out.push_str(TRUNCATION_MARKER);
    out
}

/// Reads a required "(bounded)" text field: present and a JSON string, with
/// [`bounded_text`] applied. Missing or the wrong JSON type is malformed —
/// the same treatment `session_id`/`cwd` already get.
fn read_required_text(raw: &Value, key: &str) -> Result<String, DropReason> {
    match raw.get(key).and_then(Value::as_str) {
        Some(value) => Ok(bounded_text(value)),
        None => Err(DropReason::MalformedInput),
    }
}

/// Reads an optional "(bounded)" text field: absent/null is `None`; present
/// must be a JSON string (bounded), otherwise malformed — `denial_reason`
/// is the only field of this shape (`claude.md` R14).
fn read_optional_text(raw: &Value, key: &str) -> Result<Option<String>, DropReason> {
    match raw.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(bounded_text(value))),
        Some(_) => Err(DropReason::MalformedInput),
    }
}

/// Reads the required `tool_input` field: the hook's own JSON object,
/// serialized to compact JSON text and then bounded (`claude.md` R14).
/// Never parsed back into structured data anywhere downstream.
fn read_tool_input(raw: &Value) -> Result<String, DropReason> {
    match raw.get("tool_input") {
        Some(value @ Value::Object(_)) => {
            let serialized = serde_json::to_string(value).expect("a JSON object always serializes");
            Ok(bounded_text(&serialized))
        }
        _ => Err(DropReason::MalformedInput),
    }
}

/// A short opaque label is valid purely by length — these fields are
/// documented with "e.g." (open-ended), not a closed value set
/// (`claude.md` R14).
fn valid_label(value: &str) -> bool {
    value.len() <= MAX_LABEL_LEN
}

/// Reads a required short label. Missing or the wrong JSON type is
/// malformed; present but over `MAX_LABEL_LEN` drops the event as an
/// invalid label — distinct categories, matching how `session_id` separates
/// "missing" from "too long."
fn read_required_label(raw: &Value, key: &str) -> Result<String, DropReason> {
    match raw.get(key) {
        Some(Value::String(value)) if valid_label(value) => Ok(value.clone()),
        Some(Value::String(_)) => Err(DropReason::InvalidLabel),
        _ => Err(DropReason::MalformedInput),
    }
}

/// Reads an optional short label: absent/null is `None`; present must be a
/// string within `MAX_LABEL_LEN` or the whole event is dropped.
fn read_optional_label(raw: &Value, key: &str) -> Result<Option<String>, DropReason> {
    match raw.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if valid_label(value) => Ok(Some(value.clone())),
        Some(_) => Err(DropReason::InvalidLabel),
    }
}

fn parse_session_start(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let source = match raw.get("source") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => match SessionStartSource::parse(value) {
            Some(source) => Some(source),
            None => return Err(DropReason::InvalidMetadata),
        },
        Some(_) => return Err(DropReason::InvalidMetadata),
    };
    let model = read_optional_label(raw, "model")?;
    Ok(ClaudeEvent::SessionStart { source, model })
}

fn parse_user_prompt_submit(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let prompt = read_required_text(raw, "prompt")?;
    Ok(ClaudeEvent::UserPromptSubmit { prompt })
}

fn parse_pre_tool_use(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let tool_name = read_required_label(raw, "tool_name")?;
    let tool_use_id = read_required_label(raw, "tool_use_id")?;
    let tool_input = read_tool_input(raw)?;
    let agent_id = read_optional_label(raw, "agent_id")?;
    let agent_type = read_optional_label(raw, "agent_type")?;
    Ok(ClaudeEvent::PreToolUse {
        tool_name,
        tool_use_id,
        tool_input,
        agent_id,
        agent_type,
    })
}

fn parse_post_tool_use(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let tool_name = read_required_label(raw, "tool_name")?;
    let tool_use_id = read_required_label(raw, "tool_use_id")?;
    let tool_input = read_tool_input(raw)?;
    let tool_response = read_required_text(raw, "tool_response")?;
    let agent_id = read_optional_label(raw, "agent_id")?;
    let agent_type = read_optional_label(raw, "agent_type")?;
    Ok(ClaudeEvent::PostToolUse {
        tool_name,
        tool_use_id,
        tool_input,
        tool_response,
        agent_id,
        agent_type,
    })
}

fn parse_post_tool_use_failure(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let tool_name = read_required_label(raw, "tool_name")?;
    let tool_use_id = read_required_label(raw, "tool_use_id")?;
    let tool_input = read_tool_input(raw)?;
    let error = read_required_text(raw, "error")?;
    let error_type = read_optional_label(raw, "error_type")?;
    let agent_id = read_optional_label(raw, "agent_id")?;
    let agent_type = read_optional_label(raw, "agent_type")?;
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

fn parse_permission_request(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let tool_name = read_required_label(raw, "tool_name")?;
    let tool_use_id = read_required_label(raw, "tool_use_id")?;
    let tool_input = read_tool_input(raw)?;
    Ok(ClaudeEvent::PermissionRequest {
        tool_name,
        tool_use_id,
        tool_input,
    })
}

fn parse_permission_denied(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let tool_name = read_required_label(raw, "tool_name")?;
    let tool_use_id = read_required_label(raw, "tool_use_id")?;
    let denial_reason = read_optional_text(raw, "denial_reason")?;
    Ok(ClaudeEvent::PermissionDenied {
        tool_name,
        tool_use_id,
        denial_reason,
    })
}

fn parse_elicitation(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let tool_use_id = read_required_label(raw, "tool_use_id")?;
    let server_name = read_required_label(raw, "server_name")?;
    let elicitation_request = read_required_text(raw, "elicitation_request")?;
    Ok(ClaudeEvent::Elicitation {
        tool_use_id,
        server_name,
        elicitation_request,
    })
}

fn parse_elicitation_result(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let tool_use_id = read_required_label(raw, "tool_use_id")?;
    let server_name = read_required_label(raw, "server_name")?;
    let user_response = read_required_text(raw, "user_response")?;
    Ok(ClaudeEvent::ElicitationResult {
        tool_use_id,
        server_name,
        user_response,
    })
}

fn parse_notification(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let notification_type = read_optional_label(raw, "notification_type")?;
    let notification_message = read_required_text(raw, "notification_message")?;
    Ok(ClaudeEvent::Notification {
        notification_type,
        notification_message,
    })
}

fn parse_stop(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let last_assistant_message = read_required_text(raw, "last_assistant_message")?;
    let agent_id = read_optional_label(raw, "agent_id")?;
    let agent_type = read_optional_label(raw, "agent_type")?;
    Ok(ClaudeEvent::Stop {
        last_assistant_message,
        agent_id,
        agent_type,
    })
}

fn parse_stop_failure(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let error_type = read_optional_label(raw, "error_type")?;
    Ok(ClaudeEvent::StopFailure { error_type })
}

fn parse_subagent_start(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let agent_id = read_required_label(raw, "agent_id")?;
    let agent_type = read_optional_label(raw, "agent_type")?;
    let agent_prompt = read_required_text(raw, "agent_prompt")?;
    Ok(ClaudeEvent::SubagentStart {
        agent_id,
        agent_type,
        agent_prompt,
    })
}

fn parse_subagent_stop(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let agent_id = read_required_label(raw, "agent_id")?;
    let agent_type = read_optional_label(raw, "agent_type")?;
    let last_assistant_message = read_required_text(raw, "last_assistant_message")?;
    let stop_reason = read_optional_label(raw, "stop_reason")?;
    Ok(ClaudeEvent::SubagentStop {
        agent_id,
        agent_type,
        last_assistant_message,
        stop_reason,
    })
}

fn parse_session_end(raw: &Value) -> Result<ClaudeEvent, DropReason> {
    let reason = match raw.get("reason") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => match SessionEndReason::parse(value) {
            Some(reason) => Some(reason),
            None => return Err(DropReason::InvalidMetadata),
        },
        Some(_) => return Err(DropReason::InvalidMetadata),
    };
    Ok(ClaudeEvent::SessionEnd { reason })
}

fn valid_session_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_SESSION_ID_LEN
}

fn valid_cwd(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_CWD_LEN
}

fn dropped(reason: DropReason) -> ParseOutcome {
    report_drop(reason);
    ParseOutcome::Dropped(reason)
}

/// How a delivery attempt ended. Every variant is a success for Claude
/// (`claude.md` R16): the helper must never fail, block, or delay Claude.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryOutcome {
    /// The envelope was written to the listener.
    Sent,
    /// No listener socket is reachable at all: the path is missing, or the
    /// path exists but is not a Unix socket.
    ListenerAbsent,
    /// A Unix socket exists at the path but the listener behind it did not
    /// accept the delivery within the bounded time or refused the
    /// connection: a busy/full accept queue, a restarting listener, a stale
    /// leftover socket file, or a permission problem. On some platforms a
    /// full queue blocks the connect until the deadline; on others it is
    /// refused instantly — both are the same consumer-visible case: the
    /// listener is not reachable right now.
    ListenerUnavailable,
    /// The complete escaped envelope exceeds the wire bound before any socket
    /// metadata, connection, or write is attempted.
    EnvelopeTooLarge,
}

impl DeliveryOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sent => "sent",
            Self::ListenerAbsent => "listener absent",
            Self::ListenerUnavailable => "listener unavailable",
            Self::EnvelopeTooLarge => "envelope too large",
        }
    }
}

/// Deliver one record to the default user-scoped socket (see
/// [`claude_socket_path`]). When no user-scoped location can be
/// established at all, reports the harmless unavailable drop outcome —
/// there is deliberately no shared, system-wide fallback
/// (`claude.md` R15/R16).
pub async fn deliver(record: &ClaudeHookRecord) -> DeliveryOutcome {
    // The one deadline starts before socket-path resolution, so the whole
    // helper attempt — resolution, metadata, connect, write — fits it.
    let deadline = tokio::time::Instant::now() + DELIVERY_TIMEOUT;
    match claude_socket_path() {
        Some(path) => deliver_before(record, &path, deadline).await,
        None => {
            let outcome = DeliveryOutcome::ListenerUnavailable;
            report_delivery(outcome);
            outcome
        }
    }
}

/// Deliver one record to an explicit socket path. Best-effort and bounded:
/// missing/stale/busy listeners are reported, never errors, and the whole
/// attempt fits the single [`DELIVERY_TIMEOUT`] budget — including
/// filesystem metadata work such as `symlink_metadata`, not only
/// connect/write (`claude.md` R16).
pub async fn deliver_to(record: &ClaudeHookRecord, socket_path: &Path) -> DeliveryOutcome {
    let deadline = tokio::time::Instant::now() + DELIVERY_TIMEOUT;
    deliver_before(record, socket_path, deadline).await
}

/// Shared delivery core: serialization, metadata, connect, and write all
/// run inside the single deadline. Any part overrunning the remaining
/// budget yields the harmless `ListenerUnavailable` outcome.
async fn deliver_before(
    record: &ClaudeHookRecord,
    socket_path: &Path,
    deadline: tokio::time::Instant,
) -> DeliveryOutcome {
    let attempt = async {
        let wire = match serialize_envelope(record) {
            Ok(wire) => wire,
            Err(EnvelopeSerializeError::Oversized) => return DeliveryOutcome::EnvelopeTooLarge,
        };
        match tokio::fs::symlink_metadata(socket_path).await {
            Ok(metadata) if metadata.file_type().is_socket() => {
                match UnixStream::connect(socket_path).await {
                    Ok(mut stream) => match stream.write_all(wire.as_bytes()).await {
                        Ok(()) => DeliveryOutcome::Sent,
                        Err(_) => DeliveryOutcome::ListenerUnavailable,
                    },
                    Err(error) => connect_error_outcome(&error),
                }
            }
            Ok(_) => DeliveryOutcome::ListenerAbsent,
            Err(error) if error.kind() == ErrorKind::NotFound => DeliveryOutcome::ListenerAbsent,
            Err(_) => DeliveryOutcome::ListenerUnavailable,
        }
    };
    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    let outcome = match timeout(remaining, attempt).await {
        Ok(outcome) => outcome,
        Err(_) => DeliveryOutcome::ListenerUnavailable,
    };
    report_delivery(outcome);
    outcome
}

fn connect_error_outcome(error: &std::io::Error) -> DeliveryOutcome {
    match error.kind() {
        // The path vanished between the metadata check and the connect —
        // there is truly no listener socket anymore (R16).
        ErrorKind::NotFound => DeliveryOutcome::ListenerAbsent,
        // A socket file exists, but the listener behind it is not accepting:
        // a busy/full queue, a restarting listener, or a stale leftover
        // socket file. Stale and busy cannot be told apart at connect time
        // (macOS, for instance, refuses a full queue instantly with
        // `ECONNREFUSED`), so both report the same harmless "unreachable
        // now" outcome (R16); only a missing/non-socket path is Absent.
        _ => DeliveryOutcome::ListenerUnavailable,
    }
}

/// The user-scoped socket path for this user (`claude.md` R15), resolved in
/// precedence order. Returns `None` when no user-scoped location can be
/// established — the caller then reports the harmless unavailable/drop
/// outcome. There is deliberately no shared, system-wide fallback and no
/// widening of permissions to create one:
/// 1. `DASHBOARD_CLAUDE_SOCKET` environment variable (explicit override).
/// 2. `$XDG_RUNTIME_DIR/dashboard-claude.sock` (per-user runtime dir).
/// 3. `$HOME/.local/state/dashboard/claude.sock` (per-user home state).
pub fn claude_socket_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os(SOCKET_ENV_VAR) {
        let path = PathBuf::from(path);
        if !path.as_os_str().is_empty() {
            return Some(path);
        }
    }
    if let Some(runtime_dir) = env::var_os("XDG_RUNTIME_DIR") {
        let dir = PathBuf::from(runtime_dir);
        if !dir.as_os_str().is_empty() {
            return Some(dir.join("dashboard-claude.sock"));
        }
    }
    if let Some(home) = env::var_os("HOME") {
        let path = PathBuf::from(home).join(".local/state/dashboard/claude.sock");
        if !path.as_os_str().is_empty() {
            return Some(path);
        }
    }
    None
}

/// Category-only drop log line (R14: rejected values never appear in logs).
fn report_drop(reason: DropReason) {
    log_line(&format!(
        "dashboard claude-hook: dropped ({})",
        reason.as_str()
    ));
}

/// Category-only delivery log line; non-`Sent` outcomes are expected daily
/// operation (dashboard not running), so they are informational, never
/// errors.
fn report_delivery(outcome: DeliveryOutcome) {
    if outcome != DeliveryOutcome::Sent {
        log_line(&format!(
            "dashboard claude-hook: delivery unavailable ({})",
            outcome.as_str()
        ));
    }
}

/// Single logging sink for the module. In production this is stderr and
/// lines carry only a category, never payload values (R14). The
/// `#[cfg(test)]` capture target lets the Cargo integration tests (which
/// compile this module via `#[path]`) assert that privacy property against
/// the actual log stream.
fn log_line(line: &str) {
    #[cfg(test)]
    {
        use std::io::Write;
        if let Ok(mut capture) = test_log::CAPTURE.lock() {
            if let Some(buffer) = capture.as_mut() {
                let _ = writeln!(buffer, "{line}");
                return;
            }
        }
    }
    eprintln!("{line}");
}

/// Test-only logging seam: while a test holds `Some(buffer)` in `CAPTURE`,
/// category-only lines are written there instead of stderr. Never used by
/// production code.
#[cfg(test)]
pub mod test_log {
    use std::sync::Mutex;

    /// `Some(buffer)` while a test is capturing; `None` means stderr.
    pub static CAPTURE: Mutex<Option<Vec<u8>>> = Mutex::new(None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Map;
    use std::sync::Mutex;

    /// Serializes env-dependent tests; no other test in this binary touches
    /// process environment.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    const RECEIVED: ReceivedAt = ReceivedAt(1_700_000_000_000);

    fn accepted(outcome: ParseOutcome) -> ClaudeHookRecord {
        match outcome {
            ParseOutcome::Accepted(record) => record,
            ParseOutcome::Dropped(reason) => panic!("expected accepted, got {reason:?}"),
        }
    }

    /// Builds a minimal valid payload for `event_name` plus `fields`, always
    /// including `session_id`/`cwd` and a `transcript_path` sentinel that
    /// must never survive parsing.
    fn payload(event_name: &str, fields: &[(&str, Value)]) -> String {
        let mut map = Map::new();
        map.insert("hook_event_name".into(), Value::from(event_name));
        map.insert(
            "session_id".into(),
            Value::from("00000000-0000-0000-0000-000000000000"),
        );
        map.insert("cwd".into(), Value::from("/work/project"));
        map.insert(
            "transcript_path".into(),
            Value::from("/work/project/.claude/transcript.jsonl"),
        );
        map.insert(
            "agent_transcript_path".into(),
            Value::from("/work/project/.claude/agent-transcript.jsonl"),
        );
        for (key, value) in fields {
            map.insert((*key).into(), value.clone());
        }
        serde_json::to_string(&Value::Object(map)).unwrap()
    }

    fn drop_reason(input: &str) -> DropReason {
        match parse_hook_input(input, RECEIVED) {
            ParseOutcome::Dropped(reason) => reason,
            ParseOutcome::Accepted(_) => panic!("expected a drop"),
        }
    }

    // -- Round trips: each of the 12 events keeps its own R14 fields ---------

    #[test]
    fn session_start_round_trips_source_and_model() {
        let record = accepted(parse_hook_input(
            &payload(
                "SessionStart",
                &[
                    ("source", Value::from("startup")),
                    ("model", Value::from("claude-sonnet")),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(record.session_id, "00000000-0000-0000-0000-000000000000");
        assert_eq!(record.cwd, "/work/project");
        assert_eq!(
            record.event,
            ClaudeEvent::SessionStart {
                source: Some(SessionStartSource::Startup),
                model: Some("claude-sonnet".to_string()),
            }
        );

        let wire = serialize_envelope(&record).unwrap();
        assert!(wire.contains("claude-sonnet"));
        assert!(!wire.contains("transcript"));
    }

    #[test]
    fn session_start_optional_fields_may_be_absent_or_null() {
        let absent = accepted(parse_hook_input(&payload("SessionStart", &[]), RECEIVED));
        assert_eq!(
            absent.event,
            ClaudeEvent::SessionStart {
                source: None,
                model: None
            }
        );

        let null = accepted(parse_hook_input(
            &payload(
                "SessionStart",
                &[("source", Value::Null), ("model", Value::Null)],
            ),
            RECEIVED,
        ));
        assert_eq!(
            null.event,
            ClaudeEvent::SessionStart {
                source: None,
                model: None
            }
        );
    }

    #[test]
    fn session_start_rejects_unverified_source_values() {
        assert_eq!(
            drop_reason(&payload(
                "SessionStart",
                &[("source", Value::from("forked-over-ssh"))]
            )),
            DropReason::InvalidMetadata
        );
    }

    #[test]
    fn user_prompt_submit_round_trips_prompt() {
        let record = accepted(parse_hook_input(
            &payload(
                "UserPromptSubmit",
                &[("prompt", Value::from("fix the bug"))],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::UserPromptSubmit {
                prompt: "fix the bug".to_string()
            }
        );
        let wire = serialize_envelope(&record).unwrap();
        assert!(wire.contains("fix the bug"));
    }

    #[test]
    fn user_prompt_submit_requires_prompt() {
        assert_eq!(
            drop_reason(&payload("UserPromptSubmit", &[])),
            DropReason::MalformedInput
        );
    }

    #[test]
    fn pre_tool_use_round_trips_all_fields() {
        let record = accepted(parse_hook_input(
            &payload(
                "PreToolUse",
                &[
                    ("tool_name", Value::from("Edit")),
                    ("tool_use_id", Value::from("call_1")),
                    ("tool_input", serde_json::json!({"file_path": "src/lib.rs"})),
                    ("agent_id", Value::from("agent-1")),
                    ("agent_type", Value::from("general-purpose")),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::PreToolUse {
                tool_name: "Edit".to_string(),
                tool_use_id: "call_1".to_string(),
                tool_input: "{\"file_path\":\"src/lib.rs\"}".to_string(),
                agent_id: Some("agent-1".to_string()),
                agent_type: Some("general-purpose".to_string()),
            }
        );
        assert_eq!(record.event.agent_id(), Some("agent-1"));
        let wire = serialize_envelope(&record).unwrap();
        assert!(wire.contains("src/lib.rs"));
        assert!(wire.contains("agent-1"));
    }

    #[test]
    fn pre_tool_use_requires_tool_name_and_use_id_and_input() {
        assert_eq!(
            drop_reason(&payload(
                "PreToolUse",
                &[
                    ("tool_use_id", Value::from("call_1")),
                    ("tool_input", serde_json::json!({}))
                ]
            )),
            DropReason::MalformedInput
        );
        assert_eq!(
            drop_reason(&payload(
                "PreToolUse",
                &[
                    ("tool_name", Value::from("Edit")),
                    ("tool_input", serde_json::json!({}))
                ]
            )),
            DropReason::MalformedInput
        );
        assert_eq!(
            drop_reason(&payload(
                "PreToolUse",
                &[
                    ("tool_name", Value::from("Edit")),
                    ("tool_use_id", Value::from("call_1")),
                ]
            )),
            DropReason::MalformedInput
        );
        // tool_input present but not a JSON object.
        assert_eq!(
            drop_reason(&payload(
                "PreToolUse",
                &[
                    ("tool_name", Value::from("Edit")),
                    ("tool_use_id", Value::from("call_1")),
                    ("tool_input", Value::from("not an object")),
                ]
            )),
            DropReason::MalformedInput
        );
    }

    #[test]
    fn pre_tool_use_has_no_agent_context_when_absent() {
        let record = accepted(parse_hook_input(
            &payload(
                "PreToolUse",
                &[
                    ("tool_name", Value::from("Bash")),
                    ("tool_use_id", Value::from("call_2")),
                    ("tool_input", serde_json::json!({"command": "ls"})),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(record.event.agent_id(), None);
    }

    #[test]
    fn post_tool_use_round_trips_response() {
        let record = accepted(parse_hook_input(
            &payload(
                "PostToolUse",
                &[
                    ("tool_name", Value::from("Bash")),
                    ("tool_use_id", Value::from("call_3")),
                    ("tool_input", serde_json::json!({"command": "cargo test"})),
                    ("tool_response", Value::from("all tests passed")),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::PostToolUse {
                tool_name: "Bash".to_string(),
                tool_use_id: "call_3".to_string(),
                tool_input: "{\"command\":\"cargo test\"}".to_string(),
                tool_response: "all tests passed".to_string(),
                agent_id: None,
                agent_type: None,
            }
        );
        let wire = serialize_envelope(&record).unwrap();
        assert!(wire.contains("all tests passed"));
    }

    #[test]
    fn post_tool_use_requires_tool_response() {
        assert_eq!(
            drop_reason(&payload(
                "PostToolUse",
                &[
                    ("tool_name", Value::from("Bash")),
                    ("tool_use_id", Value::from("call_3")),
                    ("tool_input", serde_json::json!({})),
                ]
            )),
            DropReason::MalformedInput
        );
    }

    #[test]
    fn post_tool_use_failure_round_trips_error_fields() {
        let record = accepted(parse_hook_input(
            &payload(
                "PostToolUseFailure",
                &[
                    ("tool_name", Value::from("Bash")),
                    ("tool_use_id", Value::from("call_4")),
                    ("tool_input", serde_json::json!({"command": "false"})),
                    ("error", Value::from("exit code 1")),
                    ("error_type", Value::from("nonzero_exit")),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::PostToolUseFailure {
                tool_name: "Bash".to_string(),
                tool_use_id: "call_4".to_string(),
                tool_input: "{\"command\":\"false\"}".to_string(),
                error: "exit code 1".to_string(),
                error_type: Some("nonzero_exit".to_string()),
                agent_id: None,
                agent_type: None,
            }
        );
    }

    #[test]
    fn permission_request_round_trips_tool_name_use_id_and_input() {
        let record = accepted(parse_hook_input(
            &payload(
                "PermissionRequest",
                &[
                    ("tool_name", Value::from("Bash")),
                    ("tool_use_id", Value::from("call_9")),
                    ("tool_input", serde_json::json!({"command": "rm -rf /"})),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::PermissionRequest {
                tool_name: "Bash".to_string(),
                tool_use_id: "call_9".to_string(),
                tool_input: "{\"command\":\"rm -rf /\"}".to_string(),
            }
        );
        assert_eq!(record.event.tool_use_id(), Some("call_9"));
    }

    #[test]
    fn permission_request_requires_tool_use_id() {
        assert_eq!(
            drop_reason(&payload(
                "PermissionRequest",
                &[
                    ("tool_name", Value::from("Bash")),
                    ("tool_input", serde_json::json!({})),
                ]
            )),
            DropReason::MalformedInput
        );
    }

    #[test]
    fn permission_denied_round_trips_with_and_without_reason() {
        let record = accepted(parse_hook_input(
            &payload(
                "PermissionDenied",
                &[
                    ("tool_name", Value::from("Bash")),
                    ("tool_use_id", Value::from("call_9")),
                    ("denial_reason", Value::from("policy forbids rm -rf")),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::PermissionDenied {
                tool_name: "Bash".to_string(),
                tool_use_id: "call_9".to_string(),
                denial_reason: Some("policy forbids rm -rf".to_string()),
            }
        );
        assert_eq!(record.event.tool_use_id(), Some("call_9"));

        let without_reason = accepted(parse_hook_input(
            &payload(
                "PermissionDenied",
                &[
                    ("tool_name", Value::from("Bash")),
                    ("tool_use_id", Value::from("call_9")),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(
            without_reason.event,
            ClaudeEvent::PermissionDenied {
                tool_name: "Bash".to_string(),
                tool_use_id: "call_9".to_string(),
                denial_reason: None,
            }
        );
    }

    #[test]
    fn permission_denied_requires_tool_name_and_use_id() {
        assert_eq!(
            drop_reason(&payload(
                "PermissionDenied",
                &[("tool_use_id", Value::from("call_9"))]
            )),
            DropReason::MalformedInput
        );
        assert_eq!(
            drop_reason(&payload(
                "PermissionDenied",
                &[("tool_name", Value::from("Bash"))]
            )),
            DropReason::MalformedInput
        );
    }

    #[test]
    fn elicitation_round_trips_all_fields() {
        let record = accepted(parse_hook_input(
            &payload(
                "Elicitation",
                &[
                    ("tool_use_id", Value::from("call_10")),
                    ("server_name", Value::from("my-mcp-server")),
                    ("elicitation_request", Value::from("confirm deletion?")),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::Elicitation {
                tool_use_id: "call_10".to_string(),
                server_name: "my-mcp-server".to_string(),
                elicitation_request: "confirm deletion?".to_string(),
            }
        );
        assert_eq!(record.event.tool_use_id(), Some("call_10"));
    }

    #[test]
    fn elicitation_requires_all_fields() {
        assert_eq!(
            drop_reason(&payload(
                "Elicitation",
                &[
                    ("server_name", Value::from("my-mcp-server")),
                    ("elicitation_request", Value::from("confirm?")),
                ]
            )),
            DropReason::MalformedInput
        );
        assert_eq!(
            drop_reason(&payload(
                "Elicitation",
                &[
                    ("tool_use_id", Value::from("call_10")),
                    ("elicitation_request", Value::from("confirm?")),
                ]
            )),
            DropReason::MalformedInput
        );
        assert_eq!(
            drop_reason(&payload(
                "Elicitation",
                &[
                    ("tool_use_id", Value::from("call_10")),
                    ("server_name", Value::from("my-mcp-server")),
                ]
            )),
            DropReason::MalformedInput
        );
    }

    #[test]
    fn elicitation_result_round_trips_all_fields() {
        let record = accepted(parse_hook_input(
            &payload(
                "ElicitationResult",
                &[
                    ("tool_use_id", Value::from("call_10")),
                    ("server_name", Value::from("my-mcp-server")),
                    ("user_response", Value::from("yes, delete it")),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::ElicitationResult {
                tool_use_id: "call_10".to_string(),
                server_name: "my-mcp-server".to_string(),
                user_response: "yes, delete it".to_string(),
            }
        );
        assert_eq!(record.event.tool_use_id(), Some("call_10"));
    }

    #[test]
    fn notification_round_trips_type_and_message() {
        let record = accepted(parse_hook_input(
            &payload(
                "Notification",
                &[
                    ("notification_type", Value::from("permission")),
                    ("notification_message", Value::from("waiting for approval")),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::Notification {
                notification_type: Some("permission".to_string()),
                notification_message: "waiting for approval".to_string(),
            }
        );
    }

    #[test]
    fn notification_requires_message_but_not_type() {
        let record = accepted(parse_hook_input(
            &payload(
                "Notification",
                &[("notification_message", Value::from("idle"))],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::Notification {
                notification_type: None,
                notification_message: "idle".to_string(),
            }
        );
        assert_eq!(
            drop_reason(&payload("Notification", &[])),
            DropReason::MalformedInput
        );
    }

    #[test]
    fn stop_round_trips_final_message_and_agent_context() {
        let record = accepted(parse_hook_input(
            &payload(
                "Stop",
                &[
                    ("last_assistant_message", Value::from("done")),
                    ("agent_id", Value::from("agent-9")),
                    ("agent_type", Value::from("explore")),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::Stop {
                last_assistant_message: "done".to_string(),
                agent_id: Some("agent-9".to_string()),
                agent_type: Some("explore".to_string()),
            }
        );
        assert_eq!(record.event.agent_id(), Some("agent-9"));
    }

    #[test]
    fn stop_failure_round_trips_error_type_and_omits_error_text() {
        let record = accepted(parse_hook_input(
            &payload(
                "StopFailure",
                &[
                    ("error_type", Value::from("timeout")),
                    ("error", Value::from("SENTINEL_ERROR")),
                    ("last_assistant_message", Value::from("SENTINEL_ASSISTANT")),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::StopFailure {
                error_type: Some("timeout".to_string())
            }
        );
        let wire = serialize_envelope(&record).unwrap();
        assert!(!wire.contains("SENTINEL"));
    }

    #[test]
    fn stop_failure_error_type_may_be_absent() {
        let record = accepted(parse_hook_input(&payload("StopFailure", &[]), RECEIVED));
        assert_eq!(record.event, ClaudeEvent::StopFailure { error_type: None });
    }

    #[test]
    fn subagent_start_round_trips_identity_and_prompt() {
        let record = accepted(parse_hook_input(
            &payload(
                "SubagentStart",
                &[
                    ("agent_id", Value::from("agent-42")),
                    ("agent_type", Value::from("general-purpose")),
                    ("agent_prompt", Value::from("investigate the flaky test")),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::SubagentStart {
                agent_id: "agent-42".to_string(),
                agent_type: Some("general-purpose".to_string()),
                agent_prompt: "investigate the flaky test".to_string(),
            }
        );
        assert_eq!(record.event.agent_id(), Some("agent-42"));
    }

    #[test]
    fn subagent_start_requires_agent_id_and_prompt() {
        assert_eq!(
            drop_reason(&payload(
                "SubagentStart",
                &[("agent_prompt", Value::from("go"))]
            )),
            DropReason::MalformedInput
        );
        assert_eq!(
            drop_reason(&payload("SubagentStart", &[("agent_id", Value::from("a"))])),
            DropReason::MalformedInput
        );
    }

    #[test]
    fn subagent_stop_round_trips_identity_and_stop_reason() {
        let record = accepted(parse_hook_input(
            &payload(
                "SubagentStop",
                &[
                    ("agent_id", Value::from("agent-42")),
                    ("agent_type", Value::from("general-purpose")),
                    ("last_assistant_message", Value::from("found the bug")),
                    ("stop_reason", Value::from("completed")),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::SubagentStop {
                agent_id: "agent-42".to_string(),
                agent_type: Some("general-purpose".to_string()),
                last_assistant_message: "found the bug".to_string(),
                stop_reason: Some("completed".to_string()),
            }
        );
    }

    #[test]
    fn session_end_reason_is_accepted() {
        for (raw, expected) in [
            ("clear", SessionEndReason::Clear),
            ("resume", SessionEndReason::Resume),
            ("logout", SessionEndReason::Logout),
            ("prompt_input_exit", SessionEndReason::PromptInputExit),
            ("other", SessionEndReason::Other),
        ] {
            let record = accepted(parse_hook_input(
                &payload("SessionEnd", &[("reason", Value::from(raw))]),
                RECEIVED,
            ));
            assert_eq!(
                record.event,
                ClaudeEvent::SessionEnd {
                    reason: Some(expected)
                },
                "reason {raw}"
            );
        }
    }

    #[test]
    fn session_end_reason_may_be_absent() {
        let record = accepted(parse_hook_input(&payload("SessionEnd", &[]), RECEIVED));
        assert_eq!(record.event, ClaudeEvent::SessionEnd { reason: None });
    }

    #[test]
    fn session_end_rejects_unverified_reason_values() {
        assert_eq!(
            drop_reason(&payload(
                "SessionEnd",
                &[("reason", Value::from("user_abandoned"))]
            )),
            DropReason::InvalidMetadata
        );
    }

    // -- Bounds: truncation and label length -----------------------------

    #[test]
    fn bounded_fields_are_truncated_with_a_marker_not_dropped() {
        let huge = "x".repeat(MAX_FIELD_BYTES + 500);
        let record = accepted(parse_hook_input(
            &payload("UserPromptSubmit", &[("prompt", Value::from(huge.clone()))]),
            RECEIVED,
        ));
        let ClaudeEvent::UserPromptSubmit { prompt } = record.event else {
            panic!("expected UserPromptSubmit");
        };
        assert!(prompt.len() < huge.len(), "prompt must be truncated");
        assert!(
            prompt.ends_with(TRUNCATION_MARKER),
            "truncated prompt must carry the marker: {prompt}"
        );
        assert!(prompt.len() <= MAX_FIELD_BYTES + TRUNCATION_MARKER.len());
    }

    #[test]
    fn bounded_field_at_or_under_the_cap_is_not_marked() {
        let exact = "y".repeat(MAX_FIELD_BYTES);
        let record = accepted(parse_hook_input(
            &payload(
                "UserPromptSubmit",
                &[("prompt", Value::from(exact.clone()))],
            ),
            RECEIVED,
        ));
        let ClaudeEvent::UserPromptSubmit { prompt } = record.event else {
            panic!("expected UserPromptSubmit");
        };
        assert_eq!(prompt, exact);
        assert!(!prompt.contains(TRUNCATION_MARKER));
    }

    #[test]
    fn truncation_cuts_at_a_valid_utf8_boundary() {
        // Each 'é' is 2 UTF-8 bytes; a naive byte-cut at MAX_FIELD_BYTES
        // would land mid-character since MAX_FIELD_BYTES is even but the
        // content here is offset by one leading ASCII byte.
        let huge = format!("a{}", "é".repeat(MAX_FIELD_BYTES));
        let record = accepted(parse_hook_input(
            &payload("UserPromptSubmit", &[("prompt", Value::from(huge))]),
            RECEIVED,
        ));
        let ClaudeEvent::UserPromptSubmit { prompt } = record.event else {
            panic!("expected UserPromptSubmit");
        };
        // Must still be valid UTF-8 (guaranteed by the type) and end with
        // the marker.
        assert!(prompt.ends_with(TRUNCATION_MARKER));
    }

    #[test]
    fn oversized_label_drops_the_whole_event() {
        assert_eq!(
            drop_reason(&payload(
                "PreToolUse",
                &[
                    ("tool_name", Value::from("i".repeat(MAX_LABEL_LEN + 1))),
                    ("tool_use_id", Value::from("call_1")),
                    ("tool_input", serde_json::json!({})),
                ]
            )),
            DropReason::InvalidLabel
        );
        // An optional label over the bound also drops the whole event.
        assert_eq!(
            drop_reason(&payload(
                "PreToolUse",
                &[
                    ("tool_name", Value::from("Edit")),
                    ("tool_use_id", Value::from("call_1")),
                    ("tool_input", serde_json::json!({})),
                    ("agent_id", Value::from("a".repeat(MAX_LABEL_LEN + 1))),
                ]
            )),
            DropReason::InvalidLabel
        );
    }

    #[test]
    fn label_at_exactly_the_bound_is_accepted() {
        let max_label = "i".repeat(MAX_LABEL_LEN);
        let record = accepted(parse_hook_input(
            &payload(
                "PreToolUse",
                &[
                    ("tool_name", Value::from(max_label.clone())),
                    ("tool_use_id", Value::from("call_1")),
                    ("tool_input", serde_json::json!({})),
                ],
            ),
            RECEIVED,
        ));
        assert_eq!(
            record.event,
            ClaudeEvent::PreToolUse {
                tool_name: max_label,
                tool_use_id: "call_1".to_string(),
                tool_input: "{}".to_string(),
                agent_id: None,
                agent_type: None,
            }
        );
    }

    #[test]
    fn wrong_type_label_is_invalid_label() {
        assert_eq!(
            drop_reason(&payload(
                "PreToolUse",
                &[
                    ("tool_name", serde_json::json!(5)),
                    ("tool_use_id", Value::from("call_1")),
                    ("tool_input", serde_json::json!({})),
                ]
            )),
            DropReason::MalformedInput
        );
        assert_eq!(
            drop_reason(&payload(
                "PreToolUse",
                &[
                    ("tool_name", Value::from("Edit")),
                    ("tool_use_id", Value::from("call_1")),
                    ("tool_input", serde_json::json!({})),
                    ("agent_id", serde_json::json!(5)),
                ]
            )),
            DropReason::InvalidLabel
        );
    }

    // -- Allowlist and identity/envelope bounds (unchanged from T02) ------

    #[test]
    fn malformed_json_is_dropped() {
        for input in [
            "",
            "not json",
            "{\"hook_event_name\":",
            "[1,2,3]",
            "null",
            "42",
        ] {
            assert_eq!(
                parse_hook_input(input, RECEIVED),
                ParseOutcome::Dropped(DropReason::MalformedInput),
                "payload {input:?}"
            );
        }
    }

    #[test]
    fn missing_event_name_is_dropped() {
        let input = "{\"session_id\":\"s\",\"cwd\":\"/w\"}";
        assert_eq!(
            parse_hook_input(input, RECEIVED),
            ParseOutcome::Dropped(DropReason::MalformedInput)
        );
    }

    #[test]
    fn events_outside_the_r13_allowlist_are_dropped_without_forwarding() {
        // Real hook events outside this dashboard's scope
        // (`claude.md` R13): reviewed and deliberately excluded, not
        // pending verification like the fifteen above.
        for name in [
            "TaskCreated",
            "ConfigChange",
            "WorktreeCreate",
            "PreCompact",
            "CwdChanged",
        ] {
            let mut map = Map::new();
            map.insert("hook_event_name".into(), Value::from(name));
            map.insert("session_id".into(), Value::from("s"));
            map.insert("cwd".into(), Value::from("/w"));
            map.insert("prompt".into(), Value::from("SENTINEL_PROMPT"));
            let input = serde_json::to_string(&Value::Object(map)).unwrap();
            match parse_hook_input(&input, RECEIVED) {
                ParseOutcome::Dropped(DropReason::UnknownEvent) => {}
                other => panic!("{name}: expected unknown-event drop, got {other:?}"),
            }
        }
    }

    #[test]
    fn oversized_input_is_dropped_before_parsing() {
        let input = format!("{{\"pad\":\"{}\"}}", "x".repeat(MAX_HOOK_INPUT_BYTES + 1));
        assert_eq!(
            parse_hook_input(&input, RECEIVED),
            ParseOutcome::Dropped(DropReason::OversizedInput)
        );
    }

    /// R15 (advisor review round 2): a legitimately large single tool
    /// result — e.g. a `Read` of a big file — must be accepted and
    /// truncated, not dropped whole, as long as the raw payload stays under
    /// the 2 MiB whole-payload cap. This supersedes the old test that
    /// proved the 256 KiB cap; that cap could still wrongly whole-drop a
    /// payload this size.
    #[test]
    fn a_large_but_under_cap_raw_payload_is_accepted_and_truncated_not_dropped() {
        // 1.5 MiB tool_response: well under MAX_HOOK_INPUT_BYTES (2 MiB),
        // well over MAX_FIELD_BYTES (4096).
        let huge_response = "x".repeat(1_500_000);
        let input = payload(
            "PostToolUse",
            &[
                ("tool_name", Value::from("Read")),
                ("tool_use_id", Value::from("call_big")),
                ("tool_input", serde_json::json!({"file_path": "big.log"})),
                ("tool_response", Value::from(huge_response.clone())),
            ],
        );
        assert!(
            input.len() > 1_500_000 && input.len() < MAX_HOOK_INPUT_BYTES,
            "fixture must exercise the 'large but under cap' band, got {} bytes",
            input.len()
        );
        let record = accepted(parse_hook_input(&input, RECEIVED));
        let ClaudeEvent::PostToolUse {
            ref tool_response, ..
        } = record.event
        else {
            panic!("expected PostToolUse");
        };
        assert!(
            tool_response.len() < huge_response.len(),
            "tool_response must be truncated"
        );
        assert!(tool_response.ends_with(TRUNCATION_MARKER));
        assert!(tool_response.len() <= MAX_FIELD_BYTES + TRUNCATION_MARKER.len());
        // The delivered envelope itself must still fit the (unchanged) wire
        // bound — only the raw whole-payload cap moved.
        let wire = serialize_envelope(&record).expect("truncated envelope must fit");
        assert!(wire.len() <= MAX_ENVELOPE_BYTES);
    }

    #[test]
    fn invalid_session_ids_are_dropped() {
        for session_id in ["", "   ", &"i".repeat(MAX_SESSION_ID_LEN + 1)] {
            let mut map = Map::new();
            map.insert("hook_event_name".into(), Value::from("SessionStart"));
            map.insert("session_id".into(), Value::from(session_id));
            map.insert("cwd".into(), Value::from("/w"));
            let input = serde_json::to_string(&Value::Object(map)).unwrap();
            assert_eq!(
                parse_hook_input(&input, RECEIVED),
                ParseOutcome::Dropped(DropReason::InvalidSessionId),
                "session_id {session_id:?}"
            );
        }
    }

    #[test]
    fn missing_session_id_is_malformed() {
        let input = "{\"hook_event_name\":\"SessionStart\",\"cwd\":\"/w\"}";
        assert_eq!(
            parse_hook_input(input, RECEIVED),
            ParseOutcome::Dropped(DropReason::MalformedInput)
        );
    }

    #[test]
    fn invalid_cwds_are_dropped() {
        for cwd in ["", "   ", &"p".repeat(MAX_CWD_LEN + 1)] {
            let mut map = Map::new();
            map.insert("hook_event_name".into(), Value::from("SessionStart"));
            map.insert("session_id".into(), Value::from("s"));
            map.insert("cwd".into(), Value::from(cwd));
            let input = serde_json::to_string(&Value::Object(map)).unwrap();
            assert_eq!(
                parse_hook_input(&input, RECEIVED),
                ParseOutcome::Dropped(DropReason::InvalidCwd),
                "cwd {cwd:?}"
            );
        }
    }

    #[test]
    fn identity_bounds_are_measured_in_utf8_bytes() {
        let bounded = serde_json::json!({
            "hook_event_name": "SessionStart",
            "session_id": "é".repeat(MAX_SESSION_ID_LEN / 2),
            "cwd": "é".repeat(MAX_CWD_LEN / 2),
        })
        .to_string();
        assert!(matches!(
            parse_hook_input(&bounded, RECEIVED),
            ParseOutcome::Accepted(_)
        ));

        let oversized_id = serde_json::json!({
            "hook_event_name": "SessionStart",
            "session_id": "é".repeat(MAX_SESSION_ID_LEN / 2 + 1),
            "cwd": "/w",
        })
        .to_string();
        assert_eq!(
            parse_hook_input(&oversized_id, RECEIVED),
            ParseOutcome::Dropped(DropReason::InvalidSessionId)
        );

        let oversized_cwd = serde_json::json!({
            "hook_event_name": "SessionStart",
            "session_id": "s",
            "cwd": "é".repeat(MAX_CWD_LEN / 2 + 1),
        })
        .to_string();
        assert_eq!(
            parse_hook_input(&oversized_cwd, RECEIVED),
            ParseOutcome::Dropped(DropReason::InvalidCwd)
        );
    }

    #[test]
    fn escaped_envelope_overflow_is_dropped_without_serialization_panic() {
        // A control character JSON-escapes to `\u00XX` (6 bytes) rather than
        // 1, so a single MAX_FIELD_BYTES-sized bounded field — which fits the
        // field's own truncation bound, measured in raw bytes before
        // escaping — is still enough to blow the serialized-envelope bound.
        let huge = "\u{1}".repeat(MAX_FIELD_BYTES);
        let input = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": "s",
            "cwd": "/w",
            "prompt": huge,
        })
        .to_string();
        assert_eq!(
            parse_hook_input(&input, RECEIVED),
            ParseOutcome::Dropped(DropReason::OversizedEnvelope)
        );

        let record = ClaudeHookRecord {
            session_id: "s".to_owned(),
            cwd: "/w".to_owned(),
            event: ClaudeEvent::UserPromptSubmit {
                prompt: "\u{1}".repeat(MAX_FIELD_BYTES),
            },
            received_at: RECEIVED,
        };
        assert_eq!(
            serialize_envelope(&record),
            Err(EnvelopeSerializeError::Oversized)
        );
    }

    #[test]
    fn wrong_field_types_are_malformed_or_invalid() {
        // session_id as a number
        let num_id = "{\"hook_event_name\":\"SessionStart\",\"session_id\":123,\"cwd\":\"/w\"}";
        assert_eq!(
            parse_hook_input(num_id, RECEIVED),
            ParseOutcome::Dropped(DropReason::MalformedInput)
        );
        // hook_event_name as a number
        let num_name = "{\"hook_event_name\":5,\"session_id\":\"s\",\"cwd\":\"/w\"}";
        assert_eq!(
            parse_hook_input(num_name, RECEIVED),
            ParseOutcome::Dropped(DropReason::MalformedInput)
        );
    }

    #[test]
    fn envelope_is_versioned_newline_delimited_and_bounded() {
        let record = accepted(parse_hook_input(
            &payload("SessionStart", &[("source", Value::from("startup"))]),
            RECEIVED,
        ));
        let wire = serialize_envelope(&record).expect("ordinary envelope must fit the bound");
        assert!(
            wire.ends_with('\n'),
            "envelope not newline-delimited: {wire:?}"
        );
        assert_eq!(wire.matches('\n').count(), 1);
        assert!(wire.len() <= MAX_ENVELOPE_BYTES);
        assert!(!wire.contains("transcript_path"));
        let value: Value = serde_json::from_str(wire.trim_end()).unwrap();
        assert_eq!(
            value["protocol_version"],
            Value::from(ENVELOPE_PROTOCOL_VERSION)
        );
        assert_eq!(
            value["record"]["session_id"],
            "00000000-0000-0000-0000-000000000000"
        );
        assert_eq!(value["record"]["cwd"], "/work/project");
        assert_eq!(value["record"]["event"]["kind"], "session_start");
        assert_eq!(value["record"]["event"]["source"], "startup");
        assert_eq!(value["record"]["received_at"], Value::from(RECEIVED.0));
        let mut keys: Vec<String> = value["record"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        keys.sort();
        assert_eq!(keys, ["cwd", "event", "received_at", "session_id"]);
    }

    #[test]
    fn envelope_wire_bytes_use_the_declared_order_and_compact_shape() {
        let record = ClaudeHookRecord {
            session_id: "s".to_owned(),
            cwd: "/w".to_owned(),
            event: ClaudeEvent::PreToolUse {
                tool_name: "Edit".to_owned(),
                tool_use_id: "call-1".to_owned(),
                tool_input: "{\"path\":\"a\"}".to_owned(),
                agent_id: None,
                agent_type: Some("general".to_owned()),
            },
            received_at: RECEIVED,
        };
        assert_eq!(
            serialize_envelope(&record).unwrap(),
            "{\"protocol_version\":1,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"pre_tool_use\",\"tool_name\":\"Edit\",\"tool_use_id\":\"call-1\",\"tool_input\":\"{\\\"path\\\":\\\"a\\\"}\",\"agent_type\":\"general\"},\"received_at\":1700000000000}}\n"
        );
    }

    #[test]
    fn envelope_omits_rejected_and_absent_fields_for_every_event_kind() {
        let failed = accepted(parse_hook_input(
            &payload("StopFailure", &[("error", Value::from("SENTINEL_ERROR"))]),
            RECEIVED,
        ));
        let failed_wire = serialize_envelope(&failed).expect("ordinary envelope must fit");
        assert!(!failed_wire.contains("SENTINEL_ERROR"));
        let failed_value: Value = serde_json::from_str(failed_wire.trim_end()).unwrap();
        assert_eq!(
            failed_value["record"]["event"],
            serde_json::json!({"kind": "stop_failure"})
        );

        let ended = accepted(parse_hook_input(
            &payload("SessionEnd", &[("reason", Value::from("other"))]),
            RECEIVED,
        ));
        let ended_wire = serialize_envelope(&ended).expect("ordinary envelope must fit");
        let ended_value: Value = serde_json::from_str(ended_wire.trim_end()).unwrap();
        assert_eq!(
            ended_value["record"]["event"],
            serde_json::json!({"kind": "session_end", "reason": "other"})
        );
    }

    #[test]
    fn transcript_paths_never_survive_any_event_kind() {
        for name in [
            "SessionStart",
            "UserPromptSubmit",
            "PreToolUse",
            "PostToolUse",
            "PostToolUseFailure",
            "PermissionRequest",
            "PermissionDenied",
            "Elicitation",
            "ElicitationResult",
            "Notification",
            "Stop",
            "StopFailure",
            "SubagentStart",
            "SubagentStop",
            "SessionEnd",
        ] {
            let fields: &[(&str, Value)] = match name {
                "PreToolUse" | "PermissionRequest" => &[
                    ("tool_name", Value::from("Edit")),
                    ("tool_use_id", Value::from("c")),
                    ("tool_input", serde_json::json!({})),
                ],
                "PostToolUse" => &[
                    ("tool_name", Value::from("Edit")),
                    ("tool_use_id", Value::from("c")),
                    ("tool_input", serde_json::json!({})),
                    ("tool_response", Value::from("ok")),
                ],
                "PostToolUseFailure" => &[
                    ("tool_name", Value::from("Edit")),
                    ("tool_use_id", Value::from("c")),
                    ("tool_input", serde_json::json!({})),
                    ("error", Value::from("boom")),
                ],
                "PermissionDenied" => &[
                    ("tool_name", Value::from("Edit")),
                    ("tool_use_id", Value::from("c")),
                    ("denial_reason", Value::from("no")),
                ],
                "Elicitation" => &[
                    ("tool_use_id", Value::from("c")),
                    ("server_name", Value::from("mcp")),
                    ("elicitation_request", Value::from("confirm?")),
                ],
                "ElicitationResult" => &[
                    ("tool_use_id", Value::from("c")),
                    ("server_name", Value::from("mcp")),
                    ("user_response", Value::from("yes")),
                ],
                "UserPromptSubmit" => &[("prompt", Value::from("hi"))],
                "Notification" => &[("notification_message", Value::from("hi"))],
                "Stop" => &[("last_assistant_message", Value::from("done"))],
                "SubagentStart" => &[
                    ("agent_id", Value::from("a")),
                    ("agent_prompt", Value::from("go")),
                ],
                "SubagentStop" => &[
                    ("agent_id", Value::from("a")),
                    ("last_assistant_message", Value::from("done")),
                ],
                _ => &[],
            };
            let record = accepted(parse_hook_input(&payload(name, fields), RECEIVED));
            let wire = serialize_envelope(&record).expect("ordinary envelope must fit");
            assert!(
                !wire.contains("transcript"),
                "{name}: transcript path leaked into {wire}"
            );
        }
    }

    #[test]
    fn received_at_now_is_epoch_millis() {
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before the Unix epoch")
            .as_millis() as u64;
        let now = ReceivedAt::now().epoch_millis();
        let after = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before the Unix epoch")
            .as_millis() as u64;
        assert!(
            before <= now && now <= after,
            "receipt time {now} outside [{before}, {after}]"
        );
    }

    #[tokio::test]
    // clippy's await-holding-lock heuristic flags holding `ENV_LOCK` across
    // the `deliver` await. This is safe by construction: `ENV_LOCK` only
    // serializes this binary's env-mutating tests (all sync and brief), the
    // awaited delivery never touches the lock, and holding it prevents a
    // race where another env test changes the socket path between set and
    // poll. Scope-allow with that reasoning recorded.
    #[allow(clippy::await_holding_lock)]
    async fn deliver_uses_the_configured_default_path_when_absent() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let prev = env::var_os(SOCKET_ENV_VAR);
        let path = std::env::temp_dir().join(format!(
            "dashboard-claude-deliver-absent-{}.sock",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        env::set_var(SOCKET_ENV_VAR, &path);

        let record = accepted(parse_hook_input(&payload("SessionStart", &[]), RECEIVED));
        let outcome = deliver(&record).await;
        assert_eq!(outcome, DeliveryOutcome::ListenerAbsent);

        match prev {
            Some(value) => env::set_var(SOCKET_ENV_VAR, value),
            None => env::remove_var(SOCKET_ENV_VAR),
        }
    }

    #[tokio::test]
    // Same env-lock reasoning as the test above: ENV_LOCK serializes this
    // binary's env-mutating tests and delivery never touches the lock.
    #[allow(clippy::await_holding_lock)]
    async fn deliver_without_any_user_scoped_location_is_unavailable() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let prev_socket = env::var_os(SOCKET_ENV_VAR);
        let prev_xdg = env::var_os("XDG_RUNTIME_DIR");
        let prev_home = env::var_os("HOME");
        env::remove_var(SOCKET_ENV_VAR);
        env::remove_var("XDG_RUNTIME_DIR");
        env::remove_var("HOME");

        let record = accepted(parse_hook_input(&payload("SessionStart", &[]), RECEIVED));
        let started = std::time::Instant::now();
        let outcome = deliver(&record).await;
        // No user-scoped location -> the harmless best-effort unavailable
        // drop, immediately, with no shared fallback attempted.
        assert_eq!(outcome, DeliveryOutcome::ListenerUnavailable);
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "no-location delivery must return immediately"
        );

        match prev_socket {
            Some(value) => env::set_var(SOCKET_ENV_VAR, value),
            None => env::remove_var(SOCKET_ENV_VAR),
        }
        match prev_xdg {
            Some(value) => env::set_var("XDG_RUNTIME_DIR", value),
            None => env::remove_var("XDG_RUNTIME_DIR"),
        }
        match prev_home {
            Some(value) => env::set_var("HOME", value),
            None => env::remove_var("HOME"),
        }
    }

    #[test]
    fn socket_path_is_user_scoped_with_precedence_and_no_shared_fallback() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let prev_socket = env::var_os(SOCKET_ENV_VAR);
        let prev_xdg = env::var_os("XDG_RUNTIME_DIR");
        let prev_home = env::var_os("HOME");

        // 1. Explicit override wins even when the others are set.
        env::set_var(SOCKET_ENV_VAR, "/run/user/1000/override.sock");
        env::set_var("XDG_RUNTIME_DIR", "/run/user/1001");
        env::set_var("HOME", "/home/alice");
        assert_eq!(
            claude_socket_path(),
            Some(PathBuf::from("/run/user/1000/override.sock"))
        );

        // 2. XDG_RUNTIME_DIR wins when no override.
        env::remove_var(SOCKET_ENV_VAR);
        assert_eq!(
            claude_socket_path(),
            Some(PathBuf::from("/run/user/1001/dashboard-claude.sock"))
        );

        // 3. HOME fallback when neither override nor runtime dir is set.
        env::remove_var("XDG_RUNTIME_DIR");
        assert_eq!(
            claude_socket_path(),
            Some(PathBuf::from(
                "/home/alice/.local/state/dashboard/claude.sock"
            ))
        );

        // 4. No user-scoped location at all -> None; there is never a
        //    shared (e.g. OS temp dir) fallback.
        env::remove_var("HOME");
        assert_eq!(claude_socket_path(), None);

        // Restore the process environment for the rest of the binary.
        match prev_socket {
            Some(value) => env::set_var(SOCKET_ENV_VAR, value),
            None => env::remove_var(SOCKET_ENV_VAR),
        }
        match prev_xdg {
            Some(value) => env::set_var("XDG_RUNTIME_DIR", value),
            None => env::remove_var("XDG_RUNTIME_DIR"),
        }
        match prev_home {
            Some(value) => env::set_var("HOME", value),
            None => env::remove_var("HOME"),
        }
    }
}
