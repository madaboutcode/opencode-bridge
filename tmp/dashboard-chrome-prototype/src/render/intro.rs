use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::palette;

const DECIDED: &[(&str, &str)] = &[
    (
        "Attention model is 3 states (running / needs-you / idle)",
        "confirmed directly with the user, no hang heuristic needed for V1",
    ),
    (
        "Card is 3 lines (nickname+title, status+elapsed, current action)",
        "user picked these 3 of 4 candidate lines directly, cost/tokens cut",
    ),
    (
        "Project box screen position is stable once assigned (not re-sorted by urgency)",
        "user picked this directly over reflow, so the screen doesn't rearrange under their eyes",
    ),
    (
        "Nickname is a deterministic adjective-noun handle hashed from session ID, both words \u{2264}6 chars, frozen word list, pure function (no cache/storage)",
        "settled per advisor review",
    ),
];

pub fn draw(f: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "Already decided — not part of this comparison",
        Style::new().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (what, why) in DECIDED {
        lines.push(Line::from(vec![
            Span::styled("  \u{2022} ", Style::new().fg(palette::TEXT_DIM)),
            Span::styled(*what, Style::new().fg(palette::TEXT_PRIMARY)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("      "),
            Span::styled(format!("({why})"), Style::new().fg(palette::TEXT_DIM)),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "The ONLY thing under test here is the grouping chrome (bordered vs boxless vs",
        Style::new().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "project-only-border) and where the nickname goes as a result.",
        Style::new().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD),
    )));

    let p = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(p, area);
}
