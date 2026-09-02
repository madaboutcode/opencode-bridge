//! Per-session state the opencode adapter keeps to itself between
//! snapshots — the call-id/name correlation map, the current action line,
//! files touched this turn, and the recent-actions ring. None of this ever
//! crosses the `HarnessAdapter` boundary directly; `reconcile.rs`'s
//! `build_snapshot` reads it to fill in the corresponding
//! `SessionSnapshot` fields.

use std::collections::VecDeque;

use crate::snapshot::Timestamp;

use super::action_line::ToolCallTracker;

/// Bound on the recent-actions ring (`layout.md` R5.3's extended running
/// block, priority 7: "shows the most recent `k` entries that fit"). Not a
/// spec'd number — chosen generously enough that the render layer's own
/// space-based trimming is always the binding constraint, not this ring.
const RECENT_ACTIONS_CAP: usize = 20;

#[derive(Debug, Default)]
pub(crate) struct TrackedSession {
    pub(crate) call_tracker: ToolCallTracker,
    pub(crate) current_action: Option<String>,
    pub(crate) files_touched: Vec<String>,
    pub(crate) recent_actions: VecDeque<String>,
    /// Best-effort wall-clock time this adapter first observed the
    /// session's current turn as running — set from `session.execution.
    /// started` when SSE delivers it, or (if the reconcile sweep is what
    /// first discovers a running session, e.g. at startup or after a
    /// missed SSE connect) left `None` and `build_snapshot` falls back to
    /// `last_updated`. There is no wire field for "when did the current
    /// turn start," so this is the adapter's own observation, not a value
    /// read off the wire.
    pub(crate) turn_started: Option<Timestamp>,
}

impl TrackedSession {
    /// Called on `session.tool.called` once a call id resolves to a
    /// rendered action line: rotates the previous `current_action` (if any)
    /// into the recent-actions ring before replacing it, and — for
    /// `edit`/`write` tools — records the touched path.
    pub(crate) fn record_action(&mut self, tool_name: &str, path: Option<&str>, line: String) {
        if let Some(previous) = self.current_action.take() {
            if self.recent_actions.len() == RECENT_ACTIONS_CAP {
                self.recent_actions.pop_front();
            }
            self.recent_actions.push_back(previous);
        }
        self.current_action = Some(line);

        if matches!(tool_name, "edit" | "write") {
            if let Some(path) = path {
                if !self.files_touched.iter().any(|f| f == path) {
                    self.files_touched.push(path.to_string());
                }
            }
        }
    }

    /// Called on `session.execution.started`: a new turn is beginning.
    pub(crate) fn start_turn(&mut self, started_at: Timestamp) {
        self.turn_started = Some(started_at);
        self.files_touched.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_action_rotates_previous_into_recent_actions() {
        let mut tracked = TrackedSession::default();
        tracked.record_action("shell", None, "cargo build".to_string());
        tracked.record_action("shell", None, "cargo test".to_string());

        assert_eq!(tracked.current_action.as_deref(), Some("cargo test"));
        assert_eq!(tracked.recent_actions, vec!["cargo build".to_string()]);
    }

    #[test]
    fn record_action_tracks_edit_and_write_paths_once() {
        let mut tracked = TrackedSession::default();
        tracked.record_action("edit", Some("src/foo.rs"), "editing: foo.rs".to_string());
        tracked.record_action("edit", Some("src/foo.rs"), "editing: foo.rs".to_string());
        tracked.record_action("shell", None, "cargo build".to_string());

        assert_eq!(tracked.files_touched, vec!["src/foo.rs".to_string()]);
    }

    #[test]
    fn start_turn_clears_files_touched_and_sets_turn_started() {
        let mut tracked = TrackedSession::default();
        tracked.record_action("edit", Some("src/foo.rs"), "editing: foo.rs".to_string());
        assert!(!tracked.files_touched.is_empty());

        let t = Timestamp::from_epoch_millis(1_000);
        tracked.start_turn(t);

        assert!(tracked.files_touched.is_empty());
        assert_eq!(tracked.turn_started, Some(t));
    }

    #[test]
    fn recent_actions_ring_is_bounded() {
        let mut tracked = TrackedSession::default();
        for i in 0..(RECENT_ACTIONS_CAP + 5) {
            tracked.record_action("shell", None, format!("cmd {i}"));
        }
        assert_eq!(tracked.recent_actions.len(), RECENT_ACTIONS_CAP);
        // Oldest-first, and the oldest surviving entry is the one that just
        // fell inside the cap window.
        assert_eq!(tracked.recent_actions.front(), Some(&"cmd 4".to_string()));
    }
}
