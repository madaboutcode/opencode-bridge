// Draws the Mosaic layout to a ratatui Frame and reports what it drew (BRIEF-v2.md
// "Header and footer", "Project tag", B1/B2/B3, and the Report section's structural
// facts — cell-classification itself is done by main.rs from the rendered buffer).

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;
use ratatui::Frame;

use crate::fixture::{Fixture, Project, State};
use crate::ladder::{self, truncate_ellipsis};
use crate::layout::{self, RegionContent, RegionKind};
use crate::palette;

pub struct TileReport {
    pub session_nick: String,
    pub raw: Rect,
    pub weight: f64,
    pub wi: u16,
    pub h: u16,
    pub regime: &'static str,
    pub blocks_rendered: Vec<&'static str>,
    pub blank_rows_left: usize,
    pub dropped: bool,
}

pub struct IdleRowReport {
    pub chips_shown: Vec<String>,
    pub overflow_count: usize,
}

pub struct RegionReport {
    pub project_name: String,
    pub project_idx: usize,
    pub raw: Rect,
    pub plate: Rect,
    pub kind: RegionKind,
    pub weight: f64,
    pub aspect: f64,
    pub tag_counts_shown: bool,
    pub idle_row: Option<IdleRowReport>,
    pub tiles: Vec<TileReport>,
}

#[derive(Default)]
pub struct HeaderCounts {
    pub projects: usize,
    pub sessions: usize,
    pub q: usize,
    pub need: usize,
    pub run: usize,
    pub idle: usize,
}

pub struct DrawReport {
    pub too_small: bool,
    pub header: HeaderCounts,
    pub hidden_projects: Vec<(String, usize)>,
    pub regions: Vec<RegionReport>,
}

pub fn draw(f: &mut Frame, area: Rect, fixture: &Fixture, tick: usize) -> DrawReport {
    f.render_widget(Block::new().style(Style::new().bg(palette::GUTTER)), area);

    if area.width < 40 || area.height < 12 {
        draw_too_small(f, area);
        return DrawReport { too_small: true, header: HeaderCounts::default(), hidden_projects: vec![], regions: vec![] };
    }

    let header_rect = Rect { x: area.x, y: area.y, width: area.width, height: 1 };
    let footer_rect = Rect { x: area.x, y: area.y + area.height - 1, width: area.width, height: 1 };
    let body_rect = Rect { x: area.x, y: area.y + 1, width: area.width, height: area.height - 2 };

    f.render_widget(Block::new().style(Style::new().bg(palette::GUTTER)), body_rect);

    let regions = layout::layout_body(fixture, body_rect);

    let mut region_reports = Vec::with_capacity(regions.len());
    for rc in &regions {
        region_reports.push(draw_region(f, fixture, rc, tick));
    }

    let header = compute_header_counts(fixture, &regions);
    let hidden = hidden_projects(fixture);

    draw_header(f, header_rect, &header, tick);
    draw_footer(f, footer_rect, &hidden);

    DrawReport { too_small: false, header, hidden_projects: hidden, regions: region_reports }
}

fn draw_region(f: &mut Frame, fixture: &Fixture, rc: &RegionContent, tick: usize) -> RegionReport {
    let project = &fixture.projects[rc.region.project_idx];
    let aspect = rc.region.aspect_ratio();
    let base = |tag_counts_shown, idle_row, tiles| RegionReport {
        project_name: project.name.clone(),
        project_idx: rc.region.project_idx,
        raw: rc.region.raw,
        plate: rc.region.plate,
        kind: rc.region.kind,
        weight: rc.region.weight,
        aspect,
        tag_counts_shown,
        idle_row,
        tiles,
    };

    if rc.region.kind == RegionKind::NotDrawn {
        return base(false, None, vec![]);
    }

    f.render_widget(Block::new().style(Style::new().bg(palette::PLATE)), rc.region.plate);
    let accent = palette::project_color(rc.region.project_idx);
    let tag_counts_shown = draw_tag_row(f, rc.tag_rect, project, accent, tick);

    if rc.region.kind == RegionKind::TagOnly {
        return base(tag_counts_shown, None, vec![]);
    }

    let idle_row = rc.idle_row_rect.map(|rect| draw_idle_row(f, rect, project, &rc.idle_sessions_sorted));

    let tiles: Vec<TileReport> =
        rc.tiles.iter().map(|t| draw_tile(f, &project.sessions[t.session_idx], t, tick)).collect();

    base(tag_counts_shown, idle_row, tiles)
}

fn draw_tag_row(f: &mut Frame, rect: Rect, project: &Project, accent: Color, tick: usize) -> bool {
    if rect.width == 0 || rect.height == 0 {
        return false;
    }
    let count = project.sessions.len();
    let count_text = format!("{count}");
    let fixed_after_name = 1 + count_text.chars().count();
    let name_budget = (rect.width as usize).saturating_sub(2 + fixed_after_name).max(3);
    let name = truncate_ellipsis(&project.name, name_budget);

    let mut x = rect.x;
    let badge_text = format!(" {name} ");
    f.buffer_mut().set_string(
        x,
        rect.y,
        &badge_text,
        Style::new().fg(palette::GUTTER).bg(accent).add_modifier(Modifier::BOLD),
    );
    x += badge_text.chars().count() as u16;
    let count_span = format!(" {count_text}");
    f.buffer_mut().set_string(x, rect.y, &count_span, Style::new().fg(palette::TEXT_DIM).add_modifier(Modifier::BOLD));
    x += count_span.chars().count() as u16;

    if rect.width < 24 {
        return false;
    }

    let mut q = 0usize;
    let mut need = 0usize;
    let mut run = 0usize;
    let mut idle = 0usize;
    for s in &project.sessions {
        match s.state {
            State::Question => q += 1,
            State::NeedsYou => need += 1,
            State::Running => run += 1,
            State::Idle => idle += 1,
        }
    }
    let mut segs: Vec<(String, Color)> = vec![];
    if q > 0 {
        segs.push((format!("? {q}"), palette::STATUS_QUESTION));
    }
    if need > 0 {
        segs.push((format!("● {need}"), palette::STATUS_NEEDS_YOU));
    }
    if run > 0 {
        segs.push((format!("{} {run}", palette::running_glyph(tick)), palette::STATUS_RUNNING));
    }
    if idle > 0 {
        segs.push((format!("○ {idle}"), palette::STATUS_IDLE));
    }
    if segs.is_empty() {
        return false;
    }
    let joined_len: usize =
        segs.iter().map(|(t, _)| t.chars().count()).sum::<usize>() + segs.len().saturating_sub(1) * 2;
    let right_edge = rect.x + rect.width;
    if joined_len as u16 > rect.width || right_edge < joined_len as u16 {
        return false;
    }
    let start_x = right_edge - joined_len as u16;
    if start_x < x {
        return false;
    }
    let mut cx = start_x;
    for (i, (t, color)) in segs.iter().enumerate() {
        if i > 0 {
            cx += 2;
        }
        f.buffer_mut().set_string(cx, rect.y, t, Style::new().fg(*color).add_modifier(Modifier::BOLD));
        cx += t.chars().count() as u16;
    }
    true
}

fn draw_idle_row(f: &mut Frame, rect: Rect, project: &Project, idle_sorted: &[usize]) -> IdleRowReport {
    if rect.width == 0 {
        return IdleRowReport { chips_shown: vec![], overflow_count: idle_sorted.len() };
    }
    let avail = (rect.width as usize).saturating_sub(8);
    let mut shown = vec![];
    let mut used = 0usize;
    let mut placed = 0usize;
    for &idx in idle_sorted {
        let s = &project.sessions[idx];
        let chip = format!("○ {} · {}", s.nick, s.age);
        let clen = chip.chars().count();
        let add = if placed == 0 { clen } else { clen + 2 };
        if used + add > avail {
            break;
        }
        used += add;
        placed += 1;
        shown.push(chip);
    }
    let mut x = rect.x;
    for (i, chip) in shown.iter().enumerate() {
        if i > 0 {
            x += 2;
        }
        f.buffer_mut().set_string(x, rect.y, chip, Style::new().fg(palette::STATUS_IDLE));
        x += chip.chars().count() as u16;
    }
    let remaining = idle_sorted.len() - placed;
    if remaining > 0 {
        let text = format!("+{remaining} idle");
        let start_x = if placed > 0 { x + 2 } else { rect.x };
        f.buffer_mut().set_string(start_x, rect.y, &text, Style::new().fg(palette::TEXT_DIM));
    }
    IdleRowReport { chips_shown: shown, overflow_count: remaining }
}

fn draw_tile(f: &mut Frame, s: &crate::fixture::Session, t: &layout::TileLayout, tick: usize) -> TileReport {
    let plate = t.plate;
    if plate.width == 0 || plate.height == 0 {
        return TileReport {
            session_nick: s.nick.clone(),
            raw: t.raw,
            weight: t.weight,
            wi: 0,
            h: 0,
            regime: "dropped",
            blocks_rendered: vec![],
            blank_rows_left: 0,
            dropped: true,
        };
    }
    f.render_widget(Block::new().style(Style::new().bg(palette::tile_bg(s.state))), plate);
    let wi = plate.width.saturating_sub(2);
    let h = plate.height;
    let content = ladder::build_tile_content(s, wi, h, tick);
    if wi > 0 {
        let text_x = plate.x + 1;
        for (i, line) in content.lines.iter().enumerate() {
            if i as u16 >= h {
                break;
            }
            f.buffer_mut().set_line(text_x, plate.y + i as u16, line, wi);
        }
    }
    TileReport {
        session_nick: s.nick.clone(),
        raw: t.raw,
        weight: t.weight,
        wi,
        h,
        regime: content.regime,
        blocks_rendered: content.blocks_rendered,
        blank_rows_left: content.blank_rows_left,
        dropped: false,
    }
}

fn compute_header_counts(fixture: &Fixture, regions: &[RegionContent]) -> HeaderCounts {
    let mut hc = HeaderCounts::default();
    for rc in regions {
        if rc.region.kind == RegionKind::NotDrawn {
            continue;
        }
        hc.projects += 1;
        let project = &fixture.projects[rc.region.project_idx];
        for s in &project.sessions {
            hc.sessions += 1;
            match s.state {
                State::Question => hc.q += 1,
                State::NeedsYou => hc.need += 1,
                State::Running => hc.run += 1,
                State::Idle => hc.idle += 1,
            }
        }
    }
    hc
}

fn hidden_projects(fixture: &Fixture) -> Vec<(String, usize)> {
    fixture.projects.iter().filter(|p| p.is_all_idle()).map(|p| (p.name.clone(), p.sessions.len())).collect()
}

fn draw_header(f: &mut Frame, rect: Rect, hc: &HeaderCounts, tick: usize) {
    if rect.width == 0 {
        return;
    }
    let wide = rect.width >= 100;
    let mut left: Vec<Span<'static>> = vec![Span::styled(
        " ◆ opencode ",
        Style::new().fg(palette::STATUS_RUNNING).add_modifier(Modifier::BOLD),
    )];
    left.push(Span::raw(" · "));
    if wide {
        left.push(Span::styled(
            format!("{} projects · {} sessions", hc.projects, hc.sessions),
            Style::new().fg(palette::TEXT_DIM),
        ));
    } else {
        left.push(Span::styled(format!("{} sessions", hc.sessions), Style::new().fg(palette::TEXT_DIM)));
    }
    if hc.q > 0 {
        left.push(Span::raw(" · "));
        left.push(Span::styled(
            format!("? {}", hc.q),
            Style::new().fg(palette::STATUS_QUESTION).add_modifier(Modifier::BOLD),
        ));
    }
    if hc.need > 0 {
        left.push(Span::raw(" · "));
        left.push(Span::styled(
            format!("● {}", hc.need),
            Style::new().fg(palette::STATUS_NEEDS_YOU).add_modifier(Modifier::BOLD),
        ));
    }
    if hc.run > 0 {
        left.push(Span::raw(" · "));
        left.push(Span::styled(
            format!("{} {}", palette::running_glyph(tick), hc.run),
            Style::new().fg(palette::STATUS_RUNNING).add_modifier(Modifier::BOLD),
        ));
    }
    if hc.idle > 0 {
        left.push(Span::raw(" · "));
        left.push(Span::styled(
            format!("○ {}", hc.idle),
            Style::new().fg(palette::TEXT_DIM).add_modifier(Modifier::BOLD),
        ));
    }
    let left_line = Line::from(left);
    f.buffer_mut().set_line(rect.x, rect.y, &left_line, rect.width);

    let right_spans: Vec<Span<'static>> = if wide {
        vec![
            Span::styled("window 10m  ", Style::new().fg(palette::TEXT_DIM)),
            Span::styled("●", Style::new().fg(palette::LIVE)),
            Span::styled(" live", Style::new().fg(palette::TEXT_DIM)),
        ]
    } else {
        vec![
            Span::styled("10m ", Style::new().fg(palette::TEXT_DIM)),
            Span::styled("●", Style::new().fg(palette::LIVE)),
            Span::styled(" live", Style::new().fg(palette::TEXT_DIM)),
        ]
    };
    let right_line = Line::from(right_spans);
    let rw: u16 = right_line.spans.iter().map(|s| s.content.chars().count() as u16).sum();
    if rw <= rect.width {
        let x = rect.x + rect.width - rw;
        f.buffer_mut().set_line(x, rect.y, &right_line, rw);
    }
}

fn draw_footer(f: &mut Frame, rect: Rect, hidden: &[(String, usize)]) {
    if rect.width == 0 {
        return;
    }
    let left_text = "q quit  w 80col  f fixture  +/- session  p project  . tick";
    let left_len = left_text.chars().count() as u16;
    f.buffer_mut().set_string(rect.x, rect.y, left_text, Style::new().fg(palette::TEXT_DIM));

    if hidden.is_empty() {
        return;
    }
    let joined = hidden.iter().map(|(n, c)| format!("{n} ({c} idle)")).collect::<Vec<_>>().join(", ");
    let text = format!("hidden: {joined}");
    // Right side must not collide with the left legend — truncate to whatever room is
    // actually left after it, not the full row width.
    let gap = 2u16;
    let right_budget = rect.width.saturating_sub(left_len + gap);
    if right_budget == 0 {
        return;
    }
    let truncated = truncate_ellipsis(&text, right_budget as usize);
    let tw = truncated.chars().count() as u16;
    if tw <= rect.width {
        let x = rect.x + rect.width - tw;
        f.buffer_mut().set_string(x, rect.y, &truncated, Style::new().fg(palette::TEXT_DIM));
    }
}

fn draw_too_small(f: &mut Frame, area: Rect) {
    let msg = "terminal too small";
    if area.height == 0 {
        return;
    }
    let msg_w = (msg.chars().count() as u16).min(area.width);
    let x = area.x + area.width.saturating_sub(msg_w) / 2;
    let y = area.y + area.height / 2;
    f.buffer_mut().set_string(x, y, msg, Style::new().fg(palette::TEXT_PRIMARY));
}
