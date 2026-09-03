//! Pure in-memory Claude session transitions and snapshot construction — T03
//! (see `crates/dashboard/src/claude/DESIGN.md`). Synchronous and free of any
//! socket or channel work on purpose: the adapter loop in [`super::ClaudeAdapter`]
//! owns the async receive/send glue, while this module holds the only per-session
//! state the Claude adapter keeps — a tracked session's project identity and
//! creation time — and turns each validated T02 envelope into a complete
//! provider-neutral [`SessionSnapshot`] or a `Gone` tombstone.
//!
//! T01c supports exactly three observed events, mapped here per the lifecycle
//! contract: `SessionStart` and `StopFailure` each yield one complete
//! `NeedsYou { question: false }` snapshot; `SessionEnd` removes the session
//! and yields one `Gone`. All snapshot content fields T01c does not verify
//! stay `None`/empty. Attention is *derived* per snapshot (it is constant
//! `NeedsYou { question: false }` for the two snapshot events), never stored.
//!
//! CONTRACT: ClaudeLifecycleState (T03; `docs/specs/dashboard/claude.md`
//! R13-R14; `crates/dashboard/src/claude/DESIGN.md`)
//!
//! GUARANTEES:
//!   - Every session identity is `HarnessKind("claude")`; `SessionStart`/
//!     `StopFailure` emit exactly one complete metadata-only
//!     `NeedsYou { question: false }` snapshot each; `SessionEnd` removes the
//!     session and emits exactly one `Gone` — including for a native id this
//!     state has never seen.
//!   - `created_at` is the first accepted event's local receipt time and
//!     duplicate starts preserve it; `last_updated` and the `NeedsYou`
//!     `turn_ended` basis are every event's own local receipt time.
//!   - Project identity resolves through the shared
//!     [`ProjectIdentityCache`]; an unresolvable cwd degrades to the documented
//!     uncanonicalized identity fallback and processing continues.
//!
//! EXPECTS:
//!   - A validated T02 envelope (protocol version 1, in receipt order). The
//!     adapter loop guarantees this before calling [`ClaudeState::process`].
//!
//! FAILURE BEHAVIOR:
//!   - A cwd that cannot be resolved degrades one snapshot to the documented
//!     uncanonicalized project identity (FALLBACK-OK, see
//!     `opencode/reconcile.rs`'s identical call site) and never stops the
//!     adapter or other sessions.
//!
//! DOES NOT:
//!   - Read transcripts, infer unverified lifecycle events, log or retain raw
//!     wire values, or implement expiry/removal — final staleness policy is
//!     T05's (see `claude.md` R17 and the design's assumptions).

use std::collections::HashMap;
use std::path::Path;

use crate::adapter::SessionEvent;
use crate::project_identity::{DirResolver, GitDirResolver, ProjectIdentityCache};
use crate::snapshot::{AttentionState, ProjectId, SessionId, SessionSnapshot, Timestamp};

use super::hook::{ClaudeEvent, ClaudeIpcEnvelope};
use super::KIND;

/// What the adapter remembers about one live Claude session. Deliberately the
/// minimum needed to build the next whole snapshot: identity, project, and
/// creation time. `attention` and `last_updated` are derived per event (the
/// attention state is constant for the two snapshot events and `last_updated`
/// is always the incoming receipt time), so storing them would be dead state.
struct ClaudeTrackedSession {
    project_id: ProjectId,
    created_at: Timestamp,
}

/// The adapter's in-memory Claude session state. Generic over
/// [`DirResolver`] so unit tests stay fixture-only and never spawn `git`;
/// production uses the default [`GitDirResolver`].
pub(crate) struct ClaudeState<R: DirResolver = GitDirResolver> {
    sessions: HashMap<SessionId, ClaudeTrackedSession>,
    project_cache: ProjectIdentityCache<R>,
}

impl<R: DirResolver> ClaudeState<R> {
    pub(crate) fn new(resolver: R) -> Self {
        Self {
            sessions: HashMap::new(),
            project_cache: ProjectIdentityCache::new(resolver),
        }
    }

    /// Processes one validated T02 envelope in receipt order and returns the
    /// provider-neutral events it produces — a complete snapshot for
    /// `SessionStart`/`StopFailure`, or a `Gone` tombstone for `SessionEnd`
    /// (always, even for a native id that was never tracked).
    pub(crate) fn process(&mut self, envelope: &ClaudeIpcEnvelope) -> Vec<SessionEvent> {
        let session_id = SessionId::new(KIND, envelope.record.session_id.clone());
        let receipt =
            Timestamp::from_epoch_millis(envelope.record.received_at.epoch_millis() as i64);

        match &envelope.record.event {
            ClaudeEvent::SessionStart { .. } | ClaudeEvent::StopFailure => {
                let tracked = match self.sessions.get(&session_id) {
                    Some(tracked) => tracked,
                    None => {
                        let project_id = resolve_project_id(
                            &mut self.project_cache,
                            &session_id,
                            &envelope.record.cwd,
                        );
                        self.sessions.insert(
                            session_id.clone(),
                            ClaudeTrackedSession {
                                project_id,
                                created_at: receipt,
                            },
                        );
                        self.sessions.get(&session_id).expect("just inserted")
                    }
                };
                let snapshot = build_snapshot(
                    &session_id,
                    &tracked.project_id,
                    tracked.created_at,
                    receipt,
                );
                vec![SessionEvent::Snapshot(Box::new(snapshot))]
            }
            ClaudeEvent::SessionEnd { .. } => {
                self.sessions.remove(&session_id);
                vec![SessionEvent::Gone(session_id)]
            }
        }
    }
}

/// Resolves an envelope `cwd` to its canonical project identity through the
/// shared per-session cache. On failure, degrades to the documented
/// uncanonicalized identity so one bad directory never stops the adapter.
fn resolve_project_id<R: DirResolver>(
    project_cache: &mut ProjectIdentityCache<R>,
    session_id: &SessionId,
    cwd: &str,
) -> ProjectId {
    match project_cache.resolve(session_id, Path::new(cwd)) {
        Ok(id) => id,
        Err(_e) => {
            // FALLBACK-OK: identical to the opencode adapter's documented
            // degraded path (`opencode/reconcile.rs::resolve_project_id`) and
            // the T03 design's Failure Domains. Canonicalization requires the
            // directory to exist and resolves via `git`; either can fail for a
            // session whose directory vanished or is momentarily unreadable.
            // Crashing the whole adapter — and every other tracked session with
            // it — over one session's bad cwd would violate the release bar's
            // "other sessions must render correctly" concern far worse than
            // showing this one session under a degraded, uncanonicalized
            // project identity until its directory is healthy again.
            ProjectId::from_uncanonicalized(Path::new(cwd))
        }
    }
}

/// Builds the complete provider-neutral snapshot for a Claude session. Every
/// content field T01c does not verify is `None`/empty (`claude.md` R13-R14:
/// only identity, project, and lifecycle metadata cross the boundary).
fn build_snapshot(
    session_id: &SessionId,
    project_id: &ProjectId,
    created_at: Timestamp,
    last_updated: Timestamp,
) -> SessionSnapshot {
    SessionSnapshot {
        session_id: session_id.clone(),
        project_id: project_id.clone(),
        parent_id: None,
        attention: AttentionState::NeedsYou {
            question: false,
            // No wire signal for turn end exists beyond the receipt time
            // (`claude.md` R14: only local receipt time crosses), so the
            // needs-you elapsed basis is the event's own receipt timestamp.
            turn_ended: last_updated,
        },
        current_action: None,
        wire_title: None,
        final_assistant_text: None,
        last_user_prompt: None,
        files_touched: Vec::new(),
        recent_actions: Vec::new(),
        created_at,
        last_updated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::hook::{
        ClaudeHookRecord, ReceivedAt, SessionEndReason, SessionStartSource,
        ENVELOPE_PROTOCOL_VERSION,
    };
    use std::io;
    use std::path::PathBuf;

    /// A resolver that never touches the filesystem or spawns `git`: every
    /// directory maps to itself. Keeps the transition tests fixture-only.
    #[derive(Debug, Default, Clone, Copy)]
    struct IdentityResolver;
    impl DirResolver for IdentityResolver {
        fn resolve(&self, dir: &Path) -> io::Result<PathBuf> {
            Ok(dir.to_path_buf())
        }
    }

    /// A resolver that fails for exactly the directory marked in `failing`.
    #[derive(Debug, Default, Clone)]
    struct SelectiveResolver {
        failing: PathBuf,
    }
    impl DirResolver for SelectiveResolver {
        fn resolve(&self, dir: &Path) -> io::Result<PathBuf> {
            if dir == self.failing {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "fixture dir missing",
                ))
            } else {
                Ok(dir.to_path_buf())
            }
        }
    }

    const R1: u64 = 1_700_000_000_100;
    const R2: u64 = 1_700_000_000_200;
    const R3: u64 = 1_700_000_000_300;

    fn envelope(
        event: ClaudeEvent,
        session: &str,
        cwd: &str,
        received_at: u64,
    ) -> ClaudeIpcEnvelope {
        ClaudeIpcEnvelope {
            protocol_version: ENVELOPE_PROTOCOL_VERSION,
            record: ClaudeHookRecord {
                session_id: session.to_string(),
                cwd: cwd.to_string(),
                event,
                received_at: ReceivedAt(received_at),
            },
        }
    }

    fn start(session: &str, cwd: &str, received_at: u64) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::SessionStart {
                source: Some(SessionStartSource::Startup),
            },
            session,
            cwd,
            received_at,
        )
    }

    fn stop(session: &str, cwd: &str, received_at: u64) -> ClaudeIpcEnvelope {
        envelope(ClaudeEvent::StopFailure, session, cwd, received_at)
    }

    fn end(session: &str, cwd: &str, received_at: u64) -> ClaudeIpcEnvelope {
        envelope(
            ClaudeEvent::SessionEnd {
                reason: Some(SessionEndReason::Other),
            },
            session,
            cwd,
            received_at,
        )
    }

    fn ts(millis: u64) -> Timestamp {
        Timestamp::from_epoch_millis(millis as i64)
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

    fn single_snapshot(events: Vec<SessionEvent>) -> SessionSnapshot {
        match events.as_slice() {
            [SessionEvent::Snapshot(snapshot)] => (**snapshot).clone(),
            other => panic!("expected one snapshot, got {other:?}"),
        }
    }

    #[test]
    fn session_start_admits_one_complete_needs_you_snapshot() {
        let mut state = ClaudeState::new(IdentityResolver);
        let snapshot = single_snapshot(state.process(&start("sess-1", "/work/proj", R1)));

        assert_eq!(snapshot.session_id.harness, KIND);
        assert_eq!(snapshot.session_id.native_id, "sess-1");
        assert_eq!(snapshot.project_id.as_path(), Path::new("/work/proj"));
        assert_eq!(
            snapshot.attention,
            AttentionState::NeedsYou {
                question: false,
                turn_ended: ts(R1),
            }
        );
        assert_eq!(snapshot.created_at, ts(R1));
        assert_eq!(snapshot.last_updated, ts(R1));
        assert_metadata_only(&snapshot);
    }

    #[test]
    fn stop_failure_as_first_event_admits_with_its_receipt_time() {
        let mut state = ClaudeState::new(IdentityResolver);
        let snapshot = single_snapshot(state.process(&stop("sess-1", "/work/proj", R2)));

        assert_eq!(snapshot.session_id.native_id, "sess-1");
        assert_eq!(snapshot.created_at, ts(R2), "first event pins created_at");
        assert_eq!(snapshot.last_updated, ts(R2));
        assert_eq!(
            snapshot.attention,
            AttentionState::NeedsYou {
                question: false,
                turn_ended: ts(R2)
            }
        );
        assert_metadata_only(&snapshot);

        // The session is now tracked: a later SessionEnd tombstones it.
        let events = state.process(&end("sess-1", "/work/proj", R3));
        assert!(matches!(events.as_slice(), [SessionEvent::Gone(id)] if id.native_id == "sess-1"));
    }

    #[test]
    fn duplicate_start_preserves_creation_time_and_refreshes_last_updated() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("sess-1", "/work/proj", R1));
        let second = single_snapshot(state.process(&start("sess-1", "/work/proj", R2)));

        assert_eq!(
            second.created_at,
            ts(R1),
            "duplicate start keeps first receipt"
        );
        assert_eq!(second.last_updated, ts(R2));
        assert_eq!(
            second.attention,
            AttentionState::NeedsYou {
                question: false,
                turn_ended: ts(R2)
            }
        );
        assert_metadata_only(&second);
    }

    #[test]
    fn stop_failure_updates_an_existing_session_to_an_unchanged_needs_you() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("sess-1", "/work/proj", R1));
        let updated = single_snapshot(state.process(&stop("sess-1", "/work/proj", R2)));

        assert_eq!(updated.created_at, ts(R1));
        assert_eq!(updated.last_updated, ts(R2));
        assert_eq!(
            updated.attention,
            AttentionState::NeedsYou {
                question: false,
                turn_ended: ts(R2)
            }
        );
    }

    #[test]
    fn session_end_removes_state_and_emits_gone() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("sess-1", "/work/proj", R1));
        assert_eq!(state.sessions.len(), 1);

        let events = state.process(&end("sess-1", "/work/proj", R3));
        assert!(matches!(events.as_slice(), [SessionEvent::Gone(id)] if id.native_id == "sess-1"));
        assert_eq!(state.sessions.len(), 0, "tombstoned session is removed");

        // A later start for the same native id is a fresh admission again.
        let fresh = single_snapshot(state.process(&start("sess-1", "/work/proj", R3)));
        assert_eq!(
            fresh.created_at,
            ts(R3),
            "re-admitted session restarts its clock"
        );
    }

    #[test]
    fn session_end_for_a_never_seen_native_id_still_emits_gone() {
        let mut state = ClaudeState::new(IdentityResolver);
        let events = state.process(&end("never-seen", "/work/x", R3));
        assert!(
            matches!(events.as_slice(), [SessionEvent::Gone(id)] if id.native_id == "never-seen")
        );
        assert_eq!(state.sessions.len(), 0);
    }

    #[test]
    fn project_resolution_failure_degrades_one_snapshot_and_continues() {
        let missing = PathBuf::from("/nowhere/project");
        let mut state = ClaudeState::new(SelectiveResolver {
            failing: missing.clone(),
        });

        // The failing cwd degrades to the documented uncanonicalized identity
        // rather than dropping the event or stopping the adapter.
        let degraded = single_snapshot(state.process(&start("bad-cwd", "/nowhere/project", R1)));
        assert_eq!(
            degraded.project_id,
            ProjectId::from_uncanonicalized(&missing),
            "degraded project identity is the raw cwd, uncanonicalized"
        );
        assert_eq!(degraded.session_id.native_id, "bad-cwd");

        // The same state instance still processes later, resolvable input.
        let ok = single_snapshot(state.process(&start("good-cwd", "/work/proj", R2)));
        assert_eq!(ok.project_id.as_path(), Path::new("/work/proj"));
        assert_eq!(ok.session_id.native_id, "good-cwd");
    }

    #[test]
    fn sessions_are_keyed_under_the_claude_harness_kind() {
        let mut state = ClaudeState::new(IdentityResolver);
        state.process(&start("123", "/work/proj", R1));
        let tracked: Vec<&SessionId> = state.sessions.keys().collect();
        assert_eq!(tracked.len(), 1);
        assert_eq!(tracked[0].harness, KIND);
        assert_eq!(tracked[0].native_id, "123");
        // The same raw native id under another harness would be a distinct key
        // — this map can never collide with an opencode session.
        assert_ne!(
            tracked[0].clone(),
            SessionId::new(crate::snapshot::HarnessKind("opencode"), "123")
        );
    }

    #[test]
    fn receipt_timestamps_are_used_verbatim_for_created_and_updated() {
        let mut state = ClaudeState::new(IdentityResolver);
        let snapshot = single_snapshot(state.process(&start("sess-1", "/work/proj", R1)));
        assert_eq!(snapshot.created_at, ts(R1));
        assert_eq!(snapshot.last_updated, ts(R1));
        assert_eq!(
            snapshot.attention,
            AttentionState::NeedsYou {
                question: false,
                turn_ended: ts(R1)
            }
        );
    }
}
