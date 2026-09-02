//! Snapshot construction and the reconcile sweep's diff logic —
//! `docs/specs/dashboard/client.md` R4 (mechanism) and R1.7 (staleness
//! mechanism: the "gone" tombstone). Deliberately kept synchronous and free
//! of any `Client`/network dependency: the async glue in `mod.rs` fetches
//! data and calls into this file, so the diff/build logic itself (AC6, AC7)
//! is testable with plain fixture data and no mock server.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use opencode_client::{Message, SessionInfo};

use crate::project_identity::ProjectIdentityCache;
use crate::snapshot::{AttentionState, ProjectId, SessionId, SessionSnapshot, Timestamp};

use super::question::looks_like_question;
use super::session_state::TrackedSession;
use super::KIND;

fn latest_user_text(messages: &[Message]) -> Option<String> {
    messages
        .iter()
        .filter(|m| m.kind == "user")
        .max_by_key(|m| m.time.created)
        .map(|m| {
            m.content
                .iter()
                .filter(|p| p.kind == "text")
                .filter_map(|p| p.text.as_deref())
                .collect::<Vec<_>>()
                .join("")
        })
}

/// SPEC.md §5's established running rule (`crates/opencode-bridge/src/
/// tools.rs`, "running = outcome absent OR time.idle older than
/// time.updated"), reused rather than re-derived — it's already the
/// correctness-verified definition this codebase uses for "is a turn in
/// progress," and R4 gives the opencode adapter no reason to define a
/// second one.
pub(crate) fn is_running(info: &SessionInfo) -> bool {
    info.outcome.is_none()
        || info
            .time
            .idle
            .map(|idle| idle < info.time.updated)
            .unwrap_or(true)
}

fn resolve_project_id<R: crate::project_identity::DirResolver>(
    project_cache: &mut ProjectIdentityCache<R>,
    session_id: &SessionId,
    info: &SessionInfo,
) -> ProjectId {
    let dir = info
        .location
        .as_ref()
        .map(|l| l.directory.as_str())
        .unwrap_or("");
    match project_cache.resolve(session_id, Path::new(dir)) {
        Ok(id) => id,
        Err(_e) => {
            // FALLBACK-OK: resolution requires the directory to exist
            // (`std::fs::canonicalize` errors otherwise) and spawns `git`,
            // either of which can fail for a session whose directory
            // vanished or is momentarily unreadable. Crashing the whole
            // adapter — and every other tracked session with it — over one
            // session's bad directory would violate the release bar's
            // "other sessions must render correctly" concern far worse
            // than showing this one session under a degraded, uncanonical
            // project identity until its directory (or the resolver) is
            // healthy again.
            ProjectId::from_uncanonicalized(Path::new(dir))
        }
    }
}

/// Builds one session's snapshot from its current wire info, this adapter's
/// own tracked state, and (when available) its message history. `messages`
/// is `None` whenever the caller didn't fetch them — always the case for a
/// running session (`layout.md`'s running block never shows final
/// assistant text or last user prompt, so there's no reason to pay for the
/// extra `GET /message` call on every sweep of every running session).
pub(crate) fn build_snapshot(
    session_id: &SessionId,
    project_id: &ProjectId,
    info: &SessionInfo,
    tracked: &TrackedSession,
    messages: Option<&[Message]>,
) -> SessionSnapshot {
    let last_updated = Timestamp::from_epoch_millis(info.time.updated);
    let created_at = Timestamp::from_epoch_millis(info.time.created);

    let (final_assistant_text, last_user_prompt) = match messages {
        Some(msgs) => (
            opencode_client::latest_assistant_text(msgs),
            latest_user_text(msgs),
        ),
        None => (None, None),
    };

    let attention = if is_running(info) {
        AttentionState::Running {
            turn_started: tracked.turn_started.unwrap_or(last_updated),
        }
    } else {
        let turn_ended = info
            .time
            .idle
            .map(Timestamp::from_epoch_millis)
            .unwrap_or(last_updated);
        let question = final_assistant_text
            .as_deref()
            .map(looks_like_question)
            .unwrap_or(false);
        AttentionState::NeedsYou {
            question,
            turn_ended,
        }
    };

    SessionSnapshot {
        session_id: session_id.clone(),
        project_id: project_id.clone(),
        parent_id: info
            .parent_id
            .clone()
            .map(|native_id| SessionId::new(session_id.harness, native_id)),
        attention,
        current_action: tracked.current_action.clone(),
        wire_title: info.title.clone(),
        final_assistant_text,
        last_user_prompt,
        files_touched: tracked.files_touched.clone(),
        recent_actions: tracked.recent_actions.iter().cloned().collect(),
        created_at,
        last_updated,
    }
}

/// Upserts one session: resolves its project identity (cached), builds its
/// snapshot from current wire info + tracked state, and returns it. Used
/// both by the full sweep below and by the SSE fast path (`mod.rs`) for a
/// single session — the SSE path must NOT go through `sweep`, which treats
/// "not in this batch" as "gone."
pub(crate) fn upsert_snapshot<R: crate::project_identity::DirResolver>(
    sessions: &mut HashMap<SessionId, TrackedSession>,
    project_cache: &mut ProjectIdentityCache<R>,
    info: &SessionInfo,
    messages: Option<&[Message]>,
) -> SessionSnapshot {
    let session_id = SessionId::new(KIND, info.id.clone());
    let project_id = resolve_project_id(project_cache, &session_id, info);
    let tracked = sessions.entry(session_id.clone()).or_default();
    build_snapshot(&session_id, &project_id, info, tracked, messages)
}

/// One full reconcile sweep (R4): given the complete current session list
/// from `GET /api/session`, upserts a snapshot for each and emits a "gone"
/// tombstone (R1.7's mechanism) for every previously-tracked session that
/// no longer appears. This is the correctness source R4 describes — it
/// doesn't matter whether SSE delivered anything at all since the last
/// sweep; the result here only depends on `current` and prior tracked
/// state, never on SSE-only data.
pub(crate) fn sweep<R: crate::project_identity::DirResolver>(
    sessions: &mut HashMap<SessionId, TrackedSession>,
    project_cache: &mut ProjectIdentityCache<R>,
    current: &[SessionInfo],
    messages: &HashMap<String, Vec<Message>>,
) -> Vec<crate::adapter::SessionEvent> {
    use crate::adapter::SessionEvent;

    let mut events = Vec::with_capacity(current.len());
    let mut seen: HashSet<SessionId> = HashSet::with_capacity(current.len());

    for info in current {
        let session_id = SessionId::new(KIND, info.id.clone());
        seen.insert(session_id.clone());
        let snapshot = upsert_snapshot(
            sessions,
            project_cache,
            info,
            messages.get(&info.id).map(Vec::as_slice),
        );
        events.push(SessionEvent::Snapshot(Box::new(snapshot)));
    }

    let gone: Vec<SessionId> = sessions
        .keys()
        .filter(|id| !seen.contains(*id))
        .cloned()
        .collect();
    for id in gone {
        sessions.remove(&id);
        events.push(SessionEvent::Gone(id));
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::SessionEvent;
    use crate::project_identity::DirResolver;
    use opencode_client::{SessionLocation, SessionTime};
    use std::io;
    use std::path::PathBuf;

    /// A resolver that never touches the filesystem or spawns `git` — every
    /// directory maps to itself. Keeps these tests fixture-only, no
    /// dependency on this repo's own working tree.
    #[derive(Default, Clone, Copy)]
    struct IdentityResolver;
    impl DirResolver for IdentityResolver {
        fn resolve(&self, dir: &Path) -> io::Result<PathBuf> {
            Ok(dir.to_path_buf())
        }
    }

    fn fixture_session(
        id: &str,
        outcome: Option<&str>,
        updated: i64,
        idle: Option<i64>,
    ) -> SessionInfo {
        SessionInfo {
            id: id.to_string(),
            outcome: outcome.map(str::to_string),
            time: SessionTime {
                created: 1_000,
                updated,
                idle,
            },
            cost: 0.0,
            tokens: serde_json::json!({}),
            title: Some(format!("title-{id}")),
            location: Some(SessionLocation {
                directory: "/tmp/project-a".to_string(),
            }),
            project_id: None,
            subpath: None,
            parent_id: None,
        }
    }

    #[test]
    fn running_session_gets_running_attention_state() {
        let info = fixture_session("ses_1", None, 2_000, None);
        let mut sessions = HashMap::new();
        let mut cache = ProjectIdentityCache::new(IdentityResolver);
        let snap = upsert_snapshot(&mut sessions, &mut cache, &info, None);
        assert!(matches!(snap.attention, AttentionState::Running { .. }));
    }

    #[test]
    fn stopped_session_with_question_text_gets_needs_you_question() {
        let info = fixture_session("ses_2", Some("succeeded"), 5_000, Some(5_000));
        let messages = vec![Message {
            kind: "assistant".to_string(),
            time: opencode_client::MessageTime { created: 4_000 },
            content: vec![opencode_client::MessagePart {
                kind: "text".to_string(),
                text: Some("Which file should I delete?".to_string()),
            }],
            error: None,
        }];
        let mut sessions = HashMap::new();
        let mut cache = ProjectIdentityCache::new(IdentityResolver);
        let snap = upsert_snapshot(&mut sessions, &mut cache, &info, Some(&messages));
        match snap.attention {
            AttentionState::NeedsYou {
                question,
                turn_ended,
            } => {
                assert!(question);
                assert_eq!(turn_ended, Timestamp::from_epoch_millis(5_000));
            }
            other => panic!("expected NeedsYou, got {other:?}"),
        }
        assert_eq!(
            snap.final_assistant_text.as_deref(),
            Some("Which file should I delete?")
        );
    }

    #[test]
    fn stopped_session_without_question_text_gets_plain_needs_you() {
        let info = fixture_session("ses_3", Some("succeeded"), 5_000, Some(5_000));
        let messages = vec![Message {
            kind: "assistant".to_string(),
            time: opencode_client::MessageTime { created: 4_000 },
            content: vec![opencode_client::MessagePart {
                kind: "text".to_string(),
                text: Some("Done, all tests pass.".to_string()),
            }],
            error: None,
        }];
        let mut sessions = HashMap::new();
        let mut cache = ProjectIdentityCache::new(IdentityResolver);
        let snap = upsert_snapshot(&mut sessions, &mut cache, &info, Some(&messages));
        match snap.attention {
            AttentionState::NeedsYou { question, .. } => assert!(!question),
            other => panic!("expected NeedsYou, got {other:?}"),
        }
    }

    #[test]
    fn parent_id_carries_through_to_snapshot() {
        let mut info = fixture_session("ses_child", None, 2_000, None);
        info.parent_id = Some("ses_parent".to_string());
        let mut sessions = HashMap::new();
        let mut cache = ProjectIdentityCache::new(IdentityResolver);
        let snap = upsert_snapshot(&mut sessions, &mut cache, &info, None);
        assert_eq!(snap.parent_id.unwrap().native_id, "ses_parent");
    }

    // --- AC6: reconcile sweep alone, no SSE, still produces a correct
    // snapshot on the next pass ---

    #[test]
    fn sweep_alone_reflects_a_session_that_only_ever_arrived_via_sweep() {
        // Simulates a dashboard that just started (or reconnected) with no
        // SSE history at all for this session — `sessions` starts empty,
        // nothing but two consecutive sweeps ever touches it.
        let mut sessions = HashMap::new();
        let mut cache = ProjectIdentityCache::new(IdentityResolver);
        let messages = HashMap::new();

        let first = vec![fixture_session("ses_a", None, 1_000, None)];
        let events1 = sweep(&mut sessions, &mut cache, &first, &messages);
        assert_eq!(events1.len(), 1);
        assert!(
            matches!(&events1[0], SessionEvent::Snapshot(s) if matches!(s.attention, AttentionState::Running { .. }))
        );

        // The session's turn ends between sweeps; SSE never told us —
        // only the next sweep's fresh SessionInfo did.
        let second = vec![fixture_session(
            "ses_a",
            Some("succeeded"),
            3_000,
            Some(3_000),
        )];
        let events2 = sweep(&mut sessions, &mut cache, &second, &messages);
        assert_eq!(events2.len(), 1);
        match &events2[0] {
            SessionEvent::Snapshot(s) => {
                assert!(matches!(s.attention, AttentionState::NeedsYou { .. }));
                assert_eq!(s.last_updated, Timestamp::from_epoch_millis(3_000));
            }
            other => panic!("expected a snapshot, got {other:?}"),
        }
    }

    // --- AC7: tombstone fires when a previously-known session vanishes ---

    #[test]
    fn sweep_emits_gone_tombstone_for_a_vanished_session() {
        let mut sessions = HashMap::new();
        let mut cache = ProjectIdentityCache::new(IdentityResolver);
        let messages = HashMap::new();

        let first = vec![
            fixture_session("ses_a", None, 1_000, None),
            fixture_session("ses_b", None, 1_000, None),
        ];
        let events1 = sweep(&mut sessions, &mut cache, &first, &messages);
        assert_eq!(events1.len(), 2);
        assert_eq!(sessions.len(), 2);

        // ses_b no longer appears in GET /api/session.
        let second = vec![fixture_session("ses_a", None, 2_000, None)];
        let events2 = sweep(&mut sessions, &mut cache, &second, &messages);

        let gone_ids: Vec<&SessionId> = events2
            .iter()
            .filter_map(|e| match e {
                SessionEvent::Gone(id) => Some(id),
                _ => None,
            })
            .collect();
        assert_eq!(gone_ids.len(), 1);
        assert_eq!(gone_ids[0].native_id, "ses_b");

        let snapshot_ids: Vec<&SessionId> = events2
            .iter()
            .filter_map(|e| match e {
                SessionEvent::Snapshot(s) => Some(&s.session_id),
                _ => None,
            })
            .collect();
        assert_eq!(snapshot_ids.len(), 1);
        assert_eq!(snapshot_ids[0].native_id, "ses_a");

        // The tombstoned session is dropped from tracked state — it can't
        // leak its claim forever (visuals.md R6.8's release-path concern).
        assert_eq!(sessions.len(), 1);
        assert!(!sessions.contains_key(gone_ids[0]));
    }

    #[test]
    fn sweep_does_not_tombstone_a_session_still_present() {
        let mut sessions = HashMap::new();
        let mut cache = ProjectIdentityCache::new(IdentityResolver);
        let messages = HashMap::new();

        let first = vec![fixture_session("ses_a", None, 1_000, None)];
        sweep(&mut sessions, &mut cache, &first, &messages);

        let second = vec![fixture_session("ses_a", None, 2_000, None)];
        let events2 = sweep(&mut sessions, &mut cache, &second, &messages);

        assert!(events2
            .iter()
            .all(|e| matches!(e, SessionEvent::Snapshot(_))));
    }

    #[test]
    fn upsert_snapshot_never_tombstones_other_sessions() {
        // The SSE fast path calls upsert_snapshot for one session at a
        // time; it must never behave like a sweep that treats every other
        // tracked session as gone.
        let mut sessions = HashMap::new();
        let mut cache = ProjectIdentityCache::new(IdentityResolver);
        let messages = HashMap::new();
        let first = vec![
            fixture_session("ses_a", None, 1_000, None),
            fixture_session("ses_b", None, 1_000, None),
        ];
        sweep(&mut sessions, &mut cache, &first, &messages);
        assert_eq!(sessions.len(), 2);

        let updated_a = fixture_session("ses_a", None, 2_000, None);
        upsert_snapshot(&mut sessions, &mut cache, &updated_a, None);

        assert_eq!(sessions.len(), 2, "ses_b must still be tracked");
    }
}
