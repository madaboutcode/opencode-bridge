//! Builds the render-time view model the ladder/layout/render modules
//! consume, from T09's real [`SessionSnapshot`]s and T10's real
//! [`NamingClaimMap`] — this is the module that plays the role the spike's
//! `fixture.rs` played, except its input is real adapter/naming data
//! instead of hand-written fixtures (T11 contract: "replace the spike's
//! fixture data source with T09's `Session` type and T10's naming output").
//!
//! Nothing here knows about any harness's wire shapes — it reads only
//! `crate::snapshot`'s public types and `crate::naming`'s public output
//! (T11 contract, acceptance criterion 6).

use std::collections::HashMap;

use crate::naming::NamingClaimMap;
use crate::snapshot::{AttentionState, ProjectId, SessionId, SessionSnapshot, Timestamp};

/// The four states a session card renders as. `Question` is `NeedsYou`'s
/// sub-state, split out here because `layout.md`'s content-regime table and
/// R5.6's priority order treat it as functionally distinct (`visuals.md`
/// R6.7).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Question,
    NeedsYou,
    Running,
    Idle,
}

#[derive(Clone, Debug)]
pub struct SubagentView {
    pub nick: String,
    pub action: String,
    /// Same 4-state model as a top-level `SessionView` — a subagent is
    /// itself a full session (`client.md` R1.5) and carries its own
    /// `AttentionState`. Rendered as a per-line glyph/color in
    /// `ladder.rs::subagent_line`, and used here to priority-sort the
    /// list (`build_projects`'s subagent-mapping step) so a `Question`
    /// subagent buried behind three `Running` ones isn't missed.
    pub state: State,
}

/// One top-level session, ready for the layout/ladder/render modules.
/// Elapsed-time (`age`) is a string because the ladder only ever displays
/// it — but it is computed fresh by [`build_projects`] from T09's stored
/// timestamp on every call, per the correct basis for this session's state
/// (T11 contract, acceptance criterion 4): never cached, never baked into
/// anything stored across frames.
#[derive(Clone, Debug)]
pub struct SessionView {
    pub session_id: SessionId,
    pub nick: String,
    pub title: String,
    pub state: State,
    pub age: String,
    pub age_secs: u64,
    /// Whole minutes waited, for `needs-you`/`question` sessions only —
    /// R5.6's "longest-waiting needs-you" priority tier reads this.
    pub wait_m: Option<u32>,
    pub action: Option<String>,
    pub subs: Vec<SubagentView>,
    pub recent: Vec<String>,
    pub files: Vec<String>,
    pub assistant_text: String,
    pub user_prompt: String,
}

#[derive(Clone, Debug)]
pub struct ProjectView {
    pub project_id: ProjectId,
    pub name: String,
    /// Top-level sessions only — subagents are folded into their parent's
    /// `subs` list, never a second entry here (`layout.md` R5.1/R5.6).
    pub sessions: Vec<SessionView>,
}

impl ProjectView {
    pub fn is_all_idle(&self) -> bool {
        !self.sessions.is_empty() && self.sessions.iter().all(|s| s.state == State::Idle)
    }
}

/// Formats an elapsed duration the way `layout.md`'s regime table expects
/// (`9m`, `45s`, `2h`). `now` is always the caller's current render-time
/// clock reading, never a value read from the snapshot itself.
pub fn format_elapsed(now: Timestamp, then: Timestamp) -> String {
    let secs = elapsed_secs(now, then);
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}

fn elapsed_secs(now: Timestamp, then: Timestamp) -> u64 {
    (now.epoch_millis() - then.epoch_millis()).max(0) as u64 / 1000
}

/// FALLBACK-OK: no spec or contract enumerates "a session T11 is asked to
/// render has no naming claim" — neither `visuals.md` R6.8 nor T10's
/// contract says who is responsible for calling `claim_batch`/
/// `claim_session` for a *subagent* session specifically, and T11's own
/// contract limits it to reading T10's public output, never driving
/// claiming itself. This is a genuine cross-task gap between T10 and T12's
/// eventual wiring, not a state this module can resolve — flagged in the
/// T11 report rather than silently designed around. The choice made here
/// (fall back to the session's own harness-native id, still
/// harness-agnostic since T09 already handed it to us as an opaque string)
/// is deliberately non-crashing: T12 calls this render path every frame,
/// so panicking over a missing cosmetic nickname would take down the
/// entire dashboard on every redraw rather than degrade one label.
fn nickname_or_fallback(naming: &NamingClaimMap, session_id: &SessionId) -> String {
    if let Some(nickname) = naming.nickname_of(session_id) {
        return nickname.display_word();
    }
    let raw = &session_id.native_id;
    let truncated: String = raw.chars().take(9).collect();
    if raw.chars().count() > 9 {
        format!("{truncated}\u{2026}")
    } else {
        truncated
    }
}

/// Derives a display name for a project from its identity path — the last
/// path component, or the full path if it has none (e.g. `/`). Pure path
/// manipulation, no opencode knowledge: nothing upstream of this module
/// hands T11 a project display name, so this is T11's own within-scope
/// choice for turning `ProjectId` into the text `layout.md`'s region tag
/// shows.
fn project_display_name(id: &ProjectId) -> String {
    id.as_path()
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| id.as_path().to_string_lossy().into_owned())
}

/// Turns any session's wire attention into the 4-state render model, the
/// elapsed-time basis timestamp, and (for `NeedsYou`/`Question` only) whole
/// minutes waited. Shared by top-level sessions and subagents alike — a
/// subagent's `SessionSnapshot` carries the same `AttentionState` shape
/// (`client.md` R1.5: a subagent is an ordinary session).
fn resolve_state(attention: AttentionState, now: Timestamp) -> (State, Timestamp, Option<u32>) {
    match attention {
        AttentionState::Running { turn_started } => (State::Running, turn_started, None),
        AttentionState::NeedsYou {
            question,
            turn_ended,
        } => {
            let state = if question {
                State::Question
            } else {
                State::NeedsYou
            };
            let minutes = (elapsed_secs(now, turn_ended) / 60) as u32;
            (state, turn_ended, Some(minutes))
        }
        AttentionState::Idle { last_update } => (State::Idle, last_update, None),
    }
}

/// Lower sorts first: `Question` (most urgent) through `Idle` (least).
/// Used only to order a project's subagent list by how much it needs a
/// human's attention — never to order top-level sessions, which follow
/// `layout.md` R5.6's own priority rule elsewhere.
fn state_urgency(state: State) -> u8 {
    match state {
        State::Question => 0,
        State::NeedsYou => 1,
        State::Running => 2,
        State::Idle => 3,
    }
}

fn session_view(
    snap: &SessionSnapshot,
    naming: &NamingClaimMap,
    now: Timestamp,
    subs: Vec<SubagentView>,
) -> SessionView {
    let (state, basis, wait_m) = resolve_state(snap.attention, now);

    SessionView {
        session_id: snap.session_id.clone(),
        nick: nickname_or_fallback(naming, &snap.session_id),
        title: snap.wire_title.clone().unwrap_or_default(),
        state,
        age: format_elapsed(now, basis),
        age_secs: elapsed_secs(now, basis),
        wait_m,
        action: snap.current_action.clone(),
        subs,
        recent: snap.recent_actions.clone(),
        files: snap.files_touched.clone(),
        assistant_text: snap.final_assistant_text.clone().unwrap_or_default(),
        user_prompt: snap.last_user_prompt.clone().unwrap_or_default(),
    }
}

/// Groups `sessions` into projects, top-level sessions only, in
/// first-appearance order within `sessions` itself (`layout.md` R5.1:
/// "projects are always packed in first-appearance order"). This module
/// keeps no state across calls — every call is a fresh grouping of exactly
/// what's handed in this time (R5.4: full recompute, no caching), so
/// "first appearance" means first appearance in this call's slice, not
/// anything remembered from a previous frame; ordering stability across
/// frames is therefore the caller's responsibility (whatever stable order
/// it hands sessions in), not something this function tracks.
///
/// Subagent sessions (`parent_id.is_some()`) never become a `ProjectView`
/// entry of their own; they're folded into their parent's `subs` list.
pub fn build_projects(
    sessions: &[SessionSnapshot],
    naming: &NamingClaimMap,
    now: Timestamp,
) -> Vec<ProjectView> {
    let mut children: HashMap<&SessionId, Vec<&SessionSnapshot>> = HashMap::new();
    for s in sessions {
        if let Some(parent) = &s.parent_id {
            children.entry(parent).or_default().push(s);
        }
    }

    let mut order: Vec<ProjectId> = Vec::new();
    let mut by_id: HashMap<ProjectId, ProjectView> = HashMap::new();

    for s in sessions {
        if s.parent_id.is_some() {
            continue;
        }

        let subs = children
            .get(&s.session_id)
            .map(|kids| {
                let mut views: Vec<SubagentView> = kids
                    .iter()
                    .map(|k| {
                        let (state, _, _) = resolve_state(k.attention, now);
                        SubagentView {
                            nick: nickname_or_fallback(naming, &k.session_id),
                            action: k.current_action.clone().unwrap_or_default(),
                            state,
                        }
                    })
                    .collect();
                // Stable sort: ties (same state) keep spawn order, since
                // `kids` was built by a single pass over `sessions` in its
                // given order. Most-urgent-first so a lone Question isn't
                // buried below several Running lines when tile height only
                // has room to show the top one or two (`ladder.rs`).
                views.sort_by_key(|v| state_urgency(v.state));
                views
            })
            .unwrap_or_default();

        let view = session_view(s, naming, now, subs);

        by_id
            .entry(s.project_id.clone())
            .or_insert_with(|| {
                order.push(s.project_id.clone());
                ProjectView {
                    project_id: s.project_id.clone(),
                    name: project_display_name(&s.project_id),
                    sessions: Vec::new(),
                }
            })
            .sessions
            .push(view);
    }

    order
        .into_iter()
        .map(|id| {
            by_id
                .remove(&id)
                .expect("every ordered id was just inserted")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::naming::LiveSession;
    use crate::snapshot::HarnessKind;
    use std::path::PathBuf;

    const KIND: HarnessKind = HarnessKind("test");

    fn sid(id: &str) -> SessionId {
        SessionId::new(KIND, id)
    }

    fn pid(path: &str) -> ProjectId {
        ProjectId::from_canonical(PathBuf::from(path))
    }

    fn base_snapshot(
        session_id: SessionId,
        project_id: ProjectId,
        attention: AttentionState,
    ) -> SessionSnapshot {
        SessionSnapshot {
            session_id,
            project_id,
            parent_id: None,
            attention,
            current_action: None,
            wire_title: None,
            final_assistant_text: None,
            last_user_prompt: None,
            files_touched: vec![],
            recent_actions: vec![],
            created_at: Timestamp::from_epoch_millis(0),
            last_updated: Timestamp::from_epoch_millis(0),
        }
    }

    #[test]
    fn groups_by_project_in_first_appearance_order() {
        let now = Timestamp::from_epoch_millis(10_000);
        let mut naming = NamingClaimMap::new();
        let sessions = vec![
            base_snapshot(
                sid("s1"),
                pid("/tmp/proj-b"),
                AttentionState::Running {
                    turn_started: Timestamp::from_epoch_millis(0),
                },
            ),
            base_snapshot(
                sid("s2"),
                pid("/tmp/proj-a"),
                AttentionState::Running {
                    turn_started: Timestamp::from_epoch_millis(0),
                },
            ),
            base_snapshot(
                sid("s3"),
                pid("/tmp/proj-b"),
                AttentionState::Running {
                    turn_started: Timestamp::from_epoch_millis(0),
                },
            ),
        ];
        naming.claim_batch(vec![
            LiveSession {
                project_id: pid("/tmp/proj-b"),
                session_id: sid("s1"),
                created_at: Timestamp::from_epoch_millis(0),
            },
            LiveSession {
                project_id: pid("/tmp/proj-a"),
                session_id: sid("s2"),
                created_at: Timestamp::from_epoch_millis(1),
            },
            LiveSession {
                project_id: pid("/tmp/proj-b"),
                session_id: sid("s3"),
                created_at: Timestamp::from_epoch_millis(2),
            },
        ]);

        let projects = build_projects(&sessions, &naming, now);
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].name, "proj-b");
        assert_eq!(projects[0].sessions.len(), 2);
        assert_eq!(projects[1].name, "proj-a");
        assert_eq!(projects[1].sessions.len(), 1);
    }

    #[test]
    fn subagents_fold_into_parent_not_a_separate_project_entry() {
        let now = Timestamp::from_epoch_millis(10_000);
        let naming = NamingClaimMap::new();
        let parent = base_snapshot(
            sid("parent"),
            pid("/tmp/proj"),
            AttentionState::Running {
                turn_started: Timestamp::from_epoch_millis(0),
            },
        );
        let mut child = base_snapshot(
            sid("child"),
            pid("/tmp/proj"),
            AttentionState::Running {
                turn_started: Timestamp::from_epoch_millis(0),
            },
        );
        child.parent_id = Some(sid("parent"));
        child.current_action = Some("editing: foo.rs".into());

        let projects = build_projects(&[parent, child], &naming, now);
        assert_eq!(projects.len(), 1);
        assert_eq!(
            projects[0].sessions.len(),
            1,
            "subagent must not appear as a top-level session"
        );
        assert_eq!(projects[0].sessions[0].subs.len(), 1);
        assert_eq!(projects[0].sessions[0].subs[0].action, "editing: foo.rs");
    }

    #[test]
    fn elapsed_time_is_computed_at_call_time_from_stored_timestamp() {
        let naming = NamingClaimMap::new();
        let snap = base_snapshot(
            sid("s1"),
            pid("/tmp/proj"),
            AttentionState::Running {
                turn_started: Timestamp::from_epoch_millis(0),
            },
        );

        let projects_early = build_projects(
            std::slice::from_ref(&snap),
            &naming,
            Timestamp::from_epoch_millis(5_000),
        );
        assert_eq!(projects_early[0].sessions[0].age, "5s");

        let projects_later = build_projects(&[snap], &naming, Timestamp::from_epoch_millis(65_000));
        assert_eq!(projects_later[0].sessions[0].age, "1m");
    }

    #[test]
    fn question_and_plain_needs_you_map_to_distinct_states() {
        let naming = NamingClaimMap::new();
        let now = Timestamp::from_epoch_millis(120_000);
        let question = base_snapshot(
            sid("q"),
            pid("/tmp/proj"),
            AttentionState::NeedsYou {
                question: true,
                turn_ended: Timestamp::from_epoch_millis(0),
            },
        );
        let plain = base_snapshot(
            sid("p"),
            pid("/tmp/proj"),
            AttentionState::NeedsYou {
                question: false,
                turn_ended: Timestamp::from_epoch_millis(0),
            },
        );
        let projects = build_projects(&[question, plain], &naming, now);
        assert_eq!(projects[0].sessions[0].state, State::Question);
        assert_eq!(projects[0].sessions[1].state, State::NeedsYou);
        assert_eq!(projects[0].sessions[0].wait_m, Some(2));
    }
}
