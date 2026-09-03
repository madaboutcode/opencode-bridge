use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use super::common::{nickname_display, status_line2, status_line3, truncate, CARD_GAP};
use super::{ContentItem, ProjectLayout};
use crate::fixture::{Project, Session};
use crate::palette;

const CARD_HEIGHT: u16 = 5;

pub fn draw_project(f: &mut Frame, rect: Rect, project: &Project, layout: &ProjectLayout, color: Color) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(color))
        .title(format!(" {} ", project.name));
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    if let Some(chip) = &layout.all_idle_chip {
        if inner.height > 0 {
            let p = Paragraph::new(Line::from(Span::styled(
                chip.clone(),
                Style::new().fg(palette::TEXT_DIM),
            )));
            f.render_widget(p, Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 });
        }
        return;
    }

    let mut y = inner.y;
    for row in &layout.rows {
        if y >= inner.y + inner.height {
            break;
        }
        let mut x = inner.x;
        for item in row {
            match item {
                ContentItem::Card(session) => {
                    let remaining = (inner.x + inner.width).saturating_sub(x);
                    let w = super::common::CARD_SLOT_WIDTH.min(remaining);
                    let h = CARD_HEIGHT.min((inner.y + inner.height).saturating_sub(y));
                    if w > 2 && h > 0 {
                        let card_rect = Rect { x, y, width: w, height: h };
                        draw_card(f, card_rect, session);
                    }
                    x += super::common::CARD_SLOT_WIDTH + CARD_GAP;
                }
                ContentItem::Overflow(text) => {
                    let remaining = (inner.x + inner.width).saturating_sub(x);
                    let w = (text.chars().count() as u16 + 2).min(remaining);
                    if w > 0 {
                        let chip_rect = Rect { x, y: y + 2, width: w, height: 1 };
                        let p = Paragraph::new(Span::styled(
                            text.clone(),
                            Style::new().fg(palette::TEXT_DIM),
                        ));
                        f.render_widget(p, chip_rect);
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
    let title = format!(" {} ", nickname_display(session));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(color))
        .title(title);
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    if inner.height == 0 {
        return;
    }
    let w = inner.width as usize;
    let mut lines = vec![Line::from(Span::styled(
        truncate(session.title, w),
        Style::new().fg(palette::TEXT_PRIMARY),
    ))];
    if inner.height > 1 {
        lines.push(Line::from(Span::styled(
            status_line2(&session.status),
            Style::new().fg(color),
        )));
    }
    if inner.height > 2 {
        if let Some(l3) = status_line3(&session.status) {
            lines.push(Line::from(Span::styled(
                truncate(&l3, w),
                Style::new().fg(palette::TEXT_DIM),
            )));
        }
    }
    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
}
