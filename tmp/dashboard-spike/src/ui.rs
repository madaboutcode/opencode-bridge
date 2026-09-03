use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::{App, VisibleProject, VisibleSession};
use crate::data::Status;
use crate::squarify::{self, TreemapItem};

// Tokyo Night colors
const BG: Color = Color::Rgb(0x1a, 0x1b, 0x26);
const FG: Color = Color::Rgb(0xc0, 0xca, 0xf5);
const GREEN: Color = Color::Rgb(0x9e, 0xce, 0x6a);
const BLUE: Color = Color::Rgb(0x7a, 0xa2, 0xf7);
const YELLOW: Color = Color::Rgb(0xe0, 0xaf, 0x68);
const RED: Color = Color::Rgb(0xf7, 0x76, 0x8e);
const COMMENT: Color = Color::Rgb(0x56, 0x5f, 0x89);
const MAGENTA: Color = Color::Rgb(0xbb, 0x9a, 0xf7);

const PROJECT_COLORS: [Color; 4] = [BLUE, MAGENTA, GREEN, YELLOW];

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Footer
    let footer_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    let live_count = count_live(app);
    let idle_count = count_idle(app);
    let window_str = if app.show_all {
        "all".to_string()
    } else {
        format!("{}m", app.window.as_secs() / 60)
    };
    let footer_text = if let Some(status) = &app.status {
        format!(
            " {status} │ [−] W−5m  [] W+5m  w reset  a show-all  j/k select  q quit  ↵ zoom",
        )
    } else {
        format!(
            " window: {window_str} ({live_count} live / {idle_count} idle) │ [−] W−5m  [] W+5m  w reset  a show-all  j/k select  q quit  ↵ zoom",
        )
    };
    let footer = Paragraph::new(Line::from(Span::styled(
        footer_text,
        Style::default().fg(COMMENT),
    )));
    frame.render_widget(footer, footer_area);

    // Title bar
    let title_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" ◆ ", Style::default().fg(MAGENTA)),
        Span::styled("dashboard", Style::default().fg(FG).add_modifier(Modifier::BOLD)),
    ]));
    frame.render_widget(title, title_area);

    // Content area
    let content = Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: area.height.saturating_sub(2),
    };

    // Check minimum viewport
    if content.width < 40 || content.height < 12 {
        let msg = Paragraph::new(Line::from(Span::styled(
            "terminal too small",
            Style::default().fg(COMMENT),
        )))
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(msg, content);
        return;
    }

    // Fill content area with background so treemap sits on Tokyo Night
    let bg_block = Block::default().style(Style::default().bg(BG));
    frame.render_widget(bg_block, content);

    let visible_projects = app.get_visible_projects();

    if visible_projects.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "no active sessions",
            Style::default().fg(COMMENT),
        )))
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(msg, content);
        return;
    }

    // Check if project tiles can fit min size
    let min_project_area = 14.0 * 7.0;
    let total_content_area = (content.width as f64) * (content.height as f64);
    // If can't fit all projects, collapse to summary
    if total_content_area < min_project_area * visible_projects.len() as f64 {
        draw_collapsed_projects(frame, &visible_projects, content, app.selected);
        return;
    }

    // Build treemap items for projects
    let project_items: Vec<TreemapItem> = visible_projects
        .iter()
        .enumerate()
        .map(|(_i, p)| TreemapItem {
            id: p.project.id.to_string(),
            weight: p.weight,
            label: p.project.name.to_string(),
        })
        .collect();

    let project_rects = squarify::squarify(&project_items, content);

    let mut global_idx = 0;
    for (proj_idx, (item_idx, proj_rect)) in project_rects.iter().enumerate() {
        let vp = &visible_projects[*item_idx];
        let color = PROJECT_COLORS[proj_idx % PROJECT_COLORS.len()];

        // Draw project block
        let project_block = Block::default()
            .borders(Borders::ALL)
            .title(Line::from(Span::styled(
                format!(" {} ", vp.project.name),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )))
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(color))
            .style(Style::default().bg(BG));

        frame.render_widget(project_block, *proj_rect);

        // Inner area for sessions
        let inner = Rect {
            x: proj_rect.x + 1,
            y: proj_rect.y + 1,
            width: proj_rect.width.saturating_sub(2),
            height: proj_rect.height.saturating_sub(2),
        };

        if inner.width < 12 || inner.height < 5 {
            // Too small for session tiles, show summary
            let summary = format!("{} live, {} idle", vp.active_count, vp.idle_count);
            let p = Paragraph::new(Line::from(Span::styled(
                summary,
                Style::default().fg(COMMENT),
            )));
            frame.render_widget(p, inner);
            global_idx += vp.visible_sessions.len();
            continue;
        }

        // Build session items
        let session_items: Vec<TreemapItem> = vp
            .visible_sessions
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let weight = if s.is_overflow {
                    0.1 // overflow gets tiny weight
                } else if s.is_idle {
                    1.0
                } else {
                    3.0
                };
                TreemapItem {
                    id: format!("{}_{}", vp.project.id, i),
                    weight,
                    label: String::new(),
                }
            })
            .collect();

        let session_rects = squarify::squarify(&session_items, inner);

        for (sess_local_idx, (_, sess_rect)) in session_rects.iter().enumerate() {
            let vs = &vp.visible_sessions[sess_local_idx];
            let is_selected = global_idx == app.selected;

            if vs.is_overflow {
                draw_overflow_tile(frame, vs, *sess_rect, is_selected);
            } else {
                draw_session_tile(frame, vs, *sess_rect, is_selected, color);
            }
            global_idx += 1;
        }
    }
}

fn draw_session_tile(
    frame: &mut Frame,
    vs: &VisibleSession,
    rect: Rect,
    selected: bool,
    _project_color: Color,
) {
    let (status_color, glyph) = match vs.session.status {
        Status::Doing => (GREEN, "●"),
        Status::Thinking => (BLUE, "◐"),
        Status::Waiting => (YELLOW, "◐"),
        Status::Stalled => (RED, "■"),
    };

    let border_color = if vs.session.status == Status::Stalled {
        // Slightly brighter/redder border for stalled
        Color::Rgb(0xff, 0x55, 0x55)
    } else if selected {
        Color::Rgb(0xff, 0xff, 0xff)
    } else {
        status_color
    };

    let title = if vs.session.parent.is_some() {
        format!("↳ {}", vs.session.title)
    } else {
        vs.session.title.to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(vec![
            Span::styled(
                format!("{glyph} "),
                Style::default().fg(status_color),
            ),
            Span::styled(&title, Style::default().fg(FG)),
        ]))
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(BG));

    frame.render_widget(block, rect);

    // Status stripe on left
    let inner = Rect {
        x: rect.x + 1,
        y: rect.y + 1,
        width: rect.width.saturating_sub(2),
        height: rect.height.saturating_sub(2),
    };

    if inner.width >= 2 && inner.height > 0 {
        let stripe = Rect {
            x: inner.x,
            y: inner.y,
            width: 1,
            height: inner.height,
        };
        let stripe_widget = Paragraph::new("").style(Style::default().bg(status_color));
        frame.render_widget(stripe_widget, stripe);
    }
}

fn draw_overflow_tile(
    frame: &mut Frame,
    vs: &VisibleSession,
    rect: Rect,
    selected: bool,
) {
    let border_color = if selected {
        Color::Rgb(0xff, 0xff, 0xff)
    } else {
        COMMENT
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            &vs.overflow_label,
            Style::default().fg(COMMENT).add_modifier(Modifier::ITALIC),
        ))
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(BG));

    frame.render_widget(block, rect);
}

fn draw_collapsed_projects(
    frame: &mut Frame,
    projects: &[VisibleProject],
    area: Rect,
    selected: usize,
) {
    let chunk_height = area.height / projects.len().min(area.height as usize) as u16;
    let mut global_idx = 0;

    for (i, vp) in projects.iter().enumerate() {
        let y = area.y + (i as u16) * chunk_height;
        let h = if i == projects.len() - 1 {
            area.height - (i as u16) * chunk_height
        } else {
            chunk_height
        };

        let rect = Rect { x: area.x, y, width: area.width, height: h };
        let color = PROJECT_COLORS[i % PROJECT_COLORS.len()];
        let is_selected = global_idx == selected;

        let border_color = if is_selected {
            Color::Rgb(0xff, 0xff, 0xff)
        } else {
            color
        };

        let summary = format!(
            "{} — {} live, {} idle",
            vp.project.name, vp.active_count, vp.idle_count
        );

        let block = Block::default()
            .borders(Borders::ALL)
            .title(Line::from(Span::styled(
                format!(" {} ", summary),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )))
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(BG));

        frame.render_widget(block, rect);
        global_idx += vp.visible_sessions.len();
    }
}

fn count_live(app: &App) -> usize {
    let mut count = 0;
    for project in &app.projects {
        for session in &project.sessions {
            if crate::data::is_active(session.ago, app.window) {
                count += 1;
            }
        }
    }
    count
}

fn count_idle(app: &App) -> usize {
    let mut count = 0;
    for project in &app.projects {
        for session in &project.sessions {
            if !crate::data::is_active(session.ago, app.window) {
                count += 1;
            }
        }
    }
    count
}
