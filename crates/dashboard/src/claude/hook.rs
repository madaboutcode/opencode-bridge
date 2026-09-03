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
//! The allowlist is the conservative one observed in T01c —
//! `tasks/spikes/2026-09-03-claude-dashboard-support/EVIDENCE.md` and
//! `redacted-schemas.md`: **SessionStart**, **StopFailure**, **SessionEnd**.
//! Every other event (UserPromptSubmit, PreToolUse, PostToolUse,
//! PermissionRequest, Notification, Stop, SubagentStart, SubagentStop,
//! CwdChanged, ...) is unverified until T05's authenticated gate and is
//! dropped as a no-op here.
//!
//! This module is self-contained on purpose: it is not wired into
//! `lib.rs`/`claude/mod.rs` until T03, so `tests/claude_ingress.rs`
//! compiles it directly via `#[path]` and executes it under Cargo now. It
//! must not depend on any other dashboard module (`code-quality`'s
//! encapsulation rule); the T03 adapter maps these records into the shared
//! snapshot types.
//!
//! CONTRACT: ClaudeHookIngress (see docs/specs/dashboard/claude.md R13-R16)
//!
//! GUARANTEES:
//!   - Only SessionStart, StopFailure, SessionEnd are accepted; any other
//!     hook event name is dropped with no output (R13).
//!   - An accepted record contains only session_id, cwd, limited event
//!     metadata, and local receipt time; sensitive/unknown fields never
//!     enter it and never appear in logs (R14).
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
//!   - Forward tool input/output, prompt/assistant text, transcript paths,
//!     error details, secrets, or unknown fields.
//!   - Implement a listener, retry, persistent state, or exit-code policy
//!     (those belong to T03/T04 wiring).

use std::env;
use std::io::ErrorKind;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::time::timeout;

/// Wire protocol version carried by every envelope (`claude.md` R15).
pub const ENVELOPE_PROTOCOL_VERSION: u32 = 1;

/// Maximum size of a hook payload this parser will consider (bytes).
/// Larger inputs are dropped as oversized — before any parsing — so a
/// bloated or hostile payload can never make us work on it.
pub const MAX_HOOK_INPUT_BYTES: usize = 64 * 1024;

/// Maximum size of a serialized envelope (bytes). Unreachable given the
/// value bounds below (128 + 4096 + short labels + JSON overhead), asserted
/// as an internal invariant at serialization time.
pub const MAX_ENVELOPE_BYTES: usize = 8 * 1024;

/// Maximum length of a session id (`claude.md` R15). Observed ids are UUIDs
/// (36 chars); the bound is deliberately looser so a future Claude id
/// format does not break ingress, but it is still hard.
pub const MAX_SESSION_ID_LEN: usize = 128;

/// Maximum length of a working-directory path (`claude.md` R15).
pub const MAX_CWD_LEN: usize = 4096;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// An allowlisted Claude hook event. Unit variants carry exactly the
/// observed, non-sensitive metadata: nothing else ever gets this far.
///
/// StopFailure deliberately carries no error classification: the only
/// observed value field on it (`error`) is sensitive
/// (`redacted-schemas.md`), and no bounded non-sensitive label was
/// observed — see `docs/specs/dashboard/claude.md` R13's `[REVIEW]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaudeEvent {
    SessionStart { source: Option<SessionStartSource> },
    StopFailure,
    SessionEnd { reason: Option<SessionEndReason> },
}

impl ClaudeEvent {
    /// Stable wire label for this event kind (`claude.md` R15 envelope).
    pub fn kind(&self) -> &'static str {
        match self {
            Self::SessionStart { .. } => "session_start",
            Self::StopFailure => "stop_failure",
            Self::SessionEnd { .. } => "session_end",
        }
    }
}

/// The internal allowlisted record (`claude.md` R13-R14). Built only from
/// `parse_hook_input`; contains no `serde_json::Value` and no rejected
/// field. This is what the T03 adapter will map into snapshot types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeHookRecord {
    pub session_id: String,
    pub cwd: String,
    pub event: ClaudeEvent,
    pub received_at: ReceivedAt,
}

/// Versioned local IPC envelope (`claude.md` R15): one bounded JSON object,
/// newline-delimited on the wire, carrying a protocol version and one
/// record. No raw hook JSON is ever inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// The wire form: one JSON object plus a trailing newline.
    pub fn to_wire(&self) -> String {
        let mut out = serde_json::to_string(&envelope_to_value(self))
            .expect("envelope serialization cannot fail: all fields are plain JSON values");
        out.push('\n');
        assert!(
            out.len() <= MAX_ENVELOPE_BYTES,
            "envelope exceeded its size bound: {} > {}",
            out.len(),
            MAX_ENVELOPE_BYTES
        );
        out
    }
}

/// Serialize one record as its full envelope wire frame (JSON + newline).
/// Convenience for callers and tests that need the wire form directly.
pub fn serialize_envelope(record: &ClaudeHookRecord) -> String {
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
    /// A hook event name that is not in the observed allowlist.
    UnknownEvent,
    /// Empty, whitespace-only, or longer than `MAX_SESSION_ID_LEN`.
    InvalidSessionId,
    /// Empty, whitespace-only, or longer than `MAX_CWD_LEN`.
    InvalidCwd,
    /// An allowlisted metadata field carried an unverified value/type.
    InvalidMetadata,
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
        }
    }
}

/// Result of parsing one hook payload: an accepted allowlisted record, or a
/// dropped category. Dropped payloads must never reach `deliver*`.
#[derive(Debug, Clone, PartialEq, Eq)]
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
        "SessionStart" => match parse_session_start(&raw) {
            Ok(event) => event,
            Err(reason) => return dropped(reason),
        },
        "StopFailure" => ClaudeEvent::StopFailure,
        "SessionEnd" => match parse_session_end(&raw) {
            Ok(event) => event,
            Err(reason) => return dropped(reason),
        },
        _ => return dropped(DropReason::UnknownEvent),
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

    ParseOutcome::Accepted(ClaudeHookRecord {
        session_id,
        cwd,
        event,
        received_at,
    })
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
    Ok(ClaudeEvent::SessionStart { source })
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
}

impl DeliveryOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sent => "sent",
            Self::ListenerAbsent => "listener absent",
            Self::ListenerUnavailable => "listener unavailable",
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
        let wire = serialize_envelope(record);
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

fn envelope_to_value(envelope: &ClaudeIpcEnvelope) -> Value {
    let record = &envelope.record;

    let mut event = Map::new();
    event.insert("kind".to_string(), Value::from(record.event.kind()));
    match &record.event {
        ClaudeEvent::SessionStart { source } => {
            if let Some(source) = source {
                event.insert("source".to_string(), Value::from(source.as_str()));
            }
        }
        ClaudeEvent::StopFailure => {}
        ClaudeEvent::SessionEnd { reason } => {
            if let Some(reason) = reason {
                event.insert("reason".to_string(), Value::from(reason.as_str()));
            }
        }
    }

    let mut record_map = Map::new();
    record_map.insert(
        "session_id".to_string(),
        Value::from(record.session_id.as_str()),
    );
    record_map.insert("cwd".to_string(), Value::from(record.cwd.as_str()));
    record_map.insert("event".to_string(), Value::Object(event));
    record_map.insert("received_at".to_string(), Value::from(record.received_at.0));

    let mut root = Map::new();
    root.insert(
        "protocol_version".to_string(),
        Value::from(envelope.protocol_version),
    );
    root.insert("record".to_string(), Value::Object(record_map));
    Value::Object(root)
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

    fn session_start(source: Option<&str>) -> String {
        let mut map = Map::new();
        map.insert("hook_event_name".into(), Value::from("SessionStart"));
        map.insert(
            "session_id".into(),
            Value::from("00000000-0000-0000-0000-000000000000"),
        );
        map.insert("cwd".into(), Value::from("/work/project"));
        if let Some(source) = source {
            map.insert("source".into(), Value::from(source));
        }
        // Sensitive / unknown fields the parser must never keep:
        map.insert(
            "transcript_path".into(),
            Value::from("/work/project/.claude/transcript.jsonl"),
        );
        map.insert("unknown_field".into(), Value::from("SENTINEL_UNKNOWN"));
        serde_json::to_string(&Value::Object(map)).unwrap()
    }

    fn session_end(reason: Option<&str>) -> String {
        let mut map = Map::new();
        map.insert("hook_event_name".into(), Value::from("SessionEnd"));
        map.insert(
            "session_id".into(),
            Value::from("00000000-0000-0000-0000-000000000000"),
        );
        map.insert("cwd".into(), Value::from("/work/project"));
        if let Some(reason) = reason {
            map.insert("reason".into(), Value::from(reason));
        }
        serde_json::to_string(&Value::Object(map)).unwrap()
    }

    #[test]
    fn session_start_with_observed_source_is_accepted() {
        let record = accepted(parse_hook_input(&session_start(Some("startup")), RECEIVED));
        assert_eq!(record.session_id, "00000000-0000-0000-0000-000000000000");
        assert_eq!(record.cwd, "/work/project");
        assert_eq!(record.received_at, RECEIVED);
        assert_eq!(
            record.event,
            ClaudeEvent::SessionStart {
                source: Some(SessionStartSource::Startup)
            }
        );
    }

    #[test]
    fn session_start_optional_source_may_be_absent_or_null() {
        let absent = accepted(parse_hook_input(&session_start(None), RECEIVED));
        assert_eq!(absent.event, ClaudeEvent::SessionStart { source: None });

        let mut with_null = Map::new();
        with_null.insert("hook_event_name".into(), Value::from("SessionStart"));
        with_null.insert("session_id".into(), Value::from("s"));
        with_null.insert("cwd".into(), Value::from("/w"));
        with_null.insert("source".into(), Value::Null);
        let null = accepted(parse_hook_input(
            &serde_json::to_string(&Value::Object(with_null)).unwrap(),
            RECEIVED,
        ));
        assert_eq!(null.event, ClaudeEvent::SessionStart { source: None });
    }

    #[test]
    fn session_start_rejects_unverified_source_values() {
        let payload = session_start(Some("forked-over-ssh"));
        match parse_hook_input(&payload, RECEIVED) {
            ParseOutcome::Dropped(DropReason::InvalidMetadata) => {}
            other => panic!("expected invalid metadata, got {other:?}"),
        }
    }

    #[test]
    fn session_start_rejects_non_string_source() {
        let mut map = Map::new();
        map.insert("hook_event_name".into(), Value::from("SessionStart"));
        map.insert("session_id".into(), Value::from("s"));
        map.insert("cwd".into(), Value::from("/w"));
        map.insert("source".into(), Value::Object(Map::new()));
        let payload = serde_json::to_string(&Value::Object(map)).unwrap();
        assert_eq!(
            parse_hook_input(&payload, RECEIVED),
            ParseOutcome::Dropped(DropReason::InvalidMetadata)
        );
    }

    #[test]
    fn stop_failure_is_accepted_without_metadata() {
        let mut map = Map::new();
        map.insert("hook_event_name".into(), Value::from("StopFailure"));
        map.insert("session_id".into(), Value::from("sess"));
        map.insert("cwd".into(), Value::from("/w"));
        map.insert("error".into(), Value::from("SENTINEL_ERROR"));
        map.insert(
            "last_assistant_message".into(),
            Value::from("SENTINEL_ASSISTANT"),
        );
        let payload = serde_json::to_string(&Value::Object(map)).unwrap();
        let record = accepted(parse_hook_input(&payload, RECEIVED));
        assert_eq!(record.event, ClaudeEvent::StopFailure);
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
            let record = accepted(parse_hook_input(&session_end(Some(raw)), RECEIVED));
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
        let record = accepted(parse_hook_input(&session_end(None), RECEIVED));
        assert_eq!(record.event, ClaudeEvent::SessionEnd { reason: None });
    }

    #[test]
    fn session_end_rejects_unverified_reason_values() {
        assert_eq!(
            parse_hook_input(&session_end(Some("user_abandoned")), RECEIVED),
            ParseOutcome::Dropped(DropReason::InvalidMetadata)
        );
    }

    #[test]
    fn malformed_json_is_dropped() {
        for payload in [
            "",
            "not json",
            "{\"hook_event_name\":",
            "[1,2,3]",
            "null",
            "42",
        ] {
            assert_eq!(
                parse_hook_input(payload, RECEIVED),
                ParseOutcome::Dropped(DropReason::MalformedInput),
                "payload {payload:?}"
            );
        }
    }

    #[test]
    fn missing_event_name_is_dropped() {
        let payload = "{\"session_id\":\"s\",\"cwd\":\"/w\"}";
        assert_eq!(
            parse_hook_input(payload, RECEIVED),
            ParseOutcome::Dropped(DropReason::MalformedInput)
        );
    }

    #[test]
    fn unknown_events_are_dropped_without_forwarding() {
        // Every event T01c could NOT verify is a no-op here (`claude.md` R13).
        for name in [
            "UserPromptSubmit",
            "PreToolUse",
            "PostToolUse",
            "PostToolUseFailure",
            "PermissionRequest",
            "Notification",
            "SubagentStart",
            "SubagentStop",
            "Stop",
            "CwdChanged",
        ] {
            let mut map = Map::new();
            map.insert("hook_event_name".into(), Value::from(name));
            map.insert("session_id".into(), Value::from("s"));
            map.insert("cwd".into(), Value::from("/w"));
            map.insert("prompt".into(), Value::from("SENTINEL_PROMPT"));
            let payload = serde_json::to_string(&Value::Object(map)).unwrap();
            match parse_hook_input(&payload, RECEIVED) {
                ParseOutcome::Dropped(DropReason::UnknownEvent) => {}
                other => panic!("{name}: expected unknown-event drop, got {other:?}"),
            }
        }
    }

    #[test]
    fn oversized_input_is_dropped_before_parsing() {
        let payload = format!("{{\"pad\":\"{}\"}}", "x".repeat(MAX_HOOK_INPUT_BYTES + 1));
        assert_eq!(
            parse_hook_input(&payload, RECEIVED),
            ParseOutcome::Dropped(DropReason::OversizedInput)
        );
    }

    #[test]
    fn invalid_session_ids_are_dropped() {
        for session_id in ["", "   ", &"i".repeat(MAX_SESSION_ID_LEN + 1)] {
            let mut map = Map::new();
            map.insert("hook_event_name".into(), Value::from("SessionStart"));
            map.insert("session_id".into(), Value::from(session_id));
            map.insert("cwd".into(), Value::from("/w"));
            let payload = serde_json::to_string(&Value::Object(map)).unwrap();
            assert_eq!(
                parse_hook_input(&payload, RECEIVED),
                ParseOutcome::Dropped(DropReason::InvalidSessionId),
                "session_id {session_id:?}"
            );
        }
    }

    #[test]
    fn missing_session_id_is_malformed() {
        let payload = "{\"hook_event_name\":\"SessionStart\",\"cwd\":\"/w\"}";
        assert_eq!(
            parse_hook_input(payload, RECEIVED),
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
            let payload = serde_json::to_string(&Value::Object(map)).unwrap();
            assert_eq!(
                parse_hook_input(&payload, RECEIVED),
                ParseOutcome::Dropped(DropReason::InvalidCwd),
                "cwd {cwd:?}"
            );
        }
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
        let record = accepted(parse_hook_input(&session_start(Some("startup")), RECEIVED));
        let wire = serialize_envelope(&record);
        assert!(
            wire.ends_with('\n'),
            "envelope not newline-delimited: {wire:?}"
        );
        assert_eq!(wire.matches('\n').count(), 1);
        assert!(wire.len() <= MAX_ENVELOPE_BYTES);
        assert!(!wire.contains("transcript_path"));
        assert!(!wire.contains("SENTINEL_UNKNOWN"));
        assert!(!wire.contains("SENTINEL"));
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
    fn envelope_omits_rejected_fields_for_every_event_kind() {
        let failed = accepted(parse_hook_input(
            "{\"hook_event_name\":\"StopFailure\",\"session_id\":\"s\",\"cwd\":\"/w\",\"error\":\"SENTINEL_ERROR\"}",
            RECEIVED,
        ));
        let failed_wire = serialize_envelope(&failed);
        assert!(!failed_wire.contains("SENTINEL_ERROR"));
        let failed_value: Value = serde_json::from_str(failed_wire.trim_end()).unwrap();
        assert_eq!(
            failed_value["record"]["event"],
            serde_json::json!({"kind": "stop_failure"})
        );

        let ended = accepted(parse_hook_input(&session_end(Some("other")), RECEIVED));
        let ended_wire = serialize_envelope(&ended);
        let ended_value: Value = serde_json::from_str(ended_wire.trim_end()).unwrap();
        assert_eq!(
            ended_value["record"]["event"],
            serde_json::json!({"kind": "session_end", "reason": "other"})
        );
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

        let record = accepted(parse_hook_input(&session_start(None), RECEIVED));
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

        let record = accepted(parse_hook_input(&session_start(None), RECEIVED));
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
