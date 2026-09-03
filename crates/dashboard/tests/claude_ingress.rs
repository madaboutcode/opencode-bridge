//! Cargo-executed real-Unix-socket ingress tests (T02, `claude.md` R13-R16).
//!
//! The Claude hook module is not wired into `lib.rs`/`claude/mod.rs` until
//! T03, so this file compiles `../src/claude/hook.rs` directly via `#[path]`.
//! That makes the owned ingress boundary an actually-compiled,
//! actually-executed Cargo test target now: parse, serialize, and
//! best-effort delivery are exercised against a real local Unix socket
//! before any library wiring exists.
//!
//! Privacy scope: no test reads or writes Claude configuration or
//! transcripts. Every socket lives under the OS temp dir (unique per test,
//! removed on drop), and negative tests assert sentinel values never cross
//! the IPC boundary.

#![cfg(unix)]

#[path = "../src/claude/hook.rs"]
mod claude_hook;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Map, Value};
use tokio::io::AsyncReadExt;
use tokio::net::{UnixListener, UnixStream};
use tokio::time::timeout;

use claude_hook::{
    parse_hook_input, ClaudeEvent, ClaudeHookRecord, DeliveryOutcome, DropReason, ParseOutcome,
    ReceivedAt,
};

const RECEIVED: ReceivedAt = ReceivedAt(1_700_000_000_000);

/// Parses a payload that is expected to be accepted; panics otherwise.
fn parsed(input: &str) -> ClaudeHookRecord {
    match parse_hook_input(input, RECEIVED) {
        ParseOutcome::Accepted(record) => record,
        ParseOutcome::Dropped(reason) => panic!("expected accepted hook input, got {reason:?}"),
    }
}

/// Asserts a payload is dropped with exactly `expected`.
fn assert_dropped(input: &str, expected: DropReason) {
    match parse_hook_input(input, RECEIVED) {
        ParseOutcome::Dropped(reason) => assert_eq!(reason, expected),
        ParseOutcome::Accepted(_) => panic!("expected drop {expected:?}, got accepted"),
    }
}

/// Builds a SessionStart payload with the given identity and optional
/// extra fields (test-side only — the ingress module never builds payloads).
fn session_start_payload(
    session_id: &str,
    cwd: &str,
    source: Option<&str>,
    extras: &[(&str, &str)],
) -> String {
    let mut map = Map::new();
    map.insert("hook_event_name".into(), Value::from("SessionStart"));
    map.insert("session_id".into(), Value::from(session_id));
    map.insert("cwd".into(), Value::from(cwd));
    if let Some(source) = source {
        map.insert("source".into(), Value::from(source));
    }
    for (key, value) in extras {
        map.insert((*key).into(), Value::from(*value));
    }
    serde_json::to_string(&Value::Object(map)).unwrap()
}

fn stop_failure_payload(session_id: &str, cwd: &str) -> String {
    let mut map = Map::new();
    map.insert("hook_event_name".into(), Value::from("StopFailure"));
    map.insert("session_id".into(), Value::from(session_id));
    map.insert("cwd".into(), Value::from(cwd));
    map.insert("error".into(), Value::from("SENTINEL_ERROR"));
    map.insert(
        "last_assistant_message".into(),
        Value::from("SENTINEL_ASSISTANT"),
    );
    serde_json::to_string(&Value::Object(map)).unwrap()
}

fn session_end_payload(session_id: &str, cwd: &str, reason: &str) -> String {
    let mut map = Map::new();
    map.insert("hook_event_name".into(), Value::from("SessionEnd"));
    map.insert("session_id".into(), Value::from(session_id));
    map.insert("cwd".into(), Value::from(cwd));
    map.insert("reason".into(), Value::from(reason));
    map.insert(
        "transcript_path".into(),
        Value::from("/secret/transcript.jsonl"),
    );
    serde_json::to_string(&Value::Object(map)).unwrap()
}

/// A unique-per-test socket path in the OS temp dir, removed on drop.
struct TempSocket(PathBuf);

impl TempSocket {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "dashboard-claude-test-{}-{name}.sock",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempSocket {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn bind_listener(socket: &TempSocket) -> UnixListener {
    UnixListener::bind(socket.path()).expect("bind test listener")
}

/// Accepts at most `expected` connections over `wait`, reading one frame
/// (up to and including the newline) per connection. Returns whatever
/// arrived before the deadline — callers assert the exact count. Takes the
/// listener by value so it can run inside a `tokio::spawn` task; on Unix
/// the socket FILE survives the listener being dropped, which is exactly
/// what the stale/restarting-socket tests rely on.
async fn collect_frames(listener: UnixListener, expected: usize, wait: Duration) -> Vec<String> {
    let deadline = tokio::time::Instant::now() + wait;
    let mut frames = Vec::new();
    while frames.len() < expected {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        let accepted = timeout(remaining, listener.accept()).await;
        let (mut stream, _) = match accepted {
            Ok(Ok(accepted)) => accepted,
            _ => break,
        };
        let mut body = String::new();
        let _ = timeout(remaining, stream.read_to_string(&mut body)).await;
        for line in body.lines() {
            if !line.trim().is_empty() {
                frames.push(line.to_owned());
            }
        }
    }
    frames
}

/// Runs `body` while the module's category-only log lines (`report_drop`,
/// `report_delivery`) are redirected into a buffer, then returns everything
/// captured. Only the test below captures, so the single global target
/// cannot collide; the lock is released around the await so parallel tests
/// logging cannot deadlock it.
async fn capture_module_logs<F: std::future::Future<Output = ()>>(body: F) -> String {
    {
        let mut guard = claude_hook::test_log::CAPTURE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = Some(Vec::new());
    }
    body.await;
    let mut guard = claude_hook::test_log::CAPTURE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match guard.take() {
        Some(bytes) => String::from_utf8(bytes).expect("captured logs are UTF-8"),
        None => String::new(),
    }
}

#[tokio::test]
async fn accepted_observed_events_are_delivered_over_a_real_socket() {
    let socket = TempSocket::new("accepted");
    let listener = bind_listener(&socket);

    let records = vec![
        parsed(&session_start_payload(
            "sess-a",
            "/work/a",
            Some("startup"),
            &[],
        )),
        parsed(&stop_failure_payload("sess-b", "/work/b")),
        parsed(&session_end_payload("sess-c", "/work/c", "other")),
    ];

    let mut senders = Vec::new();
    for record in records {
        let path = socket.path().to_path_buf();
        senders.push(tokio::spawn(async move {
            claude_hook::deliver_to(&record, &path).await
        }));
    }
    for handle in senders {
        assert_eq!(
            handle.await.expect("sender task panicked"),
            DeliveryOutcome::Sent
        );
    }

    let frames = collect_frames(listener, 3, Duration::from_secs(5)).await;
    assert_eq!(frames.len(), 3, "expected 3 envelopes, got {frames:?}");

    let mut by_session: HashMap<String, String> = HashMap::new();
    for frame in &frames {
        // One newline-delimited connection delivered exactly one envelope;
        // a multi-line frame would have been split apart by `collect_frames`
        // and changed the count. The trailing newline itself is asserted at
        // the wire layer in the module's unit tests.
        assert!(
            !frame.contains('\n'),
            "frame must be a single line: {frame:?}"
        );
        let value: Value = serde_json::from_str(frame).expect("frame is not a JSON envelope");
        assert_eq!(value["protocol_version"], Value::from(1u32));
        assert!(frame.len() <= claude_hook::MAX_ENVELOPE_BYTES);
        let session = value["record"]["session_id"].as_str().unwrap().to_string();
        by_session.insert(session, frame.clone());
    }
    assert_eq!(by_session.len(), 3);

    let start_frame = by_session["sess-a"].as_str();
    let start: Value = serde_json::from_str(start_frame).unwrap();
    assert_eq!(start["record"]["cwd"], "/work/a");
    assert_eq!(start["record"]["event"]["kind"], "session_start");
    assert_eq!(start["record"]["event"]["source"], "startup");
    assert_eq!(start["record"]["received_at"], Value::from(RECEIVED.0));
    let mut start_keys: Vec<String> = start["record"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    start_keys.sort();
    assert_eq!(start_keys, ["cwd", "event", "received_at", "session_id"]);

    let failure_frame = by_session["sess-b"].as_str();
    let failure: Value = serde_json::from_str(failure_frame).unwrap();
    assert_eq!(failure["record"]["event"]["kind"], "stop_failure");
    assert_eq!(
        failure["record"]["event"],
        serde_json::json!({"kind": "stop_failure"})
    );

    let end_frame = by_session["sess-c"].as_str();
    let end: Value = serde_json::from_str(end_frame).unwrap();
    assert_eq!(end["record"]["event"]["kind"], "session_end");
    assert_eq!(end["record"]["event"]["reason"], "other");
    // transcript_path was present in the payload and must be absent here —
    // both from the parsed record and from the raw wire frame.
    assert!(end["record"].get("transcript_path").is_none());
    assert!(!end_frame.contains("transcript") && !end_frame.contains("secret"));
}

#[tokio::test]
async fn absent_listener_succeeds_harmlessly_and_boundedly() {
    let socket = TempSocket::new("absent");
    let record = parsed(&session_start_payload(
        "sess-absent",
        "/tmp/absent",
        Some("startup"),
        &[],
    ));

    let started = tokio::time::Instant::now();
    let outcome = claude_hook::deliver_to(&record, socket.path()).await;
    assert_eq!(outcome, DeliveryOutcome::ListenerAbsent);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "delivery to an absent listener must not hang (took {:?})",
        started.elapsed()
    );
}

#[tokio::test]
async fn non_socket_file_at_the_path_is_treated_as_absent() {
    let socket = TempSocket::new("regular");
    std::fs::write(socket.path(), b"not a socket").expect("write temp file");
    let record = parsed(&session_start_payload(
        "sess-regular",
        "/tmp/regular",
        None,
        &[],
    ));

    let outcome = claude_hook::deliver_to(&record, socket.path()).await;
    assert_eq!(outcome, DeliveryOutcome::ListenerAbsent);
}

#[tokio::test]
async fn stale_socket_file_without_listener_succeeds_harmlessly() {
    let socket = TempSocket::new("stale");
    let listener = bind_listener(&socket);
    drop(listener); // the socket FILE remains on disk, but nothing accepts
    assert!(
        socket.path().exists(),
        "stale socket file should still exist"
    );

    let record = parsed(&session_start_payload(
        "sess-stale",
        "/tmp/stale",
        None,
        &[],
    ));
    let started = tokio::time::Instant::now();
    let outcome = claude_hook::deliver_to(&record, socket.path()).await;
    // A socket file exists but no listener accepts: connect is refused,
    // which on this platform is indistinguishable from a full/busy queue,
    // so the outcome is the harmless `ListenerUnavailable` (R16) — still a
    // non-error drop, still fast.
    assert_eq!(outcome, DeliveryOutcome::ListenerUnavailable);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "stale-socket delivery must not hang (took {:?})",
        started.elapsed()
    );
}

#[tokio::test]
async fn restarting_listener_degrades_harmlessly_and_recovers() {
    let socket = TempSocket::new("restart");

    // Listener generation 1 is accepting and receives a frame.
    let first = bind_listener(&socket);
    let first_collector = tokio::spawn(collect_frames(first, 1, Duration::from_secs(5)));
    let first_record = parsed(&session_start_payload("sess-r1", "/tmp/r1", None, &[]));
    let outcome = claude_hook::deliver_to(&first_record, socket.path()).await;
    assert_eq!(outcome, DeliveryOutcome::Sent);
    let first_frames = first_collector.await.unwrap();
    assert_eq!(first_frames.len(), 1);
    // The first listener is now dropped (its task ended); on Unix the
    // socket FILE lingers on disk but nothing accepts — the stale state
    // the gap below deliberately exercises.

    // During the gap, delivery degrades to a harmless non-error outcome.
    let gap_record = parsed(&session_start_payload("sess-gap", "/tmp/gap", None, &[]));
    let outcome = claude_hook::deliver_to(&gap_record, socket.path()).await;
    assert_ne!(outcome, DeliveryOutcome::Sent);

    // Restart on the same path: a fresh listener unlinks the stale socket
    // file (the dashboard's own listener will do the same on startup),
    // binds, and delivery recovers.
    let _ = std::fs::remove_file(socket.path());
    let second = bind_listener(&socket);
    let second_collector = tokio::spawn(collect_frames(second, 1, Duration::from_secs(5)));
    let second_record = parsed(&session_start_payload("sess-r2", "/tmp/r2", None, &[]));
    let outcome = claude_hook::deliver_to(&second_record, socket.path()).await;
    assert_eq!(outcome, DeliveryOutcome::Sent);
    let second_frames = second_collector.await.unwrap();
    assert_eq!(second_frames.len(), 1);
    assert!(second_frames[0].contains("sess-r2"));
}

#[tokio::test]
async fn concurrent_short_lived_sends_all_arrive() {
    let socket = TempSocket::new("concurrent");
    let listener = bind_listener(&socket);

    const N: usize = 16;
    let mut senders = Vec::new();
    for i in 0..N {
        let record = parsed(&session_start_payload(
            &format!("sess-{i}"),
            "/tmp/proj",
            Some("startup"),
            &[],
        ));
        let path = socket.path().to_path_buf();
        senders.push(tokio::spawn(async move {
            claude_hook::deliver_to(&record, &path).await
        }));
    }
    for handle in senders {
        assert_eq!(
            handle.await.expect("sender task panicked"),
            DeliveryOutcome::Sent
        );
    }

    let frames = collect_frames(listener, N, Duration::from_secs(10)).await;
    assert_eq!(
        frames.len(),
        N,
        "expected {N} envelopes, got {}",
        frames.len()
    );

    let mut sessions = HashSet::new();
    for frame in &frames {
        let value: Value = serde_json::from_str(frame).expect("frame is not a JSON envelope");
        assert_eq!(value["protocol_version"], Value::from(1u32));
        sessions.insert(value["record"]["session_id"].as_str().unwrap().to_string());
    }
    assert_eq!(
        sessions.len(),
        N,
        "session ids must be distinct per envelope"
    );
}

#[tokio::test]
async fn malformed_unknown_and_oversized_input_never_reach_the_socket() {
    let socket = TempSocket::new("never-send");
    let listener = bind_listener(&socket);

    assert_dropped("{ definitely not json", DropReason::MalformedInput);
    assert_dropped(
        "{\"hook_event_name\":\"UserPromptSubmit\",\"session_id\":\"s\",\"cwd\":\"/w\",\"prompt\":\"SENTINEL_PROMPT\",\"tool_use_id\":\"SENTINEL_TOOL\"}",
        DropReason::UnknownEvent,
    );
    let oversized = format!(
        "{{\"hook_event_name\":\"SessionStart\",\"session_id\":\"s\",\"cwd\":\"/w\",\"pad\":\"{}\"}}",
        "x".repeat(claude_hook::MAX_HOOK_INPUT_BYTES + 1)
    );
    assert_dropped(&oversized, DropReason::OversizedInput);

    // Dropped input is a no-op: no deliver call is made, so nothing arrives.
    let frames = collect_frames(listener, 1, Duration::from_millis(300)).await;
    assert!(
        frames.is_empty(),
        "dropped input must never reach the socket: {frames:?}"
    );
}

#[tokio::test]
async fn invalid_and_oversized_ids_and_paths_are_dropped_before_ipc() {
    assert_dropped(
        &session_start_payload("", "/w", None, &[]),
        DropReason::InvalidSessionId,
    );
    assert_dropped(
        &session_start_payload("   ", "/w", None, &[]),
        DropReason::InvalidSessionId,
    );
    assert_dropped(
        &session_start_payload(
            &"i".repeat(claude_hook::MAX_SESSION_ID_LEN + 1),
            "/w",
            None,
            &[],
        ),
        DropReason::InvalidSessionId,
    );
    assert_dropped(
        &session_start_payload("s", "", None, &[]),
        DropReason::InvalidCwd,
    );
    assert_dropped(
        &session_start_payload("s", &"p".repeat(claude_hook::MAX_CWD_LEN + 1), None, &[]),
        DropReason::InvalidCwd,
    );
}

#[tokio::test]
async fn maximum_bounded_record_fits_the_envelope_and_is_delivered() {
    let socket = TempSocket::new("bounded");
    let listener = bind_listener(&socket);

    let max_id = "i".repeat(claude_hook::MAX_SESSION_ID_LEN);
    let max_cwd = "/".repeat(claude_hook::MAX_CWD_LEN);
    let record = parsed(&session_start_payload(
        &max_id,
        &max_cwd,
        Some("startup"),
        &[],
    ));

    let wire = claude_hook::serialize_envelope(&record).expect("maximum ASCII envelope must fit");
    assert!(
        wire.len() <= claude_hook::MAX_ENVELOPE_BYTES,
        "wire {} bytes exceeds bound {}",
        wire.len(),
        claude_hook::MAX_ENVELOPE_BYTES
    );
    assert!(wire.ends_with('\n'));

    let collector = tokio::spawn(collect_frames(listener, 1, Duration::from_secs(5)));
    let outcome = claude_hook::deliver_to(&record, socket.path()).await;
    assert_eq!(outcome, DeliveryOutcome::Sent);

    let frames = collector.await.unwrap();
    assert_eq!(frames.len(), 1);
    let value: Value = serde_json::from_str(&frames[0]).expect("frame is not a JSON envelope");
    assert_eq!(value["record"]["session_id"].as_str().unwrap(), max_id);
    assert_eq!(value["record"]["cwd"].as_str().unwrap(), max_cwd);
}

#[tokio::test]
async fn escaped_envelope_overflow_drops_before_any_ipc() {
    let socket = TempSocket::new("escaped-overflow");
    let listener = bind_listener(&socket);
    let record = ClaudeHookRecord {
        session_id: "s".to_owned(),
        cwd: "\"".repeat(claude_hook::MAX_CWD_LEN),
        event: ClaudeEvent::SessionStart { source: None },
        received_at: RECEIVED,
    };

    let outcome = claude_hook::deliver_to(&record, socket.path()).await;
    assert_eq!(outcome, DeliveryOutcome::EnvelopeTooLarge);

    let frames = collect_frames(listener, 1, Duration::from_millis(300)).await;
    assert!(
        frames.is_empty(),
        "oversized escaped envelope must not reach the socket: {frames:?}"
    );
}

#[tokio::test]
async fn sensitive_fields_never_cross_the_ipc_boundary() {
    let socket = TempSocket::new("privacy");
    let listener = bind_listener(&socket);

    // SessionStart carrying every class of rejected field, each with a
    // unique sentinel.
    let start_payload = session_start_payload(
        "sess-priv",
        "/work/priv",
        Some("startup"),
        &[
            ("transcript_path", "SENTINEL_TRANSCRIPT"),
            ("agent_transcript_path", "SENTINEL_AGENT_TRANSCRIPT"),
            ("last_assistant_message", "SENTINEL_ASSISTANT"),
            ("prompt", "SENTINEL_PROMPT"),
            ("session_title", "SENTINEL_TITLE"),
            ("claude_secret_field", "SENTINEL_SECRET"),
        ],
    );
    let start_record = parsed(&start_payload);
    let start_wire = claude_hook::serialize_envelope(&start_record).expect("start envelope fits");
    for sentinel in [
        "SENTINEL_TRANSCRIPT",
        "SENTINEL_AGENT_TRANSCRIPT",
        "SENTINEL_ASSISTANT",
        "SENTINEL_PROMPT",
        "SENTINEL_TITLE",
        "SENTINEL_SECRET",
    ] {
        assert!(
            !start_wire.contains(sentinel),
            "sentinel {sentinel} leaked into the envelope: {start_wire}"
        );
    }
    let start_value: Value = serde_json::from_str(start_wire.trim_end()).unwrap();
    let mut record_keys: Vec<String> = start_value["record"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    record_keys.sort();
    assert_eq!(record_keys, ["cwd", "event", "received_at", "session_id"]);
    let mut event_keys: Vec<String> = start_value["record"]["event"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    event_keys.sort();
    assert_eq!(event_keys, ["kind", "source"]);

    // StopFailure carrying error details and assistant text.
    let fail_record = parsed(&stop_failure_payload("sess-fail", "/work/fail"));
    let fail_wire = claude_hook::serialize_envelope(&fail_record).expect("failure envelope fits");
    for sentinel in ["SENTINEL_ERROR", "SENTINEL_ASSISTANT"] {
        assert!(
            !fail_wire.contains(sentinel),
            "sentinel {sentinel} leaked into the envelope: {fail_wire}"
        );
    }

    // SessionEnd carrying a transcript path.
    let end_record = parsed(&session_end_payload("sess-end", "/work/end", "other"));
    let end_wire = claude_hook::serialize_envelope(&end_record).expect("end envelope fits");
    assert!(
        !end_wire.contains("SENTINEL") && !end_wire.contains("transcript"),
        "transcript data leaked into the envelope: {end_wire}"
    );

    // Deliver all three and confirm the wire frames are equally clean.
    let records = vec![start_record, fail_record, end_record];
    let mut senders = Vec::new();
    for record in records {
        let path = socket.path().to_path_buf();
        senders.push(tokio::spawn(async move {
            claude_hook::deliver_to(&record, &path).await
        }));
    }
    for handle in senders {
        assert_eq!(
            handle.await.expect("sender task panicked"),
            DeliveryOutcome::Sent
        );
    }
    let frames = collect_frames(listener, 3, Duration::from_secs(5)).await;
    assert_eq!(frames.len(), 3);
    for frame in &frames {
        assert!(
            !frame.contains("SENTINEL") && !frame.contains("transcript"),
            "sentinel crossed the IPC boundary: {frame}"
        );
    }
}

#[tokio::test]
async fn unknown_event_with_prompt_content_is_dropped_before_any_delivery() {
    let socket = TempSocket::new("unknown-prompt");
    let listener = bind_listener(&socket);

    let payload = "{\"hook_event_name\":\"PreToolUse\",\"session_id\":\"s\",\"cwd\":\"/w\",\"tool_name\":\"Edit\",\"tool_input\":{\"file_path\":\"SENTINEL_FILE\",\"prompt\":\"SENTINEL_TOOL_PROMPT\"},\"tool_response\":\"SENTINEL_RESPONSE\"}";
    assert_dropped(payload, DropReason::UnknownEvent);

    let frames = collect_frames(listener, 1, Duration::from_millis(300)).await;
    assert!(
        frames.is_empty(),
        "unknown events with tool content must never reach the socket: {frames:?}"
    );
}

#[tokio::test]
async fn busy_full_listener_yields_listener_unavailable_within_the_deadline() {
    let socket = TempSocket::new("busy");
    let listener = bind_listener(&socket);

    // Saturate the listener's accept backlog with pending connections that
    // are never accepted, so a fresh connect cannot complete. The kernel
    // backlog differs per OS (tokio asks for 1024; macOS clamps to
    // somaxconn), so dial well past either bound and keep every completed
    // connection alive for the duration of the test.
    const DIALS: usize = 2048;
    let mut dialers = Vec::new();
    for _ in 0..DIALS {
        let path = socket.path().to_path_buf();
        dialers.push(tokio::spawn(async move {
            match tokio::time::timeout(Duration::from_millis(200), UnixStream::connect(&path)).await
            {
                Ok(Ok(stream)) => Some(stream),
                _ => None,
            }
        }));
    }
    let mut held: Vec<UnixStream> = Vec::new();
    for dialer in dialers {
        if let Ok(Some(stream)) = dialer.await {
            held.push(stream);
        }
    }
    assert!(
        !held.is_empty(),
        "expected at least one pending connection to saturate the backlog"
    );

    // With the backlog full, delivery cannot reach a listener: on Linux the
    // connect blocks and the single R16 deadline cuts it; on macOS the full
    // queue makes the connect refuse instantly (ECONNREFUSED). Both end in
    // the same harmless `ListenerUnavailable` — never success, never a
    // hang, and never a failure Claude would see (R16).
    let record = parsed(&session_start_payload("sess-busy", "/work/busy", None, &[]));
    let started = tokio::time::Instant::now();
    let outcome = claude_hook::deliver_to(&record, socket.path()).await;
    let elapsed = started.elapsed();
    assert_eq!(outcome, DeliveryOutcome::ListenerUnavailable);
    assert!(
        elapsed <= claude_hook::DELIVERY_TIMEOUT + Duration::from_millis(500),
        "delivery must not exceed the R16 deadline by more than scheduling slack (took {elapsed:?})"
    );

    drop(held);
    drop(listener);
}

#[tokio::test]
async fn sentinels_never_appear_in_logs_or_wire_frames() {
    let socket = TempSocket::new("sentinels");
    let listener = bind_listener(&socket);

    let logs = capture_module_logs(async {
        // Each dropped payload carries a distinct sentinel class; the
        // category-only log line must never include any of them.
        assert_dropped(
            "{\"hook_event_name\":\"PreToolUse\",\"session_id\":\"s\",\"cwd\":\"/w\",\"prompt\":\"SENTINEL_PROMPT\",\"tool_name\":\"Edit\",\"tool_input\":{\"file_path\":\"SENTINEL_TOOL_INPUT\"},\"tool_response\":\"SENTINEL_TOOL_OUTPUT\"}",
            DropReason::UnknownEvent,
        );
        assert_dropped(
            "{\"hook_event_name\":\"StopFailure\",\"session_id\":\"s\",\"cwd\":\"\",\"error\":\"SENTINEL_ERROR\"}",
            DropReason::InvalidCwd,
        );
        assert_dropped(
            "{\"hook_event_name\":\"SessionStart\",\"session_id\":\"\",\"cwd\":\"/w\",\"transcript_path\":\"SENTINEL_TRANSCRIPT\",\"last_assistant_message\":\"SENTINEL_ASSISTANT\"}",
            DropReason::InvalidSessionId,
        );
        assert_dropped(
            "this is not json with SENTINEL_MALFORMED in it",
            DropReason::MalformedInput,
        );

        // An accepted record whose rejected extras carry every class; the
        // record itself keeps none of them.
        let record = parsed(&session_start_payload(
            "sess-sentinel",
            "/work/sentinel",
            Some("startup"),
            &[
                ("prompt", "SENTINEL_PROMPT"),
                ("last_assistant_message", "SENTINEL_ASSISTANT"),
                ("transcript_path", "SENTINEL_TRANSCRIPT"),
                ("tool_input", "SENTINEL_TOOL_INPUT"),
                ("tool_response", "SENTINEL_TOOL_OUTPUT"),
                ("error", "SENTINEL_ERROR"),
                ("arbitrary_user_field", "SENTINEL_ARBITRARY"),
            ],
        ));

        // Delivery to an absent listener logs a category-only line too.
        let absent = TempSocket::new("log-absent");
        let outcome = claude_hook::deliver_to(&record, absent.path()).await;
        assert_eq!(outcome, DeliveryOutcome::ListenerAbsent);

        // And the accepted record is really delivered to a live listener.
        let outcome = claude_hook::deliver_to(&record, socket.path()).await;
        assert_eq!(outcome, DeliveryOutcome::Sent);
    })
    .await;

    // Capture is real: every expected category line is present.
    for category in [
        "dropped (unknown event)",
        "dropped (invalid cwd)",
        "dropped (invalid session id)",
        "dropped (malformed input)",
        "delivery unavailable (listener absent)",
    ] {
        assert!(
            logs.contains(category),
            "missing category line {category:?} in captured logs: {logs}"
        );
    }

    // No sentinel value and no rejected field name appears in logs.
    for leaked in [
        "SENTINEL_PROMPT",
        "SENTINEL_ASSISTANT",
        "SENTINEL_TRANSCRIPT",
        "SENTINEL_TOOL_INPUT",
        "SENTINEL_TOOL_OUTPUT",
        "SENTINEL_ERROR",
        "SENTINEL_MALFORMED",
        "SENTINEL_ARBITRARY",
        "transcript_path",
        "last_assistant_message",
        "tool_input",
        "tool_response",
        "arbitrary_user_field",
    ] {
        assert!(
            !logs.contains(leaked),
            "sentinel/field {leaked:?} leaked into logs: {logs}"
        );
    }

    // The wire frame for the accepted record is equally clean.
    let frames = collect_frames(listener, 1, Duration::from_secs(5)).await;
    assert_eq!(frames.len(), 1, "expected exactly one delivered envelope");
    for leaked in [
        "SENTINEL_PROMPT",
        "SENTINEL_ASSISTANT",
        "SENTINEL_TRANSCRIPT",
        "SENTINEL_TOOL_INPUT",
        "SENTINEL_TOOL_OUTPUT",
        "SENTINEL_ERROR",
        "SENTINEL_ARBITRARY",
        "transcript",
    ] {
        assert!(
            !frames[0].contains(leaked),
            "sentinel {leaked:?} leaked into the wire frame: {}",
            frames[0]
        );
    }
}
