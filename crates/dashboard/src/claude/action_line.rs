//! Claude's own action-line rendering (finding 2,
//! `tasks/2026-09-05-claude-dashboard-fable-fixes/contracts/T02-tile-content-correctness.md`).
//! Turns a tool call's name and its raw `tool_input` wire string
//! (`docs/specs/dashboard/claude.md` R14 — already on the envelope, already
//! possibly truncated at 4096 bytes per R15) into the one-line `"verb:
//! object"` a Running tile shows, or the `"running: {tool_name}"` fallback
//! for anything that doesn't parse cleanly. Same architectural position as
//! `opencode/action_line.rs` (an adapter-owned action-line renderer reusing
//! `crate::text`'s neutral helpers), but with an important difference: a
//! missing or wrong-typed field here is treated exactly like a parse
//! failure, never papered over with an empty-string default — `state.rs`
//! must never show a raw tool name or a bare argument object, and a silent
//! default would eventually produce exactly that (an empty object string).
//!
//! CONTRACT: render_action_line / render_permission_text
//!
//! GUARANTEES:
//!   - Never panics on any `tool_input` string, including truncated or
//!     otherwise invalid JSON — a truncated `tool_input` is an expected
//!     input (R14/R15's 4096-byte bound), not an exceptional one.
//!   - `render_action_line` returns `"{verb}: {object}"` only when
//!     `tool_input` parses as a JSON object AND `tool_name`'s own expected
//!     field is present with the right JSON type; otherwise
//!     `"running: {tool_name}"`.
//!   - `render_permission_text` reuses the same object extraction and
//!     returns `"allow: {object}"`, falling back to `"allow: {tool_name}"`
//!     under the same conditions.
//!
//! EXPECTS:
//!   - `tool_input` is the raw wire string already on the envelope, already
//!     bounded (`claude.md` R14/R15) — never re-validated here beyond JSON
//!     parsing.
//!
//! DOES NOT:
//!   - Retain, log, or forward `tool_input`'s content.
//!   - Verify the per-tool field names below against Claude's own
//!     tool-argument schema — this table is an assumption read off observed
//!     tool names, not a documented contract; flag if evidence surfaces
//!     that a name or field differs.
//!   - Truncate the rendered line to any tile width — the same reasoning
//!     `opencode/action_line.rs` documents: fitting text to a cell is the
//!     renderer's job, not this adapter-side function's.

use serde_json::Value;

use crate::text::{basename, collapse_newlines};

/// Renders a tool call's Running-tile action line from its name and raw
/// `tool_input` JSON string. Falls back to `"running: {tool_name}"` on
/// parse failure, an unrecognized tool, or a missing/wrong-typed field.
pub(crate) fn render_action_line(tool_name: &str, tool_input: &str) -> String {
    match extract(tool_name, tool_input) {
        Some((verb, object)) => format!("{verb}: {object}"),
        None => format!("running: {tool_name}"),
    }
}

/// The Question tile's text while a permission request is pending (finding
/// 4, item 7): `"allow: {object}"` when the gated call's argument object
/// extracts cleanly, `"allow: {tool_name}"` otherwise — the same fallback
/// rule as [`render_action_line`], reusing the same extraction.
pub(crate) fn render_permission_text(tool_name: &str, tool_input: &str) -> String {
    // FALLBACK-OK: T02 contract item 7 — "parse failure/missing field falls
    // back to the bare tool name, same as the action line."
    let object = extract(tool_name, tool_input)
        .map(|(_, object)| object)
        .unwrap_or_else(|| tool_name.to_string());
    format!("allow: {object}")
}

/// Parses `tool_input` and, for a recognized `tool_name`, extracts its
/// verb and rendered object string. `None` covers every failure mode
/// identically: invalid/truncated JSON, an unrecognized tool name, or the
/// tool's expected field missing or the wrong JSON type — deliberately no
/// default/empty string stands in for a failed extraction.
fn extract(tool_name: &str, tool_input: &str) -> Option<(&'static str, String)> {
    // FALLBACK-OK: T02 contract item 1 — a truncated/invalid tool_input is
    // an expected input (R14/R15's 4096-byte bound truncates mid-object),
    // not an exceptional one; `?` here propagates to `extract`'s `None`,
    // which both callers turn into the documented bare-tool-name fallback.
    let value: Value = serde_json::from_str(tool_input).ok()?;
    // FALLBACK-OK: T02 contract item 2 — a missing field or wrong JSON type
    // is "the same as parse failure — fall back ..., never a default/empty
    // string standing in for the object." Every `?` below (`.get(..)?`,
    // `.as_str()?`) propagates to the same `None` the parse failure above
    // does, on purpose.
    match tool_name {
        "Bash" => {
            let command = value.get("command")?.as_str()?;
            Some(("running", collapse_newlines(command)))
        }
        "Edit" | "Write" => {
            let path = value.get("file_path")?.as_str()?;
            Some(("editing", basename(path).to_string()))
        }
        "Read" => {
            let path = value.get("file_path")?.as_str()?;
            Some(("reading", basename(path).to_string()))
        }
        "Grep" => {
            let pattern = value.get("pattern")?.as_str()?;
            Some(("searching", pattern.to_string()))
        }
        "Agent" => {
            let description = value.get("description")?.as_str()?;
            Some(("delegating", description.to_string()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- render_action_line: items 5(a)-(f) ---

    #[test]
    fn bash_with_valid_command_renders_running_with_collapsed_newlines() {
        let line = render_action_line("Bash", r#"{"command":"echo hi\necho there"}"#);
        assert_eq!(line, "running: echo hi · echo there");
    }

    #[test]
    fn edit_renders_editing_with_basename() {
        let line = render_action_line("Edit", r#"{"file_path":"/work/proj/src/lib.rs"}"#);
        assert_eq!(line, "editing: lib.rs");
    }

    #[test]
    fn write_renders_editing_with_basename() {
        let line = render_action_line("Write", r#"{"file_path":"/work/proj/NOTES.md"}"#);
        assert_eq!(line, "editing: NOTES.md");
    }

    #[test]
    fn read_renders_reading_with_basename() {
        let line = render_action_line("Read", r#"{"file_path":"/work/proj/src/main.rs"}"#);
        assert_eq!(line, "reading: main.rs");
    }

    #[test]
    fn grep_renders_searching_with_pattern() {
        let line = render_action_line("Grep", r#"{"pattern":"TODO"}"#);
        assert_eq!(line, "searching: TODO");
    }

    #[test]
    fn agent_renders_delegating_with_description() {
        let line = render_action_line("Agent", r#"{"description":"investigate flaky test"}"#);
        assert_eq!(line, "delegating: investigate flaky test");
    }

    #[test]
    fn unknown_tool_name_renders_bare_running_fallback() {
        let line = render_action_line("SomeMcpTool", r#"{"whatever":"value"}"#);
        assert_eq!(line, "running: SomeMcpTool");
    }

    // --- render_action_line: item 5(g), the truncation-safety test ---

    /// Builds a `tool_input` string that is genuinely truncated the way
    /// `hook.rs`'s `bounded_text` (R14/R15, 4096-byte field cap) would
    /// produce it: a long JSON string value cut mid-way, leaving an
    /// unterminated string and no closing brace. This is not a
    /// hand-picked string that merely *looks* invalid — cutting a real JSON
    /// object bigger than 4096 bytes at exactly the 4096-byte boundary is
    /// the actual truncation condition, and `serde_json::from_str` genuinely
    /// fails to parse it (unexpected end of input while inside a string).
    /// A naive implementation that calls `.unwrap()` (or otherwise doesn't
    /// treat a parse error as the same case as "missing field") panics on
    /// this input, so this test fails loudly against that implementation
    /// rather than silently passing.
    fn truncated_bash_tool_input() -> String {
        let long_command = "x".repeat(5_000);
        let full = format!(r#"{{"command":"{long_command}"}}"#);
        assert!(full.len() > 4096, "fixture must exceed the real R15 bound");
        full[..4096].to_string()
    }

    #[test]
    fn truncated_tool_input_falls_back_without_panicking() {
        let truncated = truncated_bash_tool_input();
        assert!(
            serde_json::from_str::<Value>(&truncated).is_err(),
            "fixture must actually fail to parse, not merely look invalid"
        );
        let line = render_action_line("Bash", &truncated);
        assert_eq!(line, "running: Bash");
    }

    // --- render_permission_text: item 7 ---

    #[test]
    fn permission_text_uses_the_same_object_extraction() {
        let text = render_permission_text("Bash", r#"{"command":"rm -rf build"}"#);
        assert_eq!(text, "allow: rm -rf build");
    }

    #[test]
    fn permission_text_falls_back_to_tool_name_on_parse_failure() {
        let truncated = truncated_bash_tool_input();
        let text = render_permission_text("Bash", &truncated);
        assert_eq!(text, "allow: Bash");
    }
}
