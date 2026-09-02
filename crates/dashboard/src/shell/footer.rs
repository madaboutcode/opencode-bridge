//! T12's own footer row (`docs/specs/dashboard/interactions.md` R7.1's
//! "Footer" bullet): the literal `window: W (N live / M idle)` format plus
//! a key-hint reminder, drawn on top of the same bottom row `mosaic::draw`
//! already used for its own, differently-worded footer content
//! (`layout.md` R5.1's `hidden: <name> (N idle)` text plus a generic key
//! hint). T12 owns interaction chrome per its contract (AC6); rather than
//! edit T11's `render.rs` (out of bounds — "must not touch T11 internals"),
//! this module overwrites that one row after `mosaic::draw` returns, using
//! only the public `Frame`/`Buffer` API T11 also uses, folding T11's own
//! `hidden`/`aggregated` signal back in as trailing text so it isn't lost.
//! See this task's report for this judgment call.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::Frame;

use crate::mosaic::ladder::truncate_ellipsis;
use crate::mosaic::palette;
use crate::mosaic::render::{AggregateReport, HeaderCounts};
use crate::shell::window::Window;

const HINT: &str = "q quit  j/k/arrows move  ][ window  w reset  a all  ? help";

/// Builds the exact-format footer line (`interactions.md` R7.1: `window: W
/// (N live / M idle)` / `window: all (N live / 0 idle)`), `N`/`M` read
/// straight from T11's own `DrawReport::header` — which this task's own R3
/// reclassification (`reclassify.rs`) already ran before the sessions it
/// counts ever reached `mosaic::draw`, so `header.idle` here *is* this
/// task's active/idle classification, not a second computation of it.
pub fn text(
    window: Window,
    header: &HeaderCounts,
    hidden: &[(String, usize)],
    aggregate: Option<&AggregateReport>,
) -> String {
    let window_part = match window {
        Window::Minutes(m) => format!("{m}m"),
        Window::All => "all".to_string(),
    };
    let idle = header.idle;
    let live = header.sessions.saturating_sub(idle);
    let mut out = format!("window: {window_part} ({live} live / {idle} idle)  {HINT}");

    let mut extras = Vec::new();
    if !hidden.is_empty() {
        let joined = hidden
            .iter()
            .map(|(n, c)| format!("{n} ({c} idle)"))
            .collect::<Vec<_>>()
            .join(", ");
        extras.push(format!("hidden: {joined}"));
    }
    if let Some(agg) = aggregate {
        extras.push(format!(
            "aggregated: {} projects ({} sessions)",
            agg.project_count, agg.session_count
        ));
    }
    if !extras.is_empty() {
        out.push_str("  ");
        out.push_str(&extras.join("  "));
    }
    out
}

/// Draws `content` onto `area`'s last row, clearing it first (overwriting
/// whatever `mosaic::draw` already put there).
pub fn draw(f: &mut Frame, area: Rect, content: &str) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let row = Rect {
        x: area.x,
        y: area.y + area.height - 1,
        width: area.width,
        height: 1,
    };
    let blank = " ".repeat(row.width as usize);
    f.buffer_mut()
        .set_string(row.x, row.y, &blank, Style::new().bg(palette::GUTTER));
    let truncated = truncate_ellipsis(content, row.width as usize);
    f.buffer_mut().set_string(
        row.x,
        row.y,
        &truncated,
        Style::new().fg(palette::TEXT_PRIMARY),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(sessions: usize, idle: usize) -> HeaderCounts {
        HeaderCounts {
            projects: 1,
            sessions,
            q: 0,
            need: 0,
            run: sessions.saturating_sub(idle),
            idle,
        }
    }

    #[test]
    fn windowed_mode_matches_the_exact_literal_format() {
        let t = text(Window::Minutes(10), &header(8, 5), &[], None);
        assert!(
            t.starts_with("window: 10m (3 live / 5 idle)"),
            "footer text was: {t}"
        );
    }

    #[test]
    fn show_all_mode_matches_the_exact_literal_format() {
        let t = text(Window::All, &header(8, 0), &[], None);
        assert!(
            t.starts_with("window: all (8 live / 0 idle)"),
            "footer text was: {t}"
        );
    }

    #[test]
    fn hidden_projects_are_appended_not_dropped() {
        let t = text(
            Window::Minutes(10),
            &header(3, 0),
            &[("docs-site".to_string(), 1)],
            None,
        );
        assert!(
            t.contains("hidden: docs-site (1 idle)"),
            "footer text was: {t}"
        );
    }
}
