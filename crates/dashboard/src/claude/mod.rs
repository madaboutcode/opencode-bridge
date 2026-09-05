//! The Claude hook adapter — T03 (see `crates/dashboard/src/claude/DESIGN.md`).
//!
//! This module is the second [`HarnessAdapter`] implementation: it turns T02's
//! typed, versioned `ClaudeIpcEnvelope` records (written to a user-scoped Unix
//! socket by the `hook` helper, decoded from the wire by `wire::decode_envelope`)
//! into whole provider-neutral [`SessionEvent`]s. The layered split is the
//! design's Candidate B:
//!
//! - [`hook`] (T02, consumed unchanged) — parsing and best-effort delivery.
//! - [`wire`] — decoding one bounded newline-delimited envelope, strictly.
//! - [`state`] — pure in-memory transitions and snapshot construction.
//! - [`command`] (T04) — the `dashboard claude-hook` helper command.
//! - [`listener`] (T04) — the user-scoped Unix listener for startup.
//! - this module — the `HarnessAdapter` channel loop wiring them together.
//!
//! T04 provides the listener, process dispatch, and dashboard startup wiring;
//! this module never opens a socket. T05 owns authenticated completeness and
//! the final stale-session policy; this adapter records receipt timestamps and
//! removes nothing on its own.
//!
//! CONTRACT: ClaudeAdapter (T03; `docs/specs/dashboard/claude.md` R13-R17;
//! `crates/dashboard/src/claude/DESIGN.md`)
//!
//! GUARANTEES:
//!   - Implements [`HarnessAdapter`] with `HarnessKind("claude")` and exposes
//!     [`ClaudeAdapter::channel`], an unbounded channel on which T04 submits
//!     typed T02 envelopes in receipt order.
//!   - Processes envelopes serially in channel order; a bad decoded record
//!     (a non-version-1 protocol) is a category-only drop that does not stop
//!     the task or emit partial state.
//!   - Stops cleanly when the input channel closes.
//!
//! EXPECTS:
//!   - T04 to decode socket lines with [`decode_envelope`] and to own socket
//!     listening, startup order, and process-level command dispatch.
//!
//! FAILURE BEHAVIOR:
//!   - An unknown-protocol envelope is dropped with a category-only log line
//!     (never the envelope's values); the sink is best-effort — a closed
//!     consumer ends sends, not the task.
//!
//! DOES NOT:
//!   - Own the listener, configuration, persistence, session control, or
//!     authenticated completeness claims, and never reads transcripts or
//!     touches Claude configuration.

pub mod command;
pub mod hook;
pub mod listener;
mod state;
pub mod wire;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use crate::adapter::{HarnessAdapter, SessionEvent};
use crate::project_identity::GitDirResolver;
use crate::snapshot::HarnessKind;

use state::ClaudeState;

/// This adapter's harness tag (`client.md` R1.5/R1.8) — every Claude
/// `SessionId` this module emits is keyed under it.
pub(crate) const KIND: HarnessKind = HarnessKind("claude");

/// The public hook/envelope APIs the runtime needs, re-exported from the
/// `hook` module (`docs/specs/dashboard/claude.md` R13-R16).
pub use hook::{
    deliver, deliver_to, parse_hook_input, serialize_envelope, ClaudeEvent, ClaudeHookRecord,
    ClaudeIpcEnvelope, DeliveryOutcome, DropReason, EnvelopeSerializeError, ParseOutcome,
    ReceivedAt, SessionEndReason, SessionStartSource, ENVELOPE_PROTOCOL_VERSION, MAX_CWD_LEN,
    MAX_ENVELOPE_BYTES, MAX_FIELD_BYTES, MAX_HOOK_INPUT_BYTES, MAX_LABEL_LEN, MAX_SESSION_ID_LEN,
    TRUNCATION_MARKER,
};

/// The public T03 wire decoder T04 calls after reading one socket line.
pub use wire::{decode_envelope, DecodeError};

/// The T04 helper command (`dashboard claude-hook`) and the T04 listener.
pub use command::ClaudeHookCommand;
pub use listener::{ClaudeListener, ListenerError};

/// The Claude adapter. Runs one receive loop over its typed envelope channel
/// and pushes whole-session snapshots / tombstones onto the shared sink.
pub struct ClaudeAdapter {
    input: UnboundedReceiver<ClaudeIpcEnvelope>,
}

impl ClaudeAdapter {
    /// Creates the (sender, adapter) pair T04 wires up: the sender is handed
    /// to the listener loop (or any producer of typed T02 envelopes), the
    /// adapter is handed to the core via [`HarnessAdapter::run`].
    pub fn channel() -> (UnboundedSender<ClaudeIpcEnvelope>, ClaudeAdapter) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (tx, ClaudeAdapter { input: rx })
    }
}

impl HarnessAdapter for ClaudeAdapter {
    fn kind(&self) -> HarnessKind {
        KIND
    }

    /// Starts the Claude adapter task: receives typed envelopes in channel
    /// order and pushes each event they produce onto `sink`. Ends cleanly when
    /// the input channel closes. A non-version-1 envelope (which cannot come
    /// from `wire::decode_envelope`, but could in principle be constructed by
    /// a future caller) is dropped category-only without touching state, so a
    /// bad record can never panic the task or emit a partial snapshot.
    fn run(self: Box<Self>, sink: UnboundedSender<SessionEvent>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut input = self.input;
            let mut state = ClaudeState::new(GitDirResolver);
            while let Some(envelope) = input.recv().await {
                if envelope.protocol_version != ENVELOPE_PROTOCOL_VERSION {
                    log_category("dropped (unknown protocol version)");
                    continue;
                }
                for event in state.process(&envelope) {
                    let _ = sink.send(event);
                }
            }
        })
    }
}

/// Category-only adapter log line (R14: rejected values never appear in logs).
fn log_category(message: &str) {
    eprintln!("[dashboard] claude adapter: {message}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::SessionEvent;
    use crate::claude::hook::ReceivedAt;
    use crate::project_identity::{DirResolver, GitDirResolver};
    use crate::snapshot::{AttentionState, SessionId, Timestamp};
    use std::path::Path;
    use std::time::Duration;

    const R1: u64 = 1_700_000_000_100;
    const R2: u64 = 1_700_000_000_200;
    const R3: u64 = 1_700_000_000_300;

    fn start_envelope(session: &str, cwd: &str, received_at: u64) -> ClaudeIpcEnvelope {
        ClaudeIpcEnvelope {
            protocol_version: ENVELOPE_PROTOCOL_VERSION,
            record: ClaudeHookRecord {
                session_id: session.to_string(),
                cwd: cwd.to_string(),
                event: ClaudeEvent::SessionStart {
                    source: Some(SessionStartSource::Startup),
                    model: None,
                },
                received_at: ReceivedAt(received_at),
            },
        }
    }

    fn end_envelope(session: &str, cwd: &str, received_at: u64) -> ClaudeIpcEnvelope {
        ClaudeIpcEnvelope {
            protocol_version: ENVELOPE_PROTOCOL_VERSION,
            record: ClaudeHookRecord {
                session_id: session.to_string(),
                cwd: cwd.to_string(),
                event: ClaudeEvent::SessionEnd {
                    reason: Some(SessionEndReason::Other),
                },
                received_at: ReceivedAt(received_at),
            },
        }
    }

    /// The real directory used as `cwd` in these tests is this repo's own
    /// dashboard crate directory (exists, inside a git repo), so the real
    /// GitDirResolver yields the canonical repo root.
    fn repo_root() -> std::path::PathBuf {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        GitDirResolver.resolve(dir).expect("resolve manifest dir")
    }

    #[test]
    fn kind_is_claude() {
        let (_tx, adapter) = ClaudeAdapter::channel();
        assert_eq!(adapter.kind(), HarnessKind("claude"));
    }

    #[tokio::test]
    async fn adapter_processes_envelopes_in_channel_order_and_emits_events() {
        let (tx, adapter) = ClaudeAdapter::channel();
        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<SessionEvent>();
        let handle = Box::new(adapter).run(event_tx);

        let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        tx.send(start_envelope("sess-1", &dir.to_string_lossy(), R1))
            .unwrap();
        tx.send(start_envelope("sess-1", &dir.to_string_lossy(), R2))
            .unwrap();
        tx.send(end_envelope("sess-1", &dir.to_string_lossy(), R3))
            .unwrap();
        drop(tx);

        let first = event_rx.recv().await.expect("first event");
        let second = event_rx.recv().await.expect("second event");
        let third = event_rx.recv().await.expect("third event");
        assert_eq!(event_rx.recv().await, None, "adapter emits nothing else");
        handle.await.expect("adapter task must end cleanly");

        let SessionEvent::Snapshot(first_snapshot) = &first else {
            panic!("expected snapshot, got {first:?}");
        };
        assert_eq!(first_snapshot.session_id.harness, KIND);
        assert_eq!(first_snapshot.session_id.native_id, "sess-1");
        assert_eq!(first_snapshot.project_id.as_path(), repo_root());
        assert_eq!(
            first_snapshot.attention,
            AttentionState::Idle {
                last_update: Timestamp::from_epoch_millis(R1 as i64),
            }
        );
        assert_eq!(
            first_snapshot.created_at,
            Timestamp::from_epoch_millis(R1 as i64)
        );
        assert_eq!(
            first_snapshot.last_updated,
            Timestamp::from_epoch_millis(R1 as i64)
        );

        let SessionEvent::Snapshot(second_snapshot) = &second else {
            panic!("expected snapshot, got {second:?}");
        };
        assert_eq!(
            second_snapshot.created_at,
            Timestamp::from_epoch_millis(R1 as i64),
            "duplicate start preserves creation"
        );
        assert_eq!(
            second_snapshot.last_updated,
            Timestamp::from_epoch_millis(R2 as i64)
        );
        assert_eq!(
            second_snapshot.attention,
            AttentionState::Idle {
                last_update: Timestamp::from_epoch_millis(R2 as i64),
            }
        );

        assert_eq!(
            third,
            SessionEvent::Gone(SessionId::new(KIND, "sess-1")),
            "terminal tombstone"
        );
    }

    #[tokio::test]
    async fn adapter_drops_unknown_protocol_records_and_keeps_running() {
        let (tx, adapter) = ClaudeAdapter::channel();
        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<SessionEvent>();
        let handle = Box::new(adapter).run(event_tx);

        // A bad record that cannot come from the decoder but could in
        // principle be constructed by a caller: wrong protocol version.
        let mut bad = start_envelope("sess-bad", "/does/not/matter", R1);
        bad.protocol_version = 99;
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        tx.send(bad).unwrap();
        // The adapter is still alive and processing valid input afterwards.
        tx.send(end_envelope("sess-1", &dir.to_string_lossy(), R3))
            .unwrap();
        drop(tx);

        let gone = event_rx
            .recv()
            .await
            .expect("only the valid record produces an event");
        assert_eq!(gone, SessionEvent::Gone(SessionId::new(KIND, "sess-1")));
        assert_eq!(event_rx.recv().await, None, "bad record emitted no event");
        handle.await.expect("adapter task must end cleanly");
    }

    #[tokio::test]
    async fn adapter_stops_cleanly_when_the_input_channel_closes() {
        let (tx, adapter) = ClaudeAdapter::channel();
        let (event_tx, _event_rx) = tokio::sync::mpsc::unbounded_channel::<SessionEvent>();
        let handle = Box::new(adapter).run(event_tx);
        drop(tx);
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("adapter must stop within the timeout")
            .expect("adapter task panicked");
    }
}
