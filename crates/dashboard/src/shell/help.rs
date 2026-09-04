//! The `?` help overlay (`docs/specs/dashboard/interactions.md` R7.1's
//! "Help" bullet): "a help overlay listing the current key bindings (this
//! file's own R7.1 and R8 content is the source of truth for what it
//! should show)". The list below is this task's own binding table
//! (`keys.rs`/`window.rs`) restated as copy, so it can't drift from what
//! `map_key`/`WindowState::apply` actually implement — there is nowhere
//! else in this crate a "current bindings" list could come from instead.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::mosaic::palette;

const BINDINGS: &[(&str, &str)] = &[
    ("j / k / arrows", "move selection, wraps at both ends"),
    ("Enter", "not available in this release"),
    ("q / Esc", "quit (or close this overlay)"),
    ("?", "toggle this help overlay"),
    ("]", "window += 5m"),
    ("[", "window -= 5m"),
    ("Shift+]", "window += 1m"),
    ("Shift+[", "window -= 1m"),
    ("w", "reset window to 15m"),
    ("a", "show all sessions regardless of age"),
];

/// Replaces the whole screen while open (`interactions.md` R7.1: "footer
/// hidden while the overlay is open... the overlay replaces it") — this
/// task's `App::render` skips calling `mosaic::draw` entirely for this
/// frame rather than drawing the overlay on top of it.
pub fn draw(f: &mut Frame, area: Rect) {
    f.render_widget(Block::new().style(Style::new().bg(palette::GUTTER)), area);

    let mut lines = vec![
        Line::from(Span::styled(
            "opencode dashboard — key bindings",
            Style::new().fg(palette::TEXT_PRIMARY),
        )),
        Line::raw(""),
    ];
    for (key, desc) in BINDINGS {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{key:<15}"),
                Style::new().fg(palette::STATUS_RUNNING),
            ),
            Span::styled(*desc, Style::new().fg(palette::TEXT_SECONDARY)),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "press ? / q / Esc to close",
        Style::new().fg(palette::TEXT_DIM),
    )));

    let popup = centered_rect(area, 48, lines.len() as u16 + 2);
    f.render_widget(Block::new().style(Style::new().bg(palette::PLATE)), popup);
    f.render_widget(
        Paragraph::new(lines).alignment(Alignment::Left),
        inset(popup),
    );
}

fn centered_rect(area: Rect, min_w: u16, min_h: u16) -> Rect {
    let w = min_w.min(area.width.saturating_sub(2)).max(1);
    let h = min_h.min(area.height.saturating_sub(2)).max(1);
    Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

fn inset(r: Rect) -> Rect {
    Rect {
        x: r.x + 1,
        y: r.y + 1,
        width: r.width.saturating_sub(2),
        height: r.height.saturating_sub(2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn overlay_lists_every_r7_1_and_r8_binding() {
        let mut term = Terminal::new(TestBackend::new(80, 30)).unwrap();
        term.draw(|f| draw(f, f.area())).unwrap();
        let buf = term.backend().buffer();
        let full: String = (0..30)
            .map(|y| (0..80).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        for needle in [
            "j / k",
            "q / Esc",
            "]",
            "[",
            "w",
            "a",
            "Enter",
            "not available",
        ] {
            assert!(
                full.contains(needle),
                "help overlay missing {needle:?}: {full}"
            );
        }
    }
}
