//! R3/R3.1's active-window reclassification (`docs/specs/dashboard/
//! overview.md` R3) — deliberately *not* T09's job. `snapshot.rs`'s own doc
//! comment on `AttentionState::Idle` says so directly: the opencode adapter
//! only ever constructs `Running`/`NeedsYou`, because the active-window `W`
//! is a core/T12-owned, keyboard-adjustable setting the adapter has no
//! visibility into. This module is that core-side computation: it turns a
//! stale `Running`/`NeedsYou` session into `Idle`, based on
//! `SessionSnapshot::last_updated` — never `attention`'s own timestamp
//! basis (turn-start/turn-end), never any opencode-native "updated" field.

use crate::shell::window::Window;
use crate::snapshot::{AttentionState, SessionSnapshot, Timestamp};

/// Reclassifies `sessions` for this frame's render call. Under
/// `Window::All` (R8's `a`), every session counts as active regardless of
/// age — nothing is reclassified. Under `Window::Minutes(w)`, any session
/// whose `last_updated` is more than `w` minutes older than `now` becomes
/// `Idle`; everything else (including a session already inside the window)
/// is returned unchanged.
pub fn reclassify(
    sessions: Vec<SessionSnapshot>,
    now: Timestamp,
    window: Window,
) -> Vec<SessionSnapshot> {
    let cutoff_ms = match window {
        Window::All => return sessions,
        Window::Minutes(w) => i64::from(w) * 60_000,
    };
    sessions
        .into_iter()
        .map(|mut s| {
            let age_ms = now.epoch_millis() - s.last_updated.epoch_millis();
            if age_ms > cutoff_ms {
                s.attention = AttentionState::Idle {
                    last_update: s.last_updated,
                };
            }
            s
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::{HarnessKind, ProjectId, SessionId};
    use std::path::PathBuf;

    fn snap(last_updated_ms: i64, attention: AttentionState) -> SessionSnapshot {
        SessionSnapshot {
            session_id: SessionId::new(HarnessKind("test"), "s1"),
            project_id: ProjectId::from_canonical(PathBuf::from("/tmp/p")),
            parent_id: None,
            attention,
            current_action: None,
            wire_title: None,
            final_assistant_text: None,
            last_user_prompt: None,
            files_touched: vec![],
            recent_actions: vec![],
            created_at: Timestamp::from_epoch_millis(0),
            last_updated: Timestamp::from_epoch_millis(last_updated_ms),
        }
    }

    #[test]
    fn session_older_than_window_becomes_idle() {
        let now = Timestamp::from_epoch_millis(20 * 60_000); // 20m mark
        let s = snap(
            0, // last updated at t=0, i.e. 20 minutes ago
            AttentionState::NeedsYou {
                question: false,
                turn_ended: Timestamp::from_epoch_millis(0),
            },
        );
        let out = reclassify(vec![s], now, Window::Minutes(10));
        assert!(matches!(out[0].attention, AttentionState::Idle { .. }));
    }

    #[test]
    fn session_inside_window_is_left_alone() {
        let now = Timestamp::from_epoch_millis(5 * 60_000);
        let s = snap(
            0,
            AttentionState::Running {
                turn_started: Timestamp::from_epoch_millis(0),
            },
        );
        let out = reclassify(vec![s], now, Window::Minutes(10));
        assert!(matches!(out[0].attention, AttentionState::Running { .. }));
    }

    #[test]
    fn exactly_at_the_boundary_is_still_active_not_idle() {
        // now - last_updated == window exactly: the contract's cutoff is
        // "more than W minutes old", not "at least W minutes old".
        let now = Timestamp::from_epoch_millis(10 * 60_000);
        let s = snap(
            0,
            AttentionState::Running {
                turn_started: Timestamp::from_epoch_millis(0),
            },
        );
        let out = reclassify(vec![s], now, Window::Minutes(10));
        assert!(matches!(out[0].attention, AttentionState::Running { .. }));
    }

    #[test]
    fn show_all_never_reclassifies_anything_regardless_of_age() {
        let now = Timestamp::from_epoch_millis(10_000 * 60_000);
        let s = snap(
            0,
            AttentionState::NeedsYou {
                question: false,
                turn_ended: Timestamp::from_epoch_millis(0),
            },
        );
        let out = reclassify(vec![s], now, Window::All);
        assert!(matches!(out[0].attention, AttentionState::NeedsYou { .. }));
    }
}
