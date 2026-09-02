//! Live session bookkeeping shared by every frame: the current snapshot per
//! `SessionId`, kept in lockstep with T10's claim map (T12 contract, AC9 —
//! claim wiring both directions, for every live session including
//! subagents).

use std::collections::HashMap;

use crate::adapter::SessionEvent;
use crate::naming::{LiveSession, NamingClaimMap};
use crate::snapshot::{SessionId, SessionSnapshot};

pub struct LiveState {
    sessions: HashMap<SessionId, SessionSnapshot>,
    naming: NamingClaimMap,
}

impl Default for LiveState {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveState {
    /// Deliberately does *not* derive `Default` for this struct: T10's
    /// `NamingClaimMap` itself derives `Default`, but that derived impl
    /// gives `categories: Vec::default()` (empty) rather than the 10
    /// populated categories `NamingClaimMap::new()` builds from
    /// `wordlist::CATEGORIES` — a real, panicking footgun (any claim on an
    /// empty `categories` vec hits `preferred_index`'s own
    /// "category/word lists are never empty" `debug_assert`). Flagging
    /// this for T10 rather than fixing it there (out of this task's
    /// scope); this constructor just always goes through
    /// `NamingClaimMap::new()` instead of relying on its `Default`.
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            naming: NamingClaimMap::new(),
        }
    }

    /// Applies one batch of events drained from the adapter channel in a
    /// single pass (`shell::app`'s main loop calls this once per wake-up
    /// with everything currently queued — see `app::drain`'s doc comment).
    ///
    /// Every session id not already tracked here — top-level or subagent
    /// alike, `client.md` R1.5's identity model draws no distinction
    /// between them, and neither does this function — is claimed together
    /// through one [`NamingClaimMap::claim_batch`] call, so a startup burst
    /// (the adapter's first reconcile sweep, arriving as several individual
    /// `SessionEvent`s back to back on the same channel) still resolves
    /// names deterministically by creation time regardless of arrival order
    /// (`naming::claim_map`'s own "Claim order" doc comment). A
    /// steady-state single new session goes through the same call with a
    /// one-element batch, which is exactly equivalent to
    /// `NamingClaimMap::claim_session`.
    ///
    /// A `Gone` tombstone releases its claim
    /// ([`NamingClaimMap::release_session`]) before this function returns,
    /// so the freed name/category is assignable again as soon as the next
    /// `claim_batch` call — in this same invocation, or a later one —
    /// resolves it (AC9's tombstone-to-release direction).
    pub fn apply_events(&mut self, events: Vec<SessionEvent>) {
        let mut newly_live = Vec::new();
        for event in events {
            match event {
                SessionEvent::Snapshot(snap) => {
                    if !self.sessions.contains_key(&snap.session_id) {
                        newly_live.push(LiveSession {
                            project_id: snap.project_id.clone(),
                            session_id: snap.session_id.clone(),
                            created_at: snap.created_at,
                        });
                    }
                    self.sessions.insert(snap.session_id.clone(), *snap);
                }
                SessionEvent::Gone(id) => {
                    self.naming.release_session(&id);
                    self.sessions.remove(&id);
                }
            }
        }
        if !newly_live.is_empty() {
            self.naming.claim_batch(newly_live);
        }
    }

    pub fn naming(&self) -> &NamingClaimMap {
        &self.naming
    }

    /// A fresh snapshot list for this frame's render call — cloned because
    /// `shell::reclassify` needs an owned, mutable copy to turn stale
    /// sessions into `Idle` (R3) without touching this struct's own stored
    /// copy of `attention`.
    pub fn snapshots(&self) -> Vec<SessionSnapshot> {
        self.sessions.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::{AttentionState, HarnessKind, ProjectId, Timestamp};
    use std::path::PathBuf;

    const KIND: HarnessKind = HarnessKind("test");

    fn snapshot(
        session: &str,
        project: &str,
        parent: Option<&str>,
        created_at_ms: i64,
    ) -> SessionSnapshot {
        SessionSnapshot {
            session_id: SessionId::new(KIND, session),
            project_id: ProjectId::from_canonical(PathBuf::from(project)),
            parent_id: parent.map(|p| SessionId::new(KIND, p)),
            attention: AttentionState::Running {
                turn_started: Timestamp::from_epoch_millis(created_at_ms),
            },
            current_action: None,
            wire_title: None,
            final_assistant_text: None,
            last_user_prompt: None,
            files_touched: vec![],
            recent_actions: vec![],
            created_at: Timestamp::from_epoch_millis(created_at_ms),
            last_updated: Timestamp::from_epoch_millis(created_at_ms),
        }
    }

    // --- AC9(a): a subagent fixture ends up with a real claimed nickname,
    // not the id fallback view.rs falls back to when `nickname_of` is None.

    #[test]
    fn subagent_session_gets_claimed_alongside_its_parent() {
        let mut live = LiveState::new();
        let parent = snapshot("parent-1", "/tmp/proj", None, 0);
        let child = snapshot("child-1", "/tmp/proj", Some("parent-1"), 1);

        live.apply_events(vec![
            SessionEvent::Snapshot(Box::new(parent)),
            SessionEvent::Snapshot(Box::new(child)),
        ]);

        let child_id = SessionId::new(KIND, "child-1");
        assert!(
            live.naming().nickname_of(&child_id).is_some(),
            "a subagent session (parent_id.is_some()) must be claimed by this task's wiring \
             just like a top-level one — nickname_of returning None is exactly the case that \
             makes view.rs fall back to the raw harness id"
        );
    }

    #[test]
    fn a_startup_burst_of_new_sessions_claims_deterministically_regardless_of_event_order() {
        // Same sessions, two different arrival orders on the channel —
        // mirrors naming::claim_map's own "claim order is creation-time,
        // not arrival-order" test, but exercised through this task's own
        // batching wiring instead of calling claim_batch directly.
        let forward = vec![
            SessionEvent::Snapshot(Box::new(snapshot("a1", "/tmp/proj-a", None, 100))),
            SessionEvent::Snapshot(Box::new(snapshot("b1", "/tmp/proj-b", None, 200))),
        ];
        let reversed = vec![
            SessionEvent::Snapshot(Box::new(snapshot("b1", "/tmp/proj-b", None, 200))),
            SessionEvent::Snapshot(Box::new(snapshot("a1", "/tmp/proj-a", None, 100))),
        ];

        let mut live_forward = LiveState::new();
        live_forward.apply_events(forward);
        let mut live_reversed = LiveState::new();
        live_reversed.apply_events(reversed);

        let a1 = SessionId::new(KIND, "a1");
        let b1 = SessionId::new(KIND, "b1");
        assert_eq!(
            live_forward
                .naming()
                .nickname_of(&a1)
                .map(|n| n.display_word()),
            live_reversed
                .naming()
                .nickname_of(&a1)
                .map(|n| n.display_word()),
        );
        assert_eq!(
            live_forward
                .naming()
                .nickname_of(&b1)
                .map(|n| n.display_word()),
            live_reversed
                .naming()
                .nickname_of(&b1)
                .map(|n| n.display_word()),
        );
    }

    // --- AC9(b): a tombstone's slot is observably freed in the claim
    // state, checked through the claim map's own public API (not a code
    // read).

    #[test]
    fn tombstone_release_frees_the_slot_for_a_later_project() {
        let mut live = LiveState::new();
        let only = snapshot("only-session", "/tmp/release-proj", None, 0);
        live.apply_events(vec![SessionEvent::Snapshot(Box::new(only))]);

        let session_id = SessionId::new(KIND, "only-session");
        let project_id = ProjectId::from_canonical(PathBuf::from("/tmp/release-proj"));
        assert!(live.naming().category_of(&project_id).is_some());

        live.apply_events(vec![SessionEvent::Gone(session_id.clone())]);

        assert!(
            live.naming().category_of(&project_id).is_none(),
            "the project's category must be observably released once its only session is gone"
        );
        assert!(live.naming().nickname_of(&session_id).is_none());

        // The freed category is claimable by a brand-new project in the
        // very same call — proves the release actually happened in claim
        // state, not just that the local session map forgot the id.
        let other = snapshot("s2", "/tmp/a-different-project", None, 1);
        live.apply_events(vec![SessionEvent::Snapshot(Box::new(other))]);
        let other_project = ProjectId::from_canonical(PathBuf::from("/tmp/a-different-project"));
        assert!(live.naming().category_of(&other_project).is_some());
    }

    #[test]
    fn an_update_to_an_already_tracked_session_does_not_reclaim_it() {
        let mut live = LiveState::new();
        let first = snapshot("s1", "/tmp/proj", None, 0);
        live.apply_events(vec![SessionEvent::Snapshot(Box::new(first))]);
        let session_id = SessionId::new(KIND, "s1");
        let nick_before = live
            .naming()
            .nickname_of(&session_id)
            .unwrap()
            .display_word();

        let mut updated = snapshot("s1", "/tmp/proj", None, 0);
        updated.current_action = Some("editing: foo.rs".into());
        live.apply_events(vec![SessionEvent::Snapshot(Box::new(updated))]);

        let nick_after = live
            .naming()
            .nickname_of(&session_id)
            .unwrap()
            .display_word();
        assert_eq!(
            nick_before, nick_after,
            "an update must not re-claim a new name"
        );
        assert_eq!(
            live.snapshots()[0].current_action.as_deref(),
            Some("editing: foo.rs"),
            "the stored snapshot must reflect the update"
        );
    }
}
