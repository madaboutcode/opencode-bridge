use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::common::{nickname_display, status_line2, status_line3, truncate, CARD_GAP};
use super::{ContentItem, ProjectLayout};
use crate::fixture::{Project, Session};
use crate::palette;

const CARD_HEIGHT: u16 = 3;

/// "nickname · title" as separately-styled spans (nickname bold in status color, title
/// dim), truncating only the title portion since nicknames are always short enough to fit.
pub fn line1_spans(session: &Session, width: usize, status_color: Color) -> Line<'static> {
    let nick = nickname_display(session);
    let sep = " · ";
    let used = nick.chars().count() + sep.chars().count();
    let remaining = width.saturating_sub(used).max(1);
    let title = truncate(session.title, remaining);
    Line::from(vec![
        Span::styled(nick, Style::new().fg(status_color).add_modifier(Modifier::BOLD)),
        Span::styled(sep, Style::new().fg(palette::TEXT_DIM)),
        Span::styled(title, Style::new().fg(palette::TEXT_DIM)),
    ])
}

pub fn draw_project(f: &mut Frame, rect: Rect, project: &Project, layout: &ProjectLayout, color: Color) {
    if rect.height == 0 {
        return;
    }
    let header = Line::from(vec![
        Span::styled("▎ ", Style::new().fg(color)),
        Span::styled(project.name, Style::new().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(
        Paragraph::new(header),
        Rect { x: rect.x, y: rect.y, width: rect.width, height: 1 },
    );

    if let Some(chip) = &layout.all_idle_chip {
        if rect.height > 1 {
            let p = Paragraph::new(Span::styled(chip.clone(), Style::new().fg(palette::TEXT_DIM)));
            f.render_widget(p, Rect { x: rect.x, y: rect.y + 1, width: rect.width, height: 1 });
        }
        return;
    }

    let content_y0 = rect.y + 1;
    let content_bottom = rect.y + rect.height;
    let mut y = content_y0;
    for row in &layout.rows {
        if y >= content_bottom {
            break;
        }
        let mut x = rect.x;
        for item in row {
            match item {
                ContentItem::Card(session) => {
                    let remaining = (rect.x + rect.width).saturating_sub(x);
                    let w = super::common::CARD_SLOT_WIDTH.min(remaining);
                    let h = CARD_HEIGHT.min(content_bottom.saturating_sub(y));
                    if w > 0 && h > 0 {
                        draw_card(f, Rect { x, y, width: w, height: h }, session);
                    }
                    x += super::common::CARD_SLOT_WIDTH + CARD_GAP;
                }
                ContentItem::Overflow(text) => {
                    let remaining = (rect.x + rect.width).saturating_sub(x);
                    let w = (text.chars().count() as u16).min(remaining);
                    if w > 0 {
                        let p = Paragraph::new(Span::styled(text.clone(), Style::new().fg(palette::TEXT_DIM)));
                        f.render_widget(p, Rect { x, y, width: w, height: 1 });
                    }
                    x += w + CARD_GAP;
                }
            }
        }
        y += CARD_HEIGHT + super::common::ROW_GAP;
    }
}

fn draw_card(f: &mut Frame, rect: Rect, session: &Session) {
    let color = palette::status_color(&session.status);
    let w = rect.width as usize;
    let mut lines = vec![line1_spans(session, w, color)];
    if rect.height > 1 {
        lines.push(Line::from(Span::styled(status_line2(&session.status), Style::new().fg(color))));
    }
    if rect.height > 2 {
        if let Some(l3) = status_line3(&session.status) {
            lines.push(Line::from(Span::styled(truncate(&l3, w), Style::new().fg(palette::TEXT_DIM))));
        }
    }
    f.render_widget(Paragraph::new(lines), rect);
}
