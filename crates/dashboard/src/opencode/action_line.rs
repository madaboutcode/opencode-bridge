//! Tool-call correlation (R6.4) and action-line rendering (R6.5/R6.6) —
//! `docs/specs/dashboard/client.md`. Opencode-specific: this is exactly the
//! logic the boundary (`adapter.rs`) exists to keep out of the core. The
//! core never sees a call id, a tool name, or a raw `input` object — only
//! the `String` this module produces.

use serde_json::Value;
use std::collections::HashMap;

use crate::text::{basename, collapse_newlines};

/// Per-session `call_id -> name` map (R6.4). `session.tool.input.started`
/// is the only opencode event that carries a tool's name; `session.tool.
/// called` carries the args but no name. This tracker joins the two by the
/// shared call id, scoped to one session (the opencode adapter keeps one
/// instance per tracked session — see `session_state.rs`).
#[derive(Debug, Default)]
pub(crate) struct ToolCallTracker {
    call_names: HashMap<String, String>,
}

impl ToolCallTracker {
    /// Records a `session.tool.input.started {id, name}` event.
    pub(crate) fn on_input_started(&mut self, call_id: &str, name: &str) {
        self.call_names
            .insert(call_id.to_string(), name.to_string());
    }

    /// Consumes the matching `input.started` entry (R6.4: "then dropped
    /// once consumed") and returns `(tool name, rendered action line)` for
    /// a `session.tool.called {id, input}` event. Returns `None` when no
    /// matching `input.started` was ever seen for this call id (a missed
    /// or out-of-order event) — the caller holds the previous action line
    /// rather than rendering one with no name, consistent with R6.5's "this
    /// line holds its last value until new activity replaces it."
    pub(crate) fn on_tool_called(
        &mut self,
        call_id: &str,
        input: &Value,
    ) -> Option<(String, String)> {
        let name = self.call_names.remove(call_id)?;
        let line = render_action_line(&name, input);
        Some((name, line))
    }
}

/// R6.5's coarse tool -> action-line mapping, verbatim:
/// - `shell` -> `input.command`, newlines collapsed to `" · "`.
/// - `edit` -> `"editing: "` + basename of `input.path`.
/// - anything else -> `"running: <name>"`.
///
/// Deliberately does NOT truncate to any fixed width. R6.5's wording
/// ("truncate to tile width with `…`") depends on the tile's actual
/// on-screen width, which is a render-time (T11) quantity this
/// long-running background adapter has no access to — the same reasoning
/// `layout.md`'s regime table already applies to nickname truncation
/// ("`nick`... truncated with `…` to fit", computed by the renderer, not
/// baked into the stored value). This function returns the full rendered
/// text; fitting it to a cell is T11's job.
pub(crate) fn render_action_line(name: &str, input: &Value) -> String {
    match name {
        "shell" => {
            let command = input.get("command").and_then(Value::as_str).unwrap_or("");
            collapse_newlines(command)
        }
        "edit" => {
            let path = input.get("path").and_then(Value::as_str).unwrap_or("");
            format!("editing: {}", basename(path))
        }
        other => format!("running: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Fixture shapes reused from
    // docs/internal/opencode-sse-event-catalog-2026-09-01.md §2/§3.

    #[test]
    fn shell_call_renders_command_with_newlines_collapsed() {
        let mut tracker = ToolCallTracker::default();
        tracker.on_input_started("call_1", "shell");
        let input = json!({"command": "cargo test\n--release", "workdir": "/tmp/oc_tool_spike"});
        let (name, line) = tracker.on_tool_called("call_1", &input).unwrap();
        assert_eq!(name, "shell");
        assert_eq!(line, "cargo test · --release");
    }

    #[test]
    fn edit_call_renders_editing_plus_basename() {
        let mut tracker = ToolCallTracker::default();
        tracker.on_input_started("call_2", "edit");
        let input = json!({
            "path": "/tmp/oc_tool_spike/scratch.txt",
            "oldString": "delta echo foxtrot",
            "newString": "delta echo FOXTROT",
        });
        let (name, line) = tracker.on_tool_called("call_2", &input).unwrap();
        assert_eq!(name, "edit");
        assert_eq!(line, "editing: scratch.txt");
    }

    #[test]
    fn unrecognized_tool_falls_back_to_running_name() {
        // grep is a confirmed wire tool (catalog §3) but has no dedicated
        // action-line rule (R6.6's phase 2 is deferred) — must hit the
        // generic fallback, same as glob/patch/write/skill/subagent/read.
        let mut tracker = ToolCallTracker::default();
        tracker.on_input_started("call_3", "grep");
        let input = json!({"pattern": "line", "path": "/tmp/oc_tool_spike"});
        let (name, line) = tracker.on_tool_called("call_3", &input).unwrap();
        assert_eq!(name, "grep");
        assert_eq!(line, "running: grep");
    }

    #[test]
    fn tool_called_with_no_matching_input_started_yields_none() {
        let mut tracker = ToolCallTracker::default();
        let input = json!({"command": "echo hi"});
        assert!(tracker.on_tool_called("call_unknown", &input).is_none());
    }

    #[test]
    fn call_id_is_consumed_after_one_join() {
        let mut tracker = ToolCallTracker::default();
        tracker.on_input_started("call_4", "shell");
        let input = json!({"command": "echo hi"});
        assert!(tracker.on_tool_called("call_4", &input).is_some());
        // R6.4: "then dropped once consumed" — a second `tool.called` for
        // the same id (which shouldn't happen on the real wire, but the
        // tracker must not silently misattribute a stale name if it does)
        // finds nothing.
        assert!(tracker.on_tool_called("call_4", &input).is_none());
    }

    #[test]
    fn edit_path_with_no_directory_component_uses_whole_string() {
        let mut tracker = ToolCallTracker::default();
        tracker.on_input_started("call_5", "edit");
        let input = json!({"path": "scratch.txt"});
        let (_, line) = tracker.on_tool_called("call_5", &input).unwrap();
        assert_eq!(line, "editing: scratch.txt");
    }
}
