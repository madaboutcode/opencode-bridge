//! T04 feature verification for the Claude runtime — `dashboard claude-hook`
//! and the startup listener (T04; `tasks/2026-09-03-claude-dashboard-support/
//! contracts/T04-claude-runtime.md`; `docs/specs/dashboard/claude.md`
//! R11/R12/R16).
//!
//! The feature test spawns the **built** `dashboard claude-hook` binary from
//! this package (`CARGO_BIN_EXE_dashboard`), feeds a real hook payload on
//! stdin, reads exactly one newline-delimited envelope from a real Unix
//! listener, decodes it with T03's `decode_envelope`, runs it through a live
//! T03 `ClaudeAdapter`, and asserts a provider-neutral event — plus invalid
//! input, timeout, saturation, socket-prep, and cleanup behavior on the T04
//! listener itself.
//!
//! Privacy scope: no test reads or writes Claude configuration or
//! transcripts. Every socket lives under the OS temp dir, unique per test and
//! removed on drop; the helper subprocess gets its socket exclusively via
//! `DASHBOARD_CLAUDE_SOCKET`, never a global/project Claude path. Negative
//! tests assert sentinel values never cross the IPC or log boundaries.
//!
//! CONTRACT: ClaudeRuntime feature verification (T04;
//! `tasks/2026-09-03-claude-dashboard-support/contracts/T04-claude-runtime.md`)

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use std::io::Write;
use std::os::unix::fs::FileTypeExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

use dashboard::adapter::SessionEvent;
use dashboard::claude::hook::{
    deliver_to, parse_hook_input, serialize_envelope, ClaudeHookRecord, ClaudeIpcEnvelope,
    DeliveryOutcome, ParseOutcome, ReceivedAt, MAX_ENVELOPE_BYTES, MAX_HOOK_INPUT_BYTES,
    MAX_SESSION_ID_LEN,
};
use dashboard::claude::listener::{ClaudeListener, ListenerError, MAX_CONCURRENT_CONNECTIONS};
use dashboard::claude::wire::decode_envelope;
use dashboard::claude::ClaudeAdapter;
use dashboard::snapshot::{AttentionState, HarnessKind, SessionSnapshot};
use dashboard::HarnessAdapter;

const RECEIVED: ReceivedAt = ReceivedAt(1_700_000_000_000);

/// A unique-per-test socket path in the OS temp dir, removed on drop.
struct TempSocket(PathBuf);

impl TempSocket {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "dashboard-claude-runtime-test-{}-{name}.sock",
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

/// Runs the built `dashboard claude-hook` subprocess with `input` on stdin
/// and `DASHBOARD_CLAUDE_SOCKET` set to `socket`. Runs on a std thread so a
/// hung helper fails the test rather than hanging the suite: the helper must
/// exit on its own (bounded read + bounded delivery), well inside the 15s
/// guard.
fn run_hook(input: &[u8], socket: &Path) -> Output {
    let (tx, rx) = std::sync::mpsc::channel();
    let input = input.to_vec();
    let socket = socket.to_path_buf();
    std::thread::spawn(move || {
        let mut child = Command::new(env!("CARGO_BIN_EXE_dashboard"))
            .arg("claude-hook")
            .env("DASHBOARD_CLAUDE_SOCKET", &socket)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn dashboard claude-hook");
        // The helper may exit before a large oversized write completes — that
        // is exactly the oversized case — so a broken pipe here is expected
        // and ignored; the exit/output assertions below still hold.
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(&input);
        }
        let output = child.wait_with_output().expect("wait for hook process");
        let _ = tx.send(output);
    });
    rx.recv_timeout(Duration::from_secs(15))
        .expect("dashboard claude-hook must exit within 15s")
}

/// Parses a payload expected to be accepted; panics otherwise.
fn parsed(input: &str) -> ClaudeHookRecord {
    match parse_hook_input(input, RECEIVED) {
        ParseOutcome::Accepted(record) => record,
        ParseOutcome::Dropped(reason) => panic!("expected accepted hook input, got {reason:?}"),
    }
}

/// A T02-serialized valid envelope line (newline-terminated) for listener
/// tests: the same bytes `deliver_to` would write.
fn valid_line(session_id: &str, cwd: &str) -> String {
    serialize_envelope(&parsed(
        &serde_json::json!({
            "hook_event_name": "SessionStart",
            "session_id": session_id,
            "cwd": cwd,
        })
        .to_string(),
    ))
}

/// A T02-serialized valid envelope whose wire length is exactly `target`
/// bytes, with the newline on the final byte. A real T02 envelope is far
/// smaller than `MAX_ENVELOPE_BYTES`, so the body is padded with trailing
/// whitespace — legal JSON — to reach the exact boundary. This is what an
/// exact-boundary first frame looks like to the listener's frame reader.
fn valid_line_of_exact_length(session_id: &str, target: usize) -> String {
    assert!(
        target <= MAX_ENVELOPE_BYTES,
        "exact target must stay within the envelope bound"
    );
    let base = valid_line(session_id, "/w");
    assert!(
        base.len() < target,
        "a serialized envelope must be smaller than the exact target"
    );
    let body = base.trim_end_matches('\n');
    let mut padded = format!("{body}{}", " ".repeat(target - base.len()));
    padded.push('\n');
    assert_eq!(padded.len(), target);
    assert!(padded.ends_with('\n'), "newline is the final byte");
    padded
}

/// The canonical identity of this repo's root, resolved the way the adapter
/// resolves it (git toplevel -> canonicalize).
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

fn assert_metadata_only(snapshot: &SessionSnapshot) {
    assert_eq!(snapshot.parent_id, None);
    assert_eq!(snapshot.current_action, None);
    assert_eq!(snapshot.wire_title, None);
    assert_eq!(snapshot.final_assistant_text, None);
    assert_eq!(snapshot.last_user_prompt, None);
    assert!(snapshot.files_touched.is_empty());
    assert!(snapshot.recent_actions.is_empty());
}

/// Reads bytes from the accepted stream up to and including the first
/// newline (or EOF), the one-frame contract of the T02/Claude hook.
async fn read_one_line(listener: &UnixListener) -> String {
    let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
        .await
        .expect("timed out waiting for the hook's connection")
        .expect("accept failed");
    let mut buf = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        let n = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut byte))
            .await
            .expect("timed out reading the frame")
            .expect("read failed");
        if n == 0 {
            break;
        }
        buf.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    String::from_utf8(buf).expect("hook frame must be valid UTF-8")
}

/// Starts a T04 listener at `path` and returns its envelope channel receiver
/// and task handle.
fn spawn_listener(
    path: &Path,
) -> (
    UnboundedReceiver<ClaudeIpcEnvelope>,
    tokio::task::JoinHandle<()>,
) {
    let listener = ClaudeListener::bind_at(path).expect("bind T04 listener");
    let (tx, rx) = unbounded_channel();
    let handle = listener.run(tx);
    (rx, handle)
}

/// Delivers a valid `SessionStart` record for `session_id` via T02
/// `deliver_to` and asserts the envelope reaches `rx` — with no second
/// envelope.
async fn assert_one_valid_delivery(
    socket: &TempSocket,
    rx: &mut UnboundedReceiver<ClaudeIpcEnvelope>,
    session_id: &str,
) {
    let record = parsed(
        &serde_json::json!({
            "hook_event_name": "SessionStart",
            "session_id": session_id,
            "cwd": env!("CARGO_MANIFEST_DIR"),
        })
        .to_string(),
    );
    let outcome = deliver_to(&record, socket.path()).await;
    assert_eq!(outcome, DeliveryOutcome::Sent, "valid record must deliver");
    let envelope = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("listener must forward the valid envelope")
        .expect("channel closed unexpectedly");
    assert_eq!(envelope.record.session_id, session_id);
    assert!(tokio::time::timeout(Duration::from_millis(400), rx.recv())
        .await
        .is_err());
}

// ---------------------------------------------------------------------------
// FEATURE TEST: built `dashboard claude-hook` subprocess -> real Unix socket
// -> exactly one newline envelope -> T03 decode -> live adapter ->
// provider-neutral event.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn feature_hook_subprocess_writes_one_envelope_decoded_into_adapter_event() {
    let socket = TempSocket::new("feature");
    let listener = UnixListener::bind(socket.path()).expect("bind test listener");
    let root = repo_root();

    // A valid SessionStart carrying sentinels that must never survive.
    let payload = serde_json::json!({
        "hook_event_name": "SessionStart",
        "session_id": "sess-t04-feature",
        "cwd": env!("CARGO_MANIFEST_DIR"),
        "source": "startup",
        "transcript_path": "SENTINEL_TRANSCRIPT",
        "prompt": "SENTINEL_PROMPT",
    })
    .to_string();

    let output = run_hook(payload.as_bytes(), socket.path());
    assert!(output.status.success(), "hook must exit 0");
    assert!(output.stdout.is_empty(), "hook must never write stdout");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("SENTINEL"),
        "rejected values leaked to stderr: {stderr}"
    );

    // Exactly one newline-delimited envelope arrives.
    let line = read_one_line(&listener).await;
    assert!(
        line.ends_with('\n'),
        "envelope is newline-delimited: {line:?}"
    );
    assert_eq!(line.matches('\n').count(), 1, "exactly one frame: {line:?}");
    assert!(
        !line.contains("SENTINEL") && !line.contains("transcript"),
        "privacy-boundary violation on the wire: {line:?}"
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(300), listener.accept())
            .await
            .is_err(),
        "one hook run must deliver exactly one connection"
    );

    // T03 decode, then a live adapter task.
    let envelope = decode_envelope(&line).expect("decode the T02 frame");
    assert_eq!(envelope.protocol_version, 1);
    assert_eq!(envelope.record.session_id, "sess-t04-feature");

    let (tx, adapter) = ClaudeAdapter::channel();
    let (event_tx, mut event_rx) = unbounded_channel::<SessionEvent>();
    let adapter_handle = Box::new(adapter).run(event_tx);
    tx.send(envelope).unwrap();
    drop(tx);

    // Provider-neutral event, complete Claude snapshot with canonical
    // project identity and no sentinel content.
    let event = tokio::time::timeout(Duration::from_secs(5), event_rx.recv())
        .await
        .expect("adapter must emit the start snapshot")
        .expect("adapter channel closed unexpectedly");
    let SessionEvent::Snapshot(snapshot) = &event else {
        panic!("expected snapshot, got {event:?}");
    };
    assert_eq!(snapshot.session_id.harness, HarnessKind("claude"));
    assert_eq!(snapshot.session_id.native_id, "sess-t04-feature");
    assert_eq!(
        snapshot.project_id.as_path(),
        root.as_path(),
        "canonical git toplevel project identity"
    );
    assert!(
        matches!(
            snapshot.attention,
            AttentionState::NeedsYou {
                question: false,
                ..
            }
        ),
        "SessionStart maps to NeedsYou (question: false): {:?}",
        snapshot.attention
    );
    assert_metadata_only(snapshot);
    assert!(
        !format!("{snapshot:?}").contains("SENTINEL"),
        "no rejected value may reach the provider-neutral event"
    );
    // One envelope must produce exactly one event: either the adapter has
    // already closed (channel clean) or a short wait times out — never a
    // second event.
    if let Ok(Some(extra)) = tokio::time::timeout(Duration::from_millis(300), event_rx.recv()).await
    {
        panic!("one envelope produced a second event: {extra:?}");
    }
    adapter_handle
        .await
        .expect("adapter must end cleanly after input closes");
}

// ---------------------------------------------------------------------------
// Command-side drops: malformed/unknown/oversized/invalid-UTF-8 inputs exit
// 0, write no frame, and never leak sentinels to stdout or stderr.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hook_dropped_inputs_exit_zero_with_no_frame_and_no_sentinel() {
    let scaffold = format!(
        r#"{{"hook_event_name":"SessionStart","session_id":"s","cwd":"/w","pad":"{pad}"}}"#,
        pad = ""
    );
    let oversized = format!(
        r#"{{"hook_event_name":"SessionStart","session_id":"s","cwd":"/w","pad":"{pad}"}}"#,
        pad = "x".repeat(MAX_HOOK_INPUT_BYTES + 1 - scaffold.len())
    );
    assert_eq!(oversized.len(), MAX_HOOK_INPUT_BYTES + 1);

    let cases: Vec<(&str, Vec<u8>, &str)> = vec![
        (
            "malformed",
            b"{ definitely not json SENTINEL_MALFORMED".to_vec(),
            "dropped (malformed input)",
        ),
        (
            "unknown",
            br#"{"hook_event_name":"UserPromptSubmit","session_id":"s","cwd":"/w","prompt":"SENTINEL_PROMPT"}"#.to_vec(),
            "dropped (unknown event)",
        ),
        ("oversized", oversized.into_bytes(), "dropped (oversized input)"),
        (
            "invalid-utf8",
            vec![b'{', b'"', b'h', 0xff, 0xfe, b'}'],
            "dropped (invalid utf-8)",
        ),
    ];

    for (name, input, category) in cases {
        let socket = TempSocket::new(name);
        let listener = UnixListener::bind(socket.path()).expect("bind test listener");
        let output = run_hook(&input, socket.path());
        assert!(output.status.success(), "{name}: hook must exit 0");
        assert!(
            output.stdout.is_empty(),
            "{name}: hook must not write stdout"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(category),
            "{name}: missing category {category:?} in {stderr}"
        );
        assert!(
            !stderr.contains("SENTINEL"),
            "{name}: rejected values leaked to stderr: {stderr}"
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(300), listener.accept())
                .await
                .is_err(),
            "{name}: dropped input must never reach the socket"
        );
    }
}

#[tokio::test]
async fn hook_valid_payload_with_absent_listener_exits_zero_without_leaking() {
    let socket = TempSocket::new("absent");
    let payload = serde_json::json!({
        "hook_event_name": "SessionStart",
        "session_id": "sess-absent",
        "cwd": env!("CARGO_MANIFEST_DIR"),
        "transcript_path": "SENTINEL_TRANSCRIPT",
    })
    .to_string();

    let output = run_hook(payload.as_bytes(), socket.path());
    assert!(
        output.status.success(),
        "hook must exit 0 when listener is absent"
    );
    assert!(output.stdout.is_empty(), "hook must not write stdout");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("delivery unavailable (listener absent)")
            || stderr.contains("delivery unavailable (listener unavailable)"),
        "expected a category-only delivery-unavailable line, got: {stderr}"
    );
    assert!(
        !stderr.contains("SENTINEL") && !stderr.contains("transcript"),
        "rejected values leaked to stderr: {stderr}"
    );
}

#[test]
fn hook_exits_within_read_deadline_when_stdin_stays_open() {
    // A stalled runner: partial bytes on stdin, then the pipe stays open —
    // no EOF and no more data, so the bounded read can never complete. The
    // helper must give up at its own read deadline and the *process* must
    // terminate within a tight bound even though its stdin is still open
    // (T04 review: hook helper lifetime). The suite guard (~15s) plus the
    // tight assertion below make this fail loudly instead of hanging if the
    // deadline were not a real process bound.
    let socket = TempSocket::new("stall");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut child = Command::new(env!("CARGO_BIN_EXE_dashboard"))
            .arg("claude-hook")
            .env("DASHBOARD_CLAUDE_SOCKET", socket.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn dashboard claude-hook");
        // The stdin handle is deliberately kept alive for the whole wait: the
        // pipe never closes, so only the helper's own deadline can end it.
        let mut stdin = child.stdin.take().expect("piped stdin");
        let _ = stdin.write_all(b"{\"hook_event_name\":\"SessionStart\"");
        let started = std::time::Instant::now();
        let output = child.wait_with_output().expect("wait for hook process");
        let _ = tx.send((output, started.elapsed()));
    });
    let (output, elapsed) = rx
        .recv_timeout(Duration::from_secs(15))
        .expect("dashboard claude-hook must exit even with stdin held open");
    assert!(output.status.success(), "stalled stdin must still exit 0");
    assert!(output.stdout.is_empty(), "hook must never write stdout");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("stdin read timed out"),
        "expected the timeout category, got: {stderr}"
    );
    // Tight bound: HOOK_READ_TIMEOUT is 1s, so the process must be gone well
    // before the suite guard (slack for CI scheduling only).
    assert!(
        elapsed < Duration::from_secs(6),
        "hook took {elapsed:?} to exit with stdin held open"
    );
}

// ---------------------------------------------------------------------------
// Listener: valid delivery, bad-client drops with later-valid survival,
// silence timeout, socket prep (stale/regular/symlink), saturation, cleanup.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn listener_valid_delivery_is_decoded_and_forwarded_once() {
    let socket = TempSocket::new("valid");
    let (mut rx, handle) = spawn_listener(socket.path());
    assert_one_valid_delivery(&socket, &mut rx, "valid-sess").await;
    handle.abort();
}

#[tokio::test]
async fn listener_drops_bad_clients_then_serves_a_later_valid_client() {
    let socket = TempSocket::new("bad-clients");
    let (mut rx, handle) = spawn_listener(socket.path());

    let out_of_bounds = format!(
        r#"{{"protocol_version":1,"record":{{"session_id":"{id}","cwd":"/w","event":{{"kind":"session_start"}},"received_at":1}}}}"#,
        id = "i".repeat(MAX_SESSION_ID_LEN + 1)
    );

    // Each bad client writes its bytes and closes (EOF).
    {
        let mut stream = UnixStream::connect(socket.path())
            .await
            .expect("connect malformed client");
        let _ = stream.write_all(b"not json SENTINEL_MALFORMED\n").await;
    }
    {
        let mut stream = UnixStream::connect(socket.path())
            .await
            .expect("connect unknown client");
        let _ = stream
            .write_all(
                br#"{"protocol_version":1,"record":{"session_id":"s","cwd":"/w","event":{"kind":"session_stop"},"received_at":1}}
"#,
            )
            .await;
    }
    {
        let mut stream = UnixStream::connect(socket.path())
            .await
            .expect("connect out-of-bounds client");
        let _ = stream.write_all(out_of_bounds.as_bytes()).await;
        let _ = stream.write_all(b"\n").await;
    }
    {
        // Two complete envelopes in one connection: a multiple-frame
        // violation, dropped whole regardless of kernel chunking.
        let two = format!("{}{}", valid_line("m1", "/w"), valid_line("m2", "/w"));
        let mut stream = UnixStream::connect(socket.path())
            .await
            .expect("connect multiple-frame client");
        let _ = stream.write_all(two.as_bytes()).await;
    }
    {
        // A partial line that never terminates: EOF without a newline.
        let mut stream = UnixStream::connect(socket.path())
            .await
            .expect("connect unterminated client");
        let _ = stream
            .write_all(br#"{"protocol_version":1,"record":{"session_id":"s","cwd":"/w""#)
            .await;
    }

    // None of the five bad clients may produce an envelope.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .is_err(),
        "bad clients must not reach the T03 channel"
    );

    // A later valid client still succeeds.
    assert_one_valid_delivery(&socket, &mut rx, "later-valid").await;
    handle.abort();
}

#[tokio::test]
async fn listener_silent_client_is_released_after_the_read_deadline() {
    let socket = TempSocket::new("silent");
    let (mut rx, handle) = spawn_listener(socket.path());

    // Connect and hold: no bytes at all. The listener must close the
    // connection once the finite read deadline expires.
    let mut silent = UnixStream::connect(socket.path())
        .await
        .expect("connect silent client");
    let started = std::time::Instant::now();
    let mut byte = [0u8; 1];
    let n = tokio::time::timeout(Duration::from_secs(5), silent.read(&mut byte))
        .await
        .expect("silent client must be released within the bounded deadline")
        .expect("read failed");
    assert_eq!(n, 0, "listener must close the silent connection (EOF)");
    assert!(
        started.elapsed() >= dashboard::claude::listener::FRAME_READ_TIMEOUT / 2,
        "release must come from the read deadline, not an immediate close"
    );

    // And a valid client still succeeds afterwards.
    assert_one_valid_delivery(&socket, &mut rx, "after-silent").await;
    handle.abort();
}

#[tokio::test]
async fn listener_replaces_a_stale_socket_and_binds() {
    let socket = TempSocket::new("stale");
    // A socket file with no listener behind it.
    {
        let stale = UnixListener::bind(socket.path()).expect("bind stale test listener");
        drop(stale);
    }
    assert!(
        socket.path().exists(),
        "a dropped Unix listener leaves its socket file behind"
    );

    let (mut rx, handle) = spawn_listener(socket.path());
    assert_one_valid_delivery(&socket, &mut rx, "stale-replaced").await;
    handle.abort();
}

/// Asserts `bind_at` refuses `path` with [`ListenerError::RefusedNonSocket`].
fn assert_refused(path: &Path) {
    match ClaudeListener::bind_at(path) {
        Ok(_) => panic!("non-socket path must be refused: {}", path.display()),
        Err(error) => assert_eq!(error, ListenerError::RefusedNonSocket),
    }
}

#[tokio::test]
async fn listener_refuses_a_regular_file_without_deleting_it() {
    let socket = TempSocket::new("regular");
    std::fs::write(socket.path(), b"SENTINEL_REGULAR").expect("write temp file");

    assert_refused(socket.path());
    assert_eq!(
        std::fs::read(socket.path()).expect("file must still exist"),
        b"SENTINEL_REGULAR",
        "refused non-socket paths are never deleted"
    );
}

#[tokio::test]
async fn listener_refuses_a_symlink_without_deleting_it() {
    let target = TempSocket::new("symlink-target");
    let socket = TempSocket::new("symlink");
    std::fs::write(target.path(), b"target").expect("write symlink target");
    std::os::unix::fs::symlink(target.path(), socket.path()).expect("create symlink");

    assert_refused(socket.path());
    assert!(
        std::fs::symlink_metadata(socket.path())
            .expect("symlink must still exist")
            .file_type()
            .is_symlink(),
        "a symlink at the target is never deleted"
    );
    assert_eq!(
        std::fs::read(target.path()).expect("symlink target must still exist"),
        b"target"
    );
}

#[tokio::test]
async fn listener_saturation_does_not_starve_a_later_valid_client() {
    let socket = TempSocket::new("saturation");
    let (mut rx, handle) = spawn_listener(socket.path());

    // Fill every concurrency slot with silent clients (held open).
    let mut silent = Vec::new();
    for _ in 0..MAX_CONCURRENT_CONNECTIONS {
        let stream =
            tokio::time::timeout(Duration::from_secs(5), UnixStream::connect(socket.path()))
                .await
                .expect("connect saturating client")
                .expect("connect failed");
        silent.push(stream);
    }

    // A valid delivery arrives while every slot is busy; its connection waits
    // for a slot, and the silent clients free theirs at the read deadline.
    let record = parsed(
        &serde_json::json!({
            "hook_event_name": "SessionStart",
            "session_id": "saturation-valid",
            "cwd": env!("CARGO_MANIFEST_DIR"),
        })
        .to_string(),
    );
    let outcome = deliver_to(&record, socket.path()).await;
    assert_eq!(outcome, DeliveryOutcome::Sent, "write reaches the listener");

    let started = std::time::Instant::now();
    let envelope = tokio::time::timeout(Duration::from_secs(10), rx.recv())
        .await
        .expect("the valid client must be served once silent slots free")
        .expect("channel closed unexpectedly");
    assert_eq!(envelope.record.session_id, "saturation-valid");
    // The wait is dominated by the bounded read deadline freeing the silent
    // slots; it must not be immediate (proves saturation actually held the
    // slots) and must be finite.
    assert!(started.elapsed() >= dashboard::claude::listener::FRAME_READ_TIMEOUT / 2);

    drop(silent);
    handle.abort();
}

#[tokio::test]
async fn listener_shutdown_removes_its_socket() {
    let socket = TempSocket::new("cleanup");
    let listener = ClaudeListener::bind_at(socket.path()).expect("bind listener");
    assert!(socket.path().exists(), "bind creates the socket file");

    let (tx, rx) = unbounded_channel::<ClaudeIpcEnvelope>();
    let handle = listener.run(tx);
    assert!(
        socket.path().exists(),
        "socket exists while the listener runs"
    );

    handle.abort();
    handle
        .await
        .expect_err("aborted listener task must report cancellation");

    assert!(
        !socket.path().exists(),
        "listener shutdown must remove its owned socket"
    );

    // And the path is taken over cleanly afterwards.
    let (mut rx2, handle2) = spawn_listener(socket.path());
    assert_one_valid_delivery(&socket, &mut rx2, "after-cleanup").await;
    handle2.abort();
    drop(rx);
}

#[tokio::test]
async fn listener_drop_without_run_removes_its_own_socket() {
    let socket = TempSocket::new("drop-own");
    let listener = ClaudeListener::bind_at(socket.path()).expect("bind listener");
    assert!(socket.path().exists(), "bind creates the socket file");
    drop(listener);
    assert!(
        !socket.path().exists(),
        "listener drop removes its own un-replaced socket"
    );
}

#[tokio::test]
async fn listener_drop_never_deletes_a_replaced_regular_file() {
    // The socket path is replaced by a regular file after bind — exactly the
    // scenario a blind remove_file on drop would destroy (T04 review: cleanup
    // ownership).
    let socket = TempSocket::new("rep-regular");
    let listener = ClaudeListener::bind_at(socket.path()).expect("bind listener");
    assert!(socket.path().exists(), "bind creates the socket file");

    std::fs::remove_file(socket.path()).expect("unlink the bound socket");
    std::fs::write(socket.path(), b"SENTINEL_REPLACEMENT").expect("write replacement file");

    drop(listener);
    assert_eq!(
        std::fs::read(socket.path()).expect("replacement file must still exist"),
        b"SENTINEL_REPLACEMENT",
        "drop must never delete a replacement regular file"
    );
}

#[tokio::test]
async fn listener_drop_never_deletes_a_replaced_symlink() {
    let target = TempSocket::new("rep-sym-target");
    let socket = TempSocket::new("rep-symlink");
    std::fs::write(target.path(), b"target").expect("write symlink target");
    let listener = ClaudeListener::bind_at(socket.path()).expect("bind listener");

    std::fs::remove_file(socket.path()).expect("unlink the bound socket");
    std::os::unix::fs::symlink(target.path(), socket.path()).expect("symlink replacement");

    drop(listener);
    assert!(
        std::fs::symlink_metadata(socket.path())
            .expect("symlink must still exist")
            .file_type()
            .is_symlink(),
        "drop must never delete a replacement symlink"
    );
    assert_eq!(
        std::fs::read(target.path()).expect("symlink target must still exist"),
        b"target"
    );
}

#[tokio::test]
async fn listener_drop_never_deletes_a_replacement_socket() {
    // The path is rebound by a different listener after our socket is
    // unlinked (the "dashboard restarted while the old listener object still
    // existed" shape): dropping the old listener must leave the new socket's
    // path in place.
    let socket = TempSocket::new("rep-socket");
    let listener = ClaudeListener::bind_at(socket.path()).expect("bind listener");
    assert!(socket.path().exists(), "bind creates the socket file");

    std::fs::remove_file(socket.path()).expect("unlink the bound socket");
    let replacement = UnixListener::bind(socket.path()).expect("bind replacement listener");

    drop(listener);
    let metadata =
        std::fs::symlink_metadata(socket.path()).expect("replacement socket must still exist");
    assert!(
        metadata.file_type().is_socket(),
        "drop must never delete a replacement socket"
    );
    drop(replacement);
}

#[tokio::test]
async fn listener_drops_a_second_frame_after_an_exact_boundary_frame() {
    // The first frame occupies exactly MAX_ENVELOPE_BYTES (newline last) and
    // decodes as a fully valid envelope; a second complete frame follows it.
    // A reader that stops at the bound would accept the first frame and
    // forward it; the fixed reader must read the extra classification byte
    // and drop the whole connection (T04 review: exact-size multi-frame).
    let socket = TempSocket::new("exact-multi");
    let (mut rx, handle) = spawn_listener(socket.path());

    let first = valid_line_of_exact_length("exact-boundary", MAX_ENVELOPE_BYTES);
    assert_eq!(
        first.len(),
        MAX_ENVELOPE_BYTES,
        "first frame fills the bound"
    );
    let second = valid_line("second-frame", "/w");

    let mut stream = UnixStream::connect(socket.path())
        .await
        .expect("connect exact-boundary client");
    let _ = stream.write_all(first.as_bytes()).await;
    // Let the listener consume the exact-boundary frame before the second
    // frame arrives, forcing the read to continue past the filled buffer.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _ = stream.write_all(second.as_bytes()).await;

    // The connection carried two frames and is dropped whole: no envelope —
    // not even the valid exact-boundary first frame — reaches the channel.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .is_err(),
        "an exact-boundary first frame plus a second frame must not forward an envelope"
    );

    // A later valid client still succeeds.
    assert_one_valid_delivery(&socket, &mut rx, "later-valid").await;
    handle.abort();
}

#[tokio::test]
async fn listener_accepts_an_exact_boundary_single_frame_then_eof() {
    // One valid envelope whose wire length is exactly MAX_ENVELOPE_BYTES
    // (newline included), followed by EOF: a single complete frame at the
    // exact bound stays accepted under the contract.
    let socket = TempSocket::new("exact-single");
    let (mut rx, handle) = spawn_listener(socket.path());

    let exact = valid_line_of_exact_length("exact-boundary", MAX_ENVELOPE_BYTES);
    assert_eq!(exact.len(), MAX_ENVELOPE_BYTES);

    let mut stream = UnixStream::connect(socket.path())
        .await
        .expect("connect exact-boundary client");
    let _ = stream.write_all(exact.as_bytes()).await;
    drop(stream); // EOF after exactly one frame.

    let envelope = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("exact-boundary single frame must be accepted")
        .expect("channel closed unexpectedly");
    assert_eq!(envelope.record.session_id, "exact-boundary");
    handle.abort();
}

/// Guards against the "stats padding" failure mode: the subprocess tests above
/// exercise the real binary, but the listener/adapter path must also work
/// end to end through T02 `deliver_to` (the same write the hook performs)
/// into the T04 listener and a live adapter.
#[tokio::test]
async fn feature_t02_delivery_through_listener_to_adapter_event() {
    let socket = TempSocket::new("e2e");
    let (tx, adapter) = ClaudeAdapter::channel();
    let listener = ClaudeListener::bind_at(socket.path()).expect("bind listener");
    let (event_tx, mut event_rx) = unbounded_channel::<SessionEvent>();
    let adapter_handle = Box::new(adapter).run(event_tx);
    let listener_handle = listener.run(tx);

    let record = parsed(
        &serde_json::json!({
            "hook_event_name": "SessionEnd",
            "session_id": "e2e-sess",
            "cwd": env!("CARGO_MANIFEST_DIR"),
            "reason": "other",
        })
        .to_string(),
    );
    let outcome = deliver_to(&record, socket.path()).await;
    assert_eq!(outcome, DeliveryOutcome::Sent);

    let event = tokio::time::timeout(Duration::from_secs(5), event_rx.recv())
        .await
        .expect("adapter event from listener delivery")
        .expect("adapter channel closed unexpectedly");
    assert_eq!(
        event,
        SessionEvent::Gone(dashboard::snapshot::SessionId::new(
            HarnessKind("claude"),
            "e2e-sess"
        ))
    );

    listener_handle.abort();
    adapter_handle
        .await
        .expect("adapter must end cleanly after input closes");
}
