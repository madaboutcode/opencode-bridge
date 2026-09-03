//! T03 feature verification and integration coverage for the Claude adapter.
//!
//! Unlike T02's `claude_ingress.rs` (which compiled `hook.rs` via `#[path]`
//! because the module was not yet wired into the library), this file links the
//! real `dashboard` library: it proves the T03 path is compiled, registered,
//! and executable — real T02 Unix-socket delivery -> `wire::decode_envelope` ->
//! a live `ClaudeAdapter` task -> provider-neutral `SessionEvent`s and the
//! terminal `Gone` tombstone.
//!
//! FEATURE TEST: `feature_real_socket_delivery_reaches_adapter_snapshot_and_gone`
//! exercises the whole owned path with a real Unix socket and T02's
//! `deliver_to`, and fails if the adapter path is removed.
//!
//! Privacy scope: no test reads or writes Claude configuration or transcripts;
//! every socket lives under the OS temp dir (unique per test, removed on
//! drop); sentinels never cross the decode, state, or event boundaries.
//!
//! CONTRACT: ClaudeAdapter feature verification (T03;
//! `tasks/2026-09-03-claude-dashboard-support/contracts/T03-claude-adapter.md`)

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use dashboard::adapter::SessionEvent;
use dashboard::claude::hook::{
    deliver_to, parse_hook_input, ClaudeHookRecord, DeliveryOutcome, ParseOutcome, ReceivedAt,
};
use dashboard::claude::wire::{decode_envelope, DecodeError};
use dashboard::claude::ClaudeAdapter;
use dashboard::snapshot::{AttentionState, HarnessKind, SessionId, SessionSnapshot, Timestamp};
use dashboard::HarnessAdapter;

/// Fixed receipt timestamps (epoch millis), distinct and increasing.
const R1: u64 = 1_700_000_000_100;
const R2: u64 = 1_700_000_000_200;
const R3: u64 = 1_700_000_000_300;

fn ts(millis: u64) -> Timestamp {
    Timestamp::from_epoch_millis(millis as i64)
}

fn parsed(input: &str, received_at: ReceivedAt) -> ClaudeHookRecord {
    match parse_hook_input(input, received_at) {
        ParseOutcome::Accepted(record) => record,
        ParseOutcome::Dropped(reason) => panic!("expected accepted hook input, got {reason:?}"),
    }
}

fn session_start_payload(session_id: &str, cwd: &str, source: Option<&str>) -> String {
    let source = match source {
        Some(source) => format!("\"source\":\"{source}\","),
        None => String::new(),
    };
    format!(
        "{{\"hook_event_name\":\"SessionStart\",\"session_id\":\"{session_id}\",\"cwd\":\"{cwd}\",{source} \"transcript_path\":\"SENTINEL_TRANSCRIPT\",\"prompt\":\"SENTINEL_PROMPT\"}}"
    )
}

fn stop_failure_payload(session_id: &str, cwd: &str) -> String {
    format!(
        "{{\"hook_event_name\":\"StopFailure\",\"session_id\":\"{session_id}\",\"cwd\":\"{cwd}\",\"error\":\"SENTINEL_ERROR\",\"last_assistant_message\":\"SENTINEL_ASSISTANT\"}}"
    )
}

fn session_end_payload(session_id: &str, cwd: &str, reason: &str) -> String {
    format!(
        "{{\"hook_event_name\":\"SessionEnd\",\"session_id\":\"{session_id}\",\"cwd\":\"{cwd}\",\"reason\":\"{reason}\",\"transcript_path\":\"SENTINEL_TRANSCRIPT\"}}"
    )
}

/// A unique-per-test socket path in the OS temp dir, removed on drop.
struct TempSocket(PathBuf);

impl TempSocket {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "dashboard-claude-adapter-test-{}-{name}.sock",
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

/// Accepts one delivery and reads the whole frame (one newline-terminated
/// envelope) off the accepted connection.
async fn read_one_frame(listener: &UnixListener) -> String {
    let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
        .await
        .expect("timed out waiting for a delivery")
        .expect("accept failed");
    let mut body = String::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_string(&mut body))
        .await
        .expect("timed out reading the frame")
        .expect("read failed");
    body
}

/// Spawns a live adapter task and returns its input sender, event receiver,
/// and handle. Dropping the sender closes the adapter's input channel.
fn spawn_adapter() -> (
    UnboundedSender<dashboard::claude::hook::ClaudeIpcEnvelope>,
    UnboundedReceiver<SessionEvent>,
    JoinHandle<()>,
) {
    let (tx, adapter) = ClaudeAdapter::channel();
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<SessionEvent>();
    let handle = Box::new(adapter).run(event_tx);
    (tx, event_rx, handle)
}

fn assert_metadata_only(snapshot: &SessionSnapshot) {
    assert_eq!(snapshot.parent_id, None);
    assert_eq!(snapshot.current_action, None);
    assert_eq!(snapshot.wire_title, None);
    assert_eq!(snapshot.final_assistant_text, None);
    assert_eq!(snapshot.last_user_prompt, None);
    assert!(snapshot.files_touched.is_empty());
    assert!(snapshot.recent_actions.is_empty());
}

/// The canonical identity of this repo's root, resolved exactly the way the
/// adapter resolves it (GitDirResolver -> git toplevel -> canonicalize).
fn repo_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(manifest)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .expect("run git rev-parse --show-toplevel");
    assert!(output.status.success(), "git rev-parse failed");
    let toplevel = String::from_utf8_lossy(&output.stdout).trim().to_string();
    std::fs::canonicalize(toplevel).expect("canonicalize repo root")
}

// ---------------------------------------------------------------------------
// FEATURE TEST: real Unix socket -> T02 deliver_to -> wire decode -> live
// adapter task -> provider-neutral snapshots and the terminal `Gone`.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn feature_real_socket_delivery_reaches_adapter_snapshot_and_gone() {
    let socket = TempSocket::new("feature");
    let listener = bind_listener(&socket);
    let root = repo_root();
    let cwd = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).expect("canonicalize manifest dir");
    let cwd_str = cwd.to_string_lossy().to_string();

    // T02 hook payloads (carrying sentinels that must never cross), turned
    // into records with explicit local receipt timestamps.
    let start = parsed(
        &session_start_payload("sess-feature", &cwd_str, Some("startup")),
        ReceivedAt(R1),
    );
    let stop = parsed(
        &stop_failure_payload("sess-feature", &cwd_str),
        ReceivedAt(R2),
    );
    let end = parsed(
        &session_end_payload("sess-feature", &cwd_str, "other"),
        ReceivedAt(R3),
    );

    // Real Unix socket delivery, one connection per record, one frame each.
    let mut frames = Vec::new();
    for record in [&start, &stop, &end] {
        let outcome = deliver_to(record, socket.path()).await;
        assert_eq!(outcome, DeliveryOutcome::Sent, "record must deliver");
        let frame = read_one_frame(&listener).await;
        assert!(frame.ends_with('\n'), "envelope is newline-delimited");
        assert!(
            !frame.contains("SENTINEL"),
            "no rejected value on the wire: {frame}"
        );
        frames.push(frame);
    }

    // T03 decoder: each wire line becomes a typed, validated envelope.
    let mut envelopes = Vec::new();
    for frame in &frames {
        let envelope = decode_envelope(frame).expect("decode T02 frame");
        assert_eq!(envelope.protocol_version, 1);
        envelopes.push(envelope);
    }

    // Live adapter task: submit the envelopes in receipt order.
    let (tx, mut events, handle) = spawn_adapter();
    for envelope in envelopes {
        tx.send(envelope).unwrap();
    }
    drop(tx);

    // SessionStart -> complete Claude snapshot with canonical project identity.
    let first = events.recv().await.expect("SessionStart snapshot");
    let SessionEvent::Snapshot(first_snapshot) = &first else {
        panic!("expected snapshot, got {first:?}");
    };
    assert_eq!(first_snapshot.session_id.harness, HarnessKind("claude"));
    assert_eq!(first_snapshot.session_id.native_id, "sess-feature");
    assert_eq!(
        first_snapshot.project_id.as_path(),
        root.as_path(),
        "canonical git toplevel project identity"
    );
    assert_eq!(
        first_snapshot.attention,
        AttentionState::NeedsYou {
            question: false,
            turn_ended: ts(R1)
        }
    );
    assert_eq!(first_snapshot.created_at, ts(R1));
    assert_eq!(first_snapshot.last_updated, ts(R1));
    assert_metadata_only(first_snapshot);

    // StopFailure -> attention stays NeedsYou, last_updated advances, creation
    // time is preserved.
    let second = events.recv().await.expect("StopFailure snapshot");
    let SessionEvent::Snapshot(second_snapshot) = &second else {
        panic!("expected snapshot, got {second:?}");
    };
    assert_eq!(
        second_snapshot.attention,
        AttentionState::NeedsYou {
            question: false,
            turn_ended: ts(R2)
        }
    );
    assert_eq!(
        second_snapshot.created_at,
        ts(R1),
        "creation time preserved"
    );
    assert_eq!(
        second_snapshot.last_updated,
        ts(R2),
        "receipt time advances"
    );
    assert_eq!(second_snapshot.project_id.as_path(), root.as_path());
    assert_metadata_only(second_snapshot);

    // SessionEnd -> terminal tombstone and clean channel closure.
    let third = events.recv().await.expect("SessionEnd tombstone");
    assert_eq!(
        third,
        SessionEvent::Gone(SessionId::new(HarnessKind("claude"), "sess-feature"))
    );
    assert_eq!(events.recv().await, None, "no further events after Gone");
    handle
        .await
        .expect("adapter task must end cleanly after input closes");
}

// ---------------------------------------------------------------------------
// Lifecycle and identity coverage through the public boundary.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn session_end_for_an_untracked_native_id_still_emits_gone() {
    let record = parsed(
        &session_end_payload("never-seen", "/work/x", "clear"),
        ReceivedAt(R3),
    );
    let (tx, mut events, handle) = spawn_adapter();
    tx.send(dashboard::claude::hook::ClaudeIpcEnvelope::new(record))
        .unwrap();
    drop(tx);

    let gone = events
        .recv()
        .await
        .expect("SessionEnd first event still tombstones");
    assert_eq!(
        gone,
        SessionEvent::Gone(SessionId::new(HarnessKind("claude"), "never-seen"))
    );
    assert_eq!(events.recv().await, None);
    handle.await.expect("adapter task must end cleanly");
}

#[tokio::test]
async fn duplicate_start_preserves_creation_time_through_the_adapter() {
    let cwd = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
        .unwrap()
        .to_string_lossy()
        .to_string();
    let first = parsed(&session_start_payload("dup", &cwd, None), ReceivedAt(R1));
    let second = parsed(&session_start_payload("dup", &cwd, None), ReceivedAt(R2));
    let (tx, mut events, handle) = spawn_adapter();
    tx.send(dashboard::claude::hook::ClaudeIpcEnvelope::new(first))
        .unwrap();
    tx.send(dashboard::claude::hook::ClaudeIpcEnvelope::new(second))
        .unwrap();
    drop(tx);

    let SessionEvent::Snapshot(s1) = events.recv().await.expect("first start") else {
        panic!("expected snapshot");
    };
    let SessionEvent::Snapshot(s2) = events.recv().await.expect("duplicate start") else {
        panic!("expected snapshot");
    };
    assert_eq!(s1.created_at, ts(R1));
    assert_eq!(
        s2.created_at,
        ts(R1),
        "duplicate start keeps first receipt as creation"
    );
    assert_eq!(s2.last_updated, ts(R2));
    assert_eq!(events.recv().await, None);
    handle.await.expect("adapter task must end cleanly");
}

#[tokio::test]
async fn missing_project_directory_degrades_one_snapshot_and_adapter_continues() {
    // A cwd that cannot exist: canonicalization fails, the documented degraded
    // identity (the raw, uncanonicalized path) is used, and the adapter stays
    // alive for the next record.
    let missing = "/nowhere/project-that-does-not-exist";
    let start = parsed(
        &session_start_payload("bad-cwd", missing, None),
        ReceivedAt(R1),
    );
    let end = parsed(
        &session_end_payload("good", "/work/ok", "other"),
        ReceivedAt(R3),
    );
    let (tx, mut events, handle) = spawn_adapter();
    tx.send(dashboard::claude::hook::ClaudeIpcEnvelope::new(start))
        .unwrap();
    tx.send(dashboard::claude::hook::ClaudeIpcEnvelope::new(end))
        .unwrap();
    drop(tx);

    let SessionEvent::Snapshot(snapshot) = events.recv().await.expect("degraded snapshot") else {
        panic!("expected snapshot");
    };
    assert_eq!(snapshot.session_id.native_id, "bad-cwd");
    assert_eq!(
        snapshot.project_id.as_path(),
        Path::new(missing),
        "degraded identity is the raw cwd"
    );
    assert_eq!(snapshot.created_at, ts(R1));

    // The adapter continued: the next record still produced its event.
    let gone = events
        .recv()
        .await
        .expect("adapter continues after degraded session");
    assert_eq!(
        gone,
        SessionEvent::Gone(SessionId::new(HarnessKind("claude"), "good"))
    );
    assert_eq!(events.recv().await, None);
    handle.await.expect("adapter task must end cleanly");
}

#[tokio::test]
async fn adapter_survives_an_unknown_protocol_envelope_without_emitting_state() {
    let cwd = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
        .unwrap()
        .to_string_lossy()
        .to_string();
    let (tx, mut events, handle) = spawn_adapter();

    // A record that could not come from the decoder (wrong protocol version),
    // submitted by a future caller.
    let mut bad = dashboard::claude::hook::ClaudeIpcEnvelope::new(parsed(
        &session_start_payload("sess-bad", &cwd, None),
        ReceivedAt(R1),
    ));
    bad.protocol_version = 99;
    tx.send(bad).unwrap();

    // Valid input after the bad record still processes.
    let end = parsed(
        &session_end_payload("sess-ok", &cwd, "other"),
        ReceivedAt(R3),
    );
    tx.send(dashboard::claude::hook::ClaudeIpcEnvelope::new(end))
        .unwrap();
    drop(tx);

    let gone = events
        .recv()
        .await
        .expect("only the valid record produces an event");
    assert_eq!(
        gone,
        SessionEvent::Gone(SessionId::new(HarnessKind("claude"), "sess-ok"))
    );
    assert_eq!(
        events.recv().await,
        None,
        "the unknown-protocol record emitted nothing"
    );
    handle
        .await
        .expect("adapter must keep running and end cleanly");
}

#[tokio::test]
async fn channel_closure_stops_the_adapter_task_cleanly() {
    let (tx, _adapter_keepalive, handle) = spawn_adapter();
    // Holding the adapter in `_adapter_keepalive` would keep its channel
    // receiver alive; dropping the sender alone must not leak the task.
    drop(tx);
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("adapter must stop within the timeout")
        .expect("adapter task panicked");
}

// ---------------------------------------------------------------------------
// Wire-decoder rejection coverage at the public boundary.
// ---------------------------------------------------------------------------

#[test]
fn decoder_rejects_malformed_unknown_protocol_unknown_event_and_out_of_bounds() {
    let valid = "{\"protocol_version\":1,\"record\":{\"session_id\":\"s\",\"cwd\":\"/w\",\"event\":{\"kind\":\"session_start\"},\"received_at\":1}}";

    let cases: Vec<(String, DecodeError)> = vec![
        ("not json SENTINEL".to_string(), DecodeError::Malformed),
        (
            r#"{"protocol_version":1}"#.to_string(),
            DecodeError::Malformed,
        ),
        (valid.replace("1", "2"), DecodeError::UnknownVersion),
        (
            valid.replace("session_start", "session_stop"),
            DecodeError::UnknownEvent,
        ),
        (
            format!(
                r#"{{"protocol_version":1,"record":{{"session_id":"{}","cwd":"/w","event":{{"kind":"session_start"}},"received_at":1}}}}"#,
                "i".repeat(dashboard::claude::hook::MAX_SESSION_ID_LEN + 1)
            ),
            DecodeError::OutOfBounds,
        ),
    ];
    for (line, expected) in cases {
        assert_eq!(decode_envelope(&line), Err(expected), "line: {line:.60}");
    }

    // Errors carry only the category, never the rejected values.
    let err = decode_envelope("SENTINEL_MALFORMED{").unwrap_err();
    assert!(!format!("{err:?}").contains("SENTINEL"));
}

// ---------------------------------------------------------------------------
// Cross-harness native-id separation at the public identity boundary.
// ---------------------------------------------------------------------------

#[test]
fn same_native_id_is_distinct_across_harness_kinds() {
    let claude = SessionId::new(HarnessKind("claude"), "123");
    let opencode = SessionId::new(HarnessKind("opencode"), "123");
    assert_ne!(
        claude, opencode,
        "same native id never collides across harnesses"
    );
    assert_eq!(claude.harness, HarnessKind("claude"));
    assert_eq!(opencode.harness, HarnessKind("opencode"));
    assert_eq!(claude.native_id, "123");
    assert_eq!(opencode.native_id, "123");
}
