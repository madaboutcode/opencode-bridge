//! The opencode adapter — the one `HarnessAdapter` implementation V1 ships
//! (`docs/specs/dashboard/client.md` R4). Everything under this module is
//! opencode wire knowledge: REST/SSE mechanics, the tool-call correlation
//! map, action-line rendering, the question heuristic. None of it is
//! visible outside this module except through `SessionEvent`/
//! `SessionSnapshot` — the same boundary any other harness's adapter would
//! cross (`adapter.rs`, `snapshot.rs`).
//!
//! Mechanism (R4): `GET /api/session` (paginated — see the "no pagination
//! loop yet" note on [`OpencodeAdapter::run`] below) for the full list,
//! `GET /api/event` SSE for latency, and a fixed 60-second reconcile sweep
//! as the correctness source, run independently of SSE health. The SSE
//! task's only job is to notice a state-changing event sooner than the
//! next sweep would and trigger an immediate single-session refresh
//! (`refresh_session`) — it never maintains its own mirror of a session's
//! wire metadata; every snapshot this adapter emits is built from a fresh
//! `GET /session/{id}` (or the bulk list), never from SSE fields alone.
//! That keeps "how do we turn a `SessionInfo` + our own tracked state into
//! a `SessionSnapshot`" in exactly one place — `reconcile::build_snapshot`
//! — used by both the sweep and the SSE fast path.

mod action_line;
mod reconcile;
mod session_state;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;

use opencode_client::sse::{Event, EventStream};
use opencode_client::Client;

use crate::adapter::{HarnessAdapter, SessionEvent};
use crate::project_identity::ProjectIdentityCache;
use crate::snapshot::{HarnessKind, SessionId, Timestamp};

use reconcile::{is_running, sweep, upsert_snapshot};
use session_state::TrackedSession;

/// This adapter's harness tag (`client.md` R1.5/R1.8).
pub(crate) const KIND: HarnessKind = HarnessKind("opencode");

/// R4's periodic reconcile sweep interval — fixed, independent of SSE
/// connection state (this is the whole point of the sweep: a dropped SSE
/// connection self-heals here regardless of how it dropped).
const RECONCILE_INTERVAL: Duration = Duration::from_secs(60);

/// A half-open TCP connection produces no bytes forever; force a reconnect
/// if nothing (data or keepalive) arrives in this long. Matches
/// `opencode-bridge`'s own `SSE_READ_TIMEOUT` (`crates/opencode-bridge/src/
/// sse.rs`) — same server, same reasoning, independently chosen here since
/// this adapter must not depend on that crate.
const SSE_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Default)]
struct AdapterState {
    sessions: HashMap<SessionId, TrackedSession>,
    project_cache: ProjectIdentityCache,
}

pub struct OpencodeAdapter {
    client: Arc<Client>,
}

impl OpencodeAdapter {
    pub fn new(client: Client) -> Self {
        Self {
            client: Arc::new(client),
        }
    }
}

impl HarnessAdapter for OpencodeAdapter {
    fn kind(&self) -> HarnessKind {
        KIND
    }

    /// Starts the reconcile sweep and the SSE listener as two independent
    /// tasks under one supervisor task, and returns the supervisor's
    /// handle. Neither task depends on the other's health — an SSE
    /// disconnect never stops the sweep, and a sweep failure (e.g. a
    /// transient 500) never stops SSE from continuing to feed fast-path
    /// refreshes (R4's scenario).
    ///
    /// Known scope limit, not required by any T09 acceptance criterion:
    /// `GET /api/session` pagination (R4 says "paginated") isn't looped —
    /// `opencode_client::Client::list_sessions` requests one page, now
    /// raised to `limit=500` (was the server's silent default of 50,
    /// which caused a real reported bug: a project's own sessions falling
    /// entirely off the list whenever 50+ sessions elsewhere were touched
    /// more recently — see that method's doc comment). 500 comfortably
    /// covers real observed workspace scale; true cursor pagination past
    /// that is the next step if ever needed, not implemented here.
    fn run(self: Box<Self>, sink: UnboundedSender<SessionEvent>) -> JoinHandle<()> {
        let client = self.client;
        tokio::spawn(async move {
            let state = Arc::new(Mutex::new(AdapterState::default()));

            let sweep_handle = tokio::spawn(run_reconcile_sweep(
                client.clone(),
                state.clone(),
                sink.clone(),
            ));
            let sse_handle = tokio::spawn(run_sse_loop(client, state, sink));

            let _ = tokio::join!(sweep_handle, sse_handle);
        })
    }
}

async fn run_reconcile_sweep(
    client: Arc<Client>,
    state: Arc<Mutex<AdapterState>>,
    sink: UnboundedSender<SessionEvent>,
) {
    loop {
        match client.list_sessions().await {
            Ok(sessions) => {
                // Only fetch message history for sessions whose turn has
                // ended — a running session's card never shows final
                // assistant text or the last user prompt (`layout.md`
                // R5.3's running block), so there's no reason to pay for
                // the extra `GET /message` call on every sweep of every
                // running session.
                let mut messages = HashMap::new();
                for info in &sessions {
                    if !is_running(info) {
                        if let Ok(msgs) = client.list_messages(&info.id).await {
                            messages.insert(info.id.clone(), msgs);
                        }
                    }
                }

                let events = {
                    let mut guard = state.lock().expect("adapter state mutex poisoned");
                    let AdapterState {
                        sessions: tracked,
                        project_cache,
                    } = &mut *guard;
                    sweep(tracked, project_cache, &sessions, &messages)
                };
                for event in events {
                    let _ = sink.send(event);
                }
            }
            Err(e) => {
                eprintln!("[dashboard] opencode adapter: reconcile sweep failed: {e}");
            }
        }
        tokio::time::sleep(RECONCILE_INTERVAL).await;
    }
}

async fn run_sse_loop(
    client: Arc<Client>,
    state: Arc<Mutex<AdapterState>>,
    sink: UnboundedSender<SessionEvent>,
) {
    let mut backoff = Duration::from_millis(500);
    loop {
        match client.events().await {
            Ok(resp) => {
                backoff = Duration::from_millis(500);
                let mut stream = EventStream::new(resp);
                loop {
                    match stream.next(SSE_IDLE_TIMEOUT).await {
                        Ok(Some(Ok(event))) => {
                            let Some(session_id) = event.session_id.clone() else {
                                continue;
                            };
                            let refresh = {
                                let mut guard = state.lock().expect("adapter state mutex poisoned");
                                let tracked = guard
                                    .sessions
                                    .entry(SessionId::new(KIND, session_id.clone()))
                                    .or_default();
                                handle_sse_event(&event, tracked)
                            };
                            if refresh {
                                refresh_session(&client, &state, &session_id, &sink).await;
                            }
                        }
                        // A frame that isn't valid JSON, or the idle-read
                        // timeout: log-worthy but not fatal to the loop —
                        // R4's reconcile sweep is the correctness backstop
                        // regardless.
                        Ok(Some(Err(_parse_err))) => continue,
                        Ok(None) | Err(_) => break,
                    }
                }
            }
            Err(e) => {
                eprintln!("[dashboard] opencode adapter: SSE connect failed: {e}");
            }
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(30));
    }
}

/// Mutates `tracked` from one SSE frame and reports whether the change is
/// worth an immediate single-session REST refresh (rather than waiting for
/// the next sweep). Only opencode-specific bookkeeping happens here — no
/// `SessionSnapshot` is built in this function.
fn handle_sse_event(event: &Event, tracked: &mut TrackedSession) -> bool {
    match event.kind.as_str() {
        "session.tool.input.started" => {
            let (Some(id), Some(name)) = (
                pointer_str(&event.data, "/data/id"),
                pointer_str(&event.data, "/data/name"),
            ) else {
                return false;
            };
            tracked.call_tracker.on_input_started(id, name);
            false
        }
        "session.tool.called" => {
            let Some(id) = pointer_str(&event.data, "/data/id") else {
                return false;
            };
            let input = event
                .data
                .pointer("/data/input")
                .cloned()
                .unwrap_or(Value::Null);
            let Some((name, line)) = tracked.call_tracker.on_tool_called(id, &input) else {
                return false;
            };
            let path = input.get("path").and_then(Value::as_str);
            tracked.record_action(&name, path, line);
            true
        }
        "session.execution.started" => {
            tracked.start_turn(Timestamp::now());
            true
        }
        "session.execution.succeeded"
        | "session.execution.failed"
        | "session.execution.interrupted" => true,
        _ => false,
    }
}

fn pointer_str<'a>(data: &'a Value, pointer: &str) -> Option<&'a str> {
    data.pointer(pointer).and_then(Value::as_str)
}

/// Refreshes exactly one session: re-fetches its `SessionInfo` (and, if its
/// turn has ended, its messages) via REST and emits a fresh snapshot built
/// from that plus the SSE-updated tracked state. Deliberately calls
/// `upsert_snapshot`, not `sweep` — a single-session refresh must not treat
/// every other tracked session as gone (`reconcile.rs`'s
/// `upsert_snapshot_never_tombstones_other_sessions` test covers exactly
/// this).
async fn refresh_session(
    client: &Client,
    state: &Mutex<AdapterState>,
    native_id: &str,
    sink: &UnboundedSender<SessionEvent>,
) {
    let info = match client.get_session(native_id).await {
        Ok(info) => info,
        // Transient failure (e.g. the session vanished between the SSE
        // event and this fetch): not fatal — the next reconcile sweep
        // either re-confirms it or emits its "gone" tombstone.
        Err(_e) => return,
    };
    let messages = if is_running(&info) {
        None
    } else {
        client.list_messages(native_id).await.ok()
    };

    let snapshot = {
        let mut guard = state.lock().expect("adapter state mutex poisoned");
        let AdapterState {
            sessions: tracked,
            project_cache,
        } = &mut *guard;
        upsert_snapshot(tracked, project_cache, &info, messages.as_deref())
    };
    let _ = sink.send(SessionEvent::Snapshot(Box::new(snapshot)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(kind: &str, data: Value) -> Event {
        Event {
            kind: kind.to_string(),
            session_id: data
                .get("sessionID")
                .and_then(Value::as_str)
                .map(str::to_string),
            seq: None,
            data: json!({ "type": kind, "data": data }),
        }
    }

    #[test]
    fn input_started_then_called_produces_refresh_and_records_action() {
        let mut tracked = TrackedSession::default();
        let started = event(
            "session.tool.input.started",
            json!({"id": "call_1", "name": "edit", "sessionID": "ses_1"}),
        );
        assert!(!handle_sse_event(&started, &mut tracked));
        assert!(tracked.current_action.is_none());

        let called = event(
            "session.tool.called",
            json!({"id": "call_1", "input": {"path": "src/foo.rs"}, "sessionID": "ses_1"}),
        );
        assert!(handle_sse_event(&called, &mut tracked));
        assert_eq!(tracked.current_action.as_deref(), Some("editing: foo.rs"));
        assert_eq!(tracked.files_touched, vec!["src/foo.rs".to_string()]);
    }

    #[test]
    fn tool_called_without_matching_started_does_not_refresh() {
        let mut tracked = TrackedSession::default();
        let called = event(
            "session.tool.called",
            json!({"id": "call_unseen", "input": {}, "sessionID": "ses_1"}),
        );
        assert!(!handle_sse_event(&called, &mut tracked));
        assert!(tracked.current_action.is_none());
    }

    #[test]
    fn execution_started_resets_files_touched_and_requests_refresh() {
        let mut tracked = TrackedSession::default();
        tracked.files_touched.push("stale.rs".to_string());
        let started = event("session.execution.started", json!({"sessionID": "ses_1"}));
        assert!(handle_sse_event(&started, &mut tracked));
        assert!(tracked.files_touched.is_empty());
        assert!(tracked.turn_started.is_some());
    }

    #[test]
    fn execution_terminal_events_request_refresh() {
        for kind in [
            "session.execution.succeeded",
            "session.execution.failed",
            "session.execution.interrupted",
        ] {
            let mut tracked = TrackedSession::default();
            let ev = event(kind, json!({"sessionID": "ses_1"}));
            assert!(
                handle_sse_event(&ev, &mut tracked),
                "{kind} should trigger a refresh"
            );
        }
    }

    #[test]
    fn unrelated_event_kinds_do_not_refresh() {
        let mut tracked = TrackedSession::default();
        let ev = event(
            "session.usage.updated",
            json!({"sessionID": "ses_1", "cost": 0.01}),
        );
        assert!(!handle_sse_event(&ev, &mut tracked));
    }
}
