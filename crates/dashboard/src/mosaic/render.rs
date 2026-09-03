//! Draws the Mosaic layout to a ratatui `Frame` and reports what it drew.
//! Ported from the verified spike
//! (`tmp/20260901-prototype-dashboard-layout/src/render.rs`), rewired to
//! consume `crate::mosaic::view::build_projects` (real T09 snapshots + T10
//! naming) instead of the spike's fixture, plus two things the spike never
//! drew at all (`layout.md` R9 series — see the T11 contract's instruction
//! to verify each degrade state rather than assume it carried over):
//!
//! - **R9** — the centered "zero sessions active" empty-state panel
//!   ([`draw_empty_state`]). The spike had no such panel; an all-idle world
//!   just rendered an empty body with nothing in it.
//! - **R9.2's aggregate chip** — [`draw_aggregate_chip`], for the
//!   `layout::AggregateChip` `layout.rs` now produces when projects don't
//!   all fit even as summary tiles.
//!
//! R9.1 (below 40x12 → "terminal too small") was already correct in the
//! spike and is carried over unchanged ([`draw_too_small`]).

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;
use ratatui::Frame;

use crate::mosaic::ladder::{self, truncate_ellipsis};
use crate::mosaic::layout::{self, AggregateChip, RegionContent, RegionKind};
use crate::mosaic::palette;
use crate::mosaic::view::{self, ProjectView, State};
use crate::naming::NamingClaimMap;
use crate::snapshot::{SessionSnapshot, Timestamp};

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

pub struct BottomRowReport {
    pub chips_shown: Vec<String>,
    pub idle_overflow_count: usize,
    pub session_overflow_count: usize,
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
    pub bottom_row: Option<BottomRowReport>,
    pub tiles: Vec<TileReport>,
}

pub struct AggregateReport {
    pub raw: Rect,
    pub project_count: usize,
    pub session_count: usize,
}

pub struct EmptyStateReport {
    pub window_minutes: u32,
    pub hidden_idle_count: usize,
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
    pub empty_state: Option<EmptyStateReport>,
    pub header: HeaderCounts,
    pub hidden_projects: Vec<(String, usize)>,
    pub aggregate: Option<AggregateReport>,
    pub regions: Vec<RegionReport>,
}

/// Entry point (T11 contract: "T12's main loop calls this task's render
/// function every frame"). Consumes only T09's `SessionSnapshot` and T10's
/// `NamingClaimMap` public surface — no opencode-specific knowledge
/// anywhere below this call (T11 contract, acceptance criterion 6).
///
/// `now` drives every elapsed-time string and the running-glyph animation
/// tick, computed fresh on every call — nothing here is cached across
/// frames (`layout.md` R5.4). `window_minutes` is `overview.md` R3's
/// active-window setting, owned by T12/the core, not this module; it's
/// used only for R9's empty-state copy.
pub fn draw(
    f: &mut Frame,
    area: Rect,
    sessions: &[SessionSnapshot],
    naming: &NamingClaimMap,
    now: Timestamp,
    window_minutes: u32,
) -> DrawReport {
    f.render_widget(Block::new().style(Style::new().bg(palette::GUTTER)), area);

    // R9.1: below the minimum viewport, draw nothing else.
    if area.width < layout::VIEWPORT_MIN_W || area.height < layout::VIEWPORT_MIN_H {
        draw_too_small(f, area);
        return DrawReport {
            too_small: true,
            empty_state: None,
            header: HeaderCounts::default(),
            hidden_projects: vec![],
            aggregate: None,
            regions: vec![],
        };
    }

    let header_rect = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };
    let footer_rect = Rect {
        x: area.x,
        y: area.y + area.height - 1,
        width: area.width,
        height: 1,
    };
    let body_rect = Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: area.height - 2,
    };

    f.render_widget(
        Block::new().style(Style::new().bg(palette::GUTTER)),
        body_rect,
    );

    let projects = view::build_projects(sessions, naming, now);
    let header = compute_header_counts(&projects);
    let hidden = hidden_projects(&projects);
    let tick = (now.epoch_millis().max(0) / 250) as usize;

    // R9: zero sessions active within the current window. `attention` has
    // already been reclassified to `Idle` for anything outside the window
    // by the time it reaches this snapshot (see `snapshot.rs`'s
    // `AttentionState` doc comment) — so "no active session anywhere"
    // reduces to "every session in every project is `Idle`".
    let any_active = projects
        .iter()
        .any(|p| p.sessions.iter().any(|s| s.state != State::Idle));
    if !any_active {
        let hidden_idle_count: usize = projects.iter().map(|p| p.sessions.len()).sum();
        draw_empty_state(f, body_rect, window_minutes, hidden_idle_count);
        draw_header(f, header_rect, &header, tick);
        draw_footer(f, footer_rect, &hidden, None);
        return DrawReport {
            too_small: false,
            empty_state: Some(EmptyStateReport {
                window_minutes,
                hidden_idle_count,
            }),
            header,
            hidden_projects: hidden,
            aggregate: None,
            regions: vec![],
        };
    }

    let body_layout = layout::layout_body(&projects, body_rect);

    let mut region_reports = Vec::with_capacity(body_layout.regions.len());
    for rc in &body_layout.regions {
        region_reports.push(draw_region(f, &projects, rc, tick));
    }

    let aggregate_report = body_layout.aggregate.as_ref().map(|agg| {
        draw_aggregate_chip(f, agg);
        AggregateReport {
            raw: agg.raw,
            project_count: agg.project_indices.len(),
            session_count: agg.session_count,
        }
    });

    draw_header(f, header_rect, &header, tick);
    draw_footer(f, footer_rect, &hidden, aggregate_report.as_ref());

    DrawReport {
        too_small: false,
        empty_state: None,
        header,
        hidden_projects: hidden,
        aggregate: aggregate_report,
        regions: region_reports,
    }
}

fn draw_region(
    f: &mut Frame,
    projects: &[ProjectView],
    rc: &RegionContent,
    tick: usize,
) -> RegionReport {
    let project = &projects[rc.region.project_idx];
    let aspect = rc.region.aspect_ratio();
    let base = |tag_counts_shown, bottom_row, tiles| RegionReport {
        project_name: project.name.clone(),
        project_idx: rc.region.project_idx,
        raw: rc.region.raw,
        plate: rc.region.plate,
        kind: rc.region.kind,
        weight: rc.region.weight,
        aspect,
        tag_counts_shown,
        bottom_row,
        tiles,
    };

    f.render_widget(
        Block::new().style(Style::new().bg(palette::PLATE)),
        rc.region.plate,
    );
    // R5.11: the project accent goes on the tag's project-name text only —
    // never the plate background just drawn above, never a tile, never a
    // border (there are none).
    let accent = palette::project_color(rc.region.project_idx);
    let tag_counts_shown = draw_tag_row(f, rc.tag_rect, project, accent, tick);

    if rc.region.kind == RegionKind::Summary {
        return base(tag_counts_shown, None, vec![]);
    }

    let bottom_row = rc.bottom_row_rect.map(|rect| {
        draw_bottom_row(
            f,
            rect,
            project,
            &rc.idle_sessions_sorted,
            rc.overflow_count,
        )
    });

    let tiles: Vec<TileReport> = rc
        .tiles
        .iter()
        .map(|t| draw_tile(f, &project.sessions[t.session_idx], t, tick))
        .collect();

    base(tag_counts_shown, bottom_row, tiles)
}

fn draw_tag_row(
    f: &mut Frame,
    rect: Rect,
    project: &ProjectView,
    accent: Color,
    tick: usize,
) -> bool {
    // FALLBACK-OK: layout.md R9.2 — "the layout will leave space unused
    // ... rather than violate a higher-priority rule to fill it." A
    // zero-size tag rect is squarify's own degenerate-allocation output
    // (extreme project counts, see `layout.rs`'s aggregation doc comment);
    // drawing nothing here, rather than asserting, is exactly what R9.2
    // asks for.
    if rect.width == 0 || rect.height == 0 {
        return false;
    }
    let count = project.sessions.len();
    let count_text = format!("{count}");
    let fixed_after_name = 1 + count_text.chars().count();
    let name_budget = (rect.width as usize)
        .saturating_sub(2 + fixed_after_name)
        .max(3);
    let name = truncate_ellipsis(&project.name, name_budget);

    let mut x = rect.x;
    let badge_text = format!(" {name} ");
    f.buffer_mut().set_string(
        x,
        rect.y,
        &badge_text,
        Style::new()
            .fg(palette::GUTTER)
            .bg(accent)
            .add_modifier(Modifier::BOLD),
    );
    x += badge_text.chars().count() as u16;
    let count_span = format!(" {count_text}");
    f.buffer_mut().set_string(
        x,
        rect.y,
        &count_span,
        Style::new()
            .fg(palette::TEXT_DIM)
            .add_modifier(Modifier::BOLD),
    );
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
        segs.push((
            format!("{} {q}", palette::state_glyph(State::Question, tick)),
            palette::STATUS_QUESTION,
        ));
    }
    if need > 0 {
        segs.push((
            format!("{} {need}", palette::state_glyph(State::NeedsYou, tick)),
            palette::STATUS_NEEDS_YOU,
        ));
    }
    if run > 0 {
        segs.push((
            format!("{} {run}", palette::running_glyph(tick)),
            palette::STATUS_RUNNING,
        ));
    }
    if idle > 0 {
        segs.push((
            format!("{} {idle}", palette::state_glyph(State::Idle, tick)),
            palette::STATUS_IDLE,
        ));
    }
    if segs.is_empty() {
        return false;
    }
    let joined_len: usize = segs.iter().map(|(t, _)| t.chars().count()).sum::<usize>()
        + segs.len().saturating_sub(1) * 2;
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
        f.buffer_mut().set_string(
            cx,
            rect.y,
            t,
            Style::new().fg(*color).add_modifier(Modifier::BOLD),
        );
        cx += t.chars().count() as u16;
    }
    true
}

/// Draws the region's bottom row: idle chips (`○ nick · age`,
/// most-recent-first), then the R5.6 overflow chip (`+N sessions`) if this
/// project had more than 3 active sessions, then the R5.2 idle overflow
/// (`+N idle`) if chips ran out of room. See `layout.rs`'s
/// `RegionContent::bottom_row_rect` doc comment for why both concerns
/// share this one row.
fn draw_bottom_row(
    f: &mut Frame,
    rect: Rect,
    project: &ProjectView,
    idle_sorted: &[usize],
    session_overflow_count: usize,
) -> BottomRowReport {
    // FALLBACK-OK: layout.md R9.2 — same degenerate-allocation reasoning as
    // `draw_tag_row` above; a zero-width bottom row draws no chips rather
    // than asserting.
    if rect.width == 0 {
        return BottomRowReport {
            chips_shown: vec![],
            idle_overflow_count: idle_sorted.len(),
            session_overflow_count,
        };
    }
    let avail = (rect.width as usize).saturating_sub(8);
    let mut shown = vec![];
    let mut used = 0usize;
    let mut placed = 0usize;
    for &idx in idle_sorted {
        let s = &project.sessions[idx];
        let chip = format!(
            "{} {} · {}",
            palette::state_glyph(State::Idle, 0), // tick unused for Idle
            s.nick,
            s.age
        );
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
        f.buffer_mut()
            .set_string(x, rect.y, chip, Style::new().fg(palette::STATUS_IDLE));
        x += chip.chars().count() as u16;
    }

    let idle_overflow_count = idle_sorted.len() - placed;
    let mut trailer_parts: Vec<String> = vec![];
    if session_overflow_count > 0 {
        trailer_parts.push(format!("+{session_overflow_count} sessions"));
    }
    if idle_overflow_count > 0 {
        trailer_parts.push(format!("+{idle_overflow_count} idle"));
    }
    if !trailer_parts.is_empty() {
        let text = trailer_parts.join("  ");
        let start_x = if placed > 0 { x + 2 } else { rect.x };
        f.buffer_mut()
            .set_string(start_x, rect.y, &text, Style::new().fg(palette::TEXT_DIM));
    }
    BottomRowReport {
        chips_shown: shown,
        idle_overflow_count,
        session_overflow_count,
    }
}

fn draw_tile(
    f: &mut Frame,
    s: &view::SessionView,
    t: &layout::TileLayout,
    tick: usize,
) -> TileReport {
    let plate = t.plate;
    // FALLBACK-OK: layout.md R9.2 — same reasoning as `draw_tag_row`; a
    // degenerate tile allocation is reported as `dropped` (visible in the
    // evidence dump) rather than asserted.
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
    f.render_widget(
        Block::new().style(Style::new().bg(palette::tile_bg(s.state))),
        plate,
    );
    let wi = plate.width.saturating_sub(2);
    let h = plate.height;
    let content = ladder::build_tile_content(s, wi, h, tick);
    if wi > 0 {
        let text_x = plate.x + 1;
        for (i, line) in content.lines.iter().enumerate() {
            if i as u16 >= h {
                break;
            }
            f.buffer_mut()
                .set_line(text_x, plate.y + i as u16, line, wi);
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

/// R9.2's third degrade step: the projects that didn't fit even as
/// summary tiles, shown as one small chip rather than vanishing.
fn draw_aggregate_chip(f: &mut Frame, agg: &AggregateChip) {
    // FALLBACK-OK: layout.md R9.2 — even the aggregate chip itself can, in
    // principle, be squeezed to zero size at extreme project counts; same
    // "leave it blank rather than assert" reasoning as `draw_tag_row`.
    if agg.plate.width == 0 || agg.plate.height == 0 {
        return;
    }
    f.render_widget(
        Block::new().style(Style::new().bg(palette::PLATE)),
        agg.plate,
    );
    let text = format!(
        "+{} projects ({} sessions)",
        agg.project_indices.len(),
        agg.session_count
    );
    let truncated = truncate_ellipsis(&text, agg.plate.width as usize);
    f.buffer_mut().set_string(
        agg.plate.x,
        agg.plate.y,
        &truncated,
        Style::new()
            .fg(palette::TEXT_DIM)
            .add_modifier(Modifier::BOLD),
    );
}

fn compute_header_counts(projects: &[ProjectView]) -> HeaderCounts {
    // Computed from the full project set, not from whatever regions ended
    // up drawn — the header/footer chrome reports total live state
    // regardless of how the body degrades (R9.2's hierarchy is a *body*
    // concern; a project folded into the aggregate chip is still live and
    // still counted here).
    let mut hc = HeaderCounts::default();
    for project in projects {
        hc.projects += 1;
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

fn hidden_projects(projects: &[ProjectView]) -> Vec<(String, usize)> {
    projects
        .iter()
        .filter(|p| p.is_all_idle())
        .map(|p| (p.name.clone(), p.sessions.len()))
        .collect()
}

fn draw_header(f: &mut Frame, rect: Rect, hc: &HeaderCounts, tick: usize) {
    // FALLBACK-OK: unreachable under the current call graph — `draw()`
    // only reaches this call after its own R9.1 check guarantees
    // `area.width >= 40`, and `rect.width` here is `area.width` unchanged.
    // Kept as a cheap guard (ported verbatim from the spike, which had the
    // same one) rather than an assertion, since a future caller change
    // that violates the invariant should degrade to "draw nothing" for a
    // header line, not crash the frame.
    if rect.width == 0 {
        return;
    }
    let wide = rect.width >= 100;
    let mut left: Vec<Span<'static>> = vec![Span::styled(
        format!(" {} opencode ", palette::header_glyph()),
        Style::new()
            .fg(palette::STATUS_RUNNING)
            .add_modifier(Modifier::BOLD),
    )];
    left.push(Span::raw(" · "));
    if wide {
        left.push(Span::styled(
            format!("{} projects · {} sessions", hc.projects, hc.sessions),
            Style::new().fg(palette::TEXT_DIM),
        ));
    } else {
        left.push(Span::styled(
            format!("{} sessions", hc.sessions),
            Style::new().fg(palette::TEXT_DIM),
        ));
    }
    if hc.q > 0 {
        left.push(Span::raw(" · "));
        left.push(Span::styled(
            format!("{} {}", palette::state_glyph(State::Question, tick), hc.q),
            Style::new()
                .fg(palette::STATUS_QUESTION)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if hc.need > 0 {
        left.push(Span::raw(" · "));
        left.push(Span::styled(
            format!(
                "{} {}",
                palette::state_glyph(State::NeedsYou, tick),
                hc.need
            ),
            Style::new()
                .fg(palette::STATUS_NEEDS_YOU)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if hc.run > 0 {
        left.push(Span::raw(" · "));
        left.push(Span::styled(
            format!("{} {}", palette::running_glyph(tick), hc.run),
            Style::new()
                .fg(palette::STATUS_RUNNING)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if hc.idle > 0 {
        left.push(Span::raw(" · "));
        left.push(Span::styled(
            format!("{} {}", palette::state_glyph(State::Idle, tick), hc.idle),
            Style::new()
                .fg(palette::TEXT_DIM)
                .add_modifier(Modifier::BOLD),
        ));
    }
    let left_line = Line::from(left);
    f.buffer_mut()
        .set_line(rect.x, rect.y, &left_line, rect.width);
}

fn draw_footer(
    f: &mut Frame,
    rect: Rect,
    hidden: &[(String, usize)],
    aggregate: Option<&AggregateReport>,
) {
    // FALLBACK-OK: same reasoning as `draw_header` above — unreachable
    // under the current call graph, kept as a cheap guard.
    if rect.width == 0 {
        return;
    }
    let left_text = "q quit  arrows/jk move  enter select  ] window  a all";
    let left_len = left_text.chars().count() as u16;
    f.buffer_mut().set_string(
        rect.x,
        rect.y,
        left_text,
        Style::new().fg(palette::TEXT_DIM),
    );

    let mut parts: Vec<String> = vec![];
    if !hidden.is_empty() {
        let joined = hidden
            .iter()
            .map(|(n, c)| format!("{n} ({c} idle)"))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("hidden: {joined}"));
    }
    if let Some(agg) = aggregate {
        parts.push(format!(
            "aggregated: {} projects ({} sessions)",
            agg.project_count, agg.session_count
        ));
    }
    if parts.is_empty() {
        return;
    }
    let text = parts.join("  ");
    let gap = 2u16;
    let right_budget = rect.width.saturating_sub(left_len + gap);
    if right_budget == 0 {
        return;
    }
    let truncated = truncate_ellipsis(&text, right_budget as usize);
    let tw = truncated.chars().count() as u16;
    if tw <= rect.width {
        let x = rect.x + rect.width - tw;
        f.buffer_mut()
            .set_string(x, rect.y, &truncated, Style::new().fg(palette::TEXT_DIM));
    }
}

/// R9.1: below the 40x12 minimum viewport, this is the only thing drawn.
fn draw_too_small(f: &mut Frame, area: Rect) {
    let msg = "terminal too small";
    // FALLBACK-OK: layout.md R9.1 — this path is *for* a viewport already
    // known to be below minimum size (could genuinely be 0 rows during a
    // resize race, since terminal dimensions are OS-provided); the rule
    // itself says "draw nothing else," and a message that can't fit is
    // exactly that.
    if area.height == 0 {
        return;
    }
    let msg_w = (msg.chars().count() as u16).min(area.width);
    let x = area.x + area.width.saturating_sub(msg_w) / 2;
    let y = area.y + area.height / 2;
    f.buffer_mut()
        .set_string(x, y, msg, Style::new().fg(palette::TEXT_PRIMARY));
}

/// R9: zero sessions active within the current window — a centered panel
/// instead of tiling idle sessions into tiny boxes, in place of the body's
/// project regions.
fn draw_empty_state(f: &mut Frame, area: Rect, window_minutes: u32, hidden_idle_count: usize) {
    // FALLBACK-OK: layout.md R9.2 — same "leave it blank rather than
    // assert" reasoning as the tile/tag/aggregate guards above; the body
    // area could in principle come in at 0 height even above the R9.1
    // viewport floor (e.g. an unusually short header/footer combination).
    if area.height == 0 {
        return;
    }
    let msg = format!(
        "No sessions updated in last {window_minutes}m — {hidden_idle_count} older sessions hidden — press ] or a"
    );
    let msg_w = (msg.chars().count() as u16).min(area.width);
    let truncated = truncate_ellipsis(&msg, msg_w as usize);
    let x = area.x + area.width.saturating_sub(msg_w) / 2;
    let y = area.y + area.height / 2;
    f.buffer_mut()
        .set_string(x, y, &truncated, Style::new().fg(palette::TEXT_PRIMARY));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mosaic::fixtures;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// Renders at `(w, h)` and returns the report plus the plain-text
    /// buffer contents (one string per row) — enough for both structural
    /// assertions (report fields) and text-content assertions (copy).
    /// Color assertions read `term`'s buffer directly instead, since a
    /// plain-text dump carries none (T11 contract's explicit note).
    fn render_at(
        sessions: &[SessionSnapshot],
        naming: &NamingClaimMap,
        now: Timestamp,
        window_minutes: u32,
        w: u16,
        h: u16,
    ) -> (DrawReport, Terminal<TestBackend>) {
        let mut term = Terminal::new(TestBackend::new(w, h)).expect("test backend");
        let mut report = None;
        term.draw(|f| {
            let area = f.area();
            report = Some(draw(f, area, sessions, naming, now, window_minutes));
        })
        .expect("draw");
        (report.expect("report set"), term)
    }

    fn row_text(term: &Terminal<TestBackend>, y: u16, w: u16) -> String {
        let buf = term.backend().buffer();
        (0..w).map(|x| buf[(x, y)].symbol()).collect()
    }

    #[test]
    fn viewport_below_40x12_draws_only_the_too_small_panel() {
        let (sessions, naming, now) = fixtures::design_center();
        let (report, term) = render_at(&sessions, &naming, now, 10, 35, 10);
        assert!(report.too_small);
        assert!(report.regions.is_empty());
        let full: String = (0..10)
            .map(|y| row_text(&term, y, 35))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(full.contains("terminal too small"), "buffer: {full}");
    }

    #[test]
    fn r9_zero_active_shows_the_exact_copy_pattern() {
        let (sessions, naming, now) = fixtures::zero_active();
        let (report, term) = render_at(&sessions, &naming, now, 10, 150, 42);
        assert!(!report.too_small);
        assert!(report.regions.is_empty());
        let empty_state = report
            .empty_state
            .expect("R9 must fire when every session is idle");
        assert_eq!(empty_state.window_minutes, 10);
        assert_eq!(
            empty_state.hidden_idle_count, 6,
            "all 6 top-level sessions in the fixture are idle"
        );

        let full: String = (0..42)
            .map(|y| row_text(&term, y, 150))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            full.contains(
                "No sessions updated in last 10m — 6 older sessions hidden — press ] or a"
            ),
            "buffer did not contain the exact R9 copy pattern: {full}"
        );
    }

    #[test]
    fn design_center_packs_all_active_projects_and_hides_the_idle_only_one() {
        let (sessions, naming, now) = fixtures::design_center();
        let (report, _term) = render_at(&sessions, &naming, now, 10, 150, 42);
        assert!(!report.too_small);
        assert!(report.empty_state.is_none());
        // 4 active projects (web-dashboard, infra-tools, mobile-app,
        // scratch-cli); docs-site is idle-only and excluded from packing.
        assert_eq!(report.regions.len(), 4);
        assert_eq!(report.hidden_projects, vec![("docs-site".to_string(), 1)]);
        let total_tiles: usize = report.regions.iter().map(|r| r.tiles.len()).sum();
        assert!(
            total_tiles > 0,
            "design center must actually draw session tiles"
        );
    }

    #[test]
    fn single_low_weight_project_gets_no_sliver() {
        let (sessions, naming, now) = fixtures::single_low_weight_project();
        let (report, _term) = render_at(&sessions, &naming, now, 10, 150, 42);
        assert_eq!(report.regions.len(), 2);
        for r in &report.regions {
            // Every drawn region must clear the R5.5 region minimum or
            // have degraded to a Summary tile deliberately — never sit at
            // an in-between unreadable sliver size.
            let cleared_full_minimum =
                r.raw.width >= layout::REGION_MIN_W && r.raw.height >= layout::REGION_MIN_H;
            assert!(
                cleared_full_minimum || r.kind == RegionKind::Summary,
                "region {} at {:?} is neither full-size nor a summary tile",
                r.project_name,
                r.raw
            );
        }
    }

    #[test]
    fn project_accent_never_appears_on_a_tile_background() {
        let (sessions, naming, now) = fixtures::design_center();
        let (report, term) = render_at(&sessions, &naming, now, 10, 150, 42);
        let buf = term.backend().buffer();

        let accents: Vec<Color> = (0..report.regions.len())
            .map(palette::project_color)
            .collect();
        let tile_bgs = [
            palette::TILE_BG_QUESTION,
            palette::TILE_BG_NEEDS_YOU,
            palette::TILE_BG_RUNNING,
            palette::TILE_BG_IDLE,
        ];

        for region in &report.regions {
            for tile in &region.tiles {
                if tile.dropped {
                    continue;
                }
                for y in tile.raw.y..(tile.raw.y + tile.raw.height).min(42) {
                    for x in tile.raw.x..(tile.raw.x + tile.raw.width).min(150) {
                        let bg = buf[(x, y)].bg;
                        assert!(
                            !accents.contains(&bg) || tile_bgs.contains(&bg),
                            "a project accent color leaked onto a tile background at ({x},{y})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn tag_row_project_name_is_colored_with_that_projects_accent() {
        let (sessions, naming, now) = fixtures::design_center();
        let (report, term) = render_at(&sessions, &naming, now, 10, 150, 42);
        let buf = term.backend().buffer();

        for region in &report.regions {
            let expected = palette::project_color(region.project_idx);
            let tag_y = region.plate.y;
            // The tag badge starts one cell in from the plate's left edge
            // (draw_tag_row's leading `" {name} "` badge text).
            let cell = &buf[(region.plate.x, tag_y)];
            assert_eq!(
                cell.bg, expected,
                "region {}'s tag badge background must carry its own project accent",
                region.project_name
            );
        }
    }

    #[test]
    fn running_tile_background_matches_the_running_state_color() {
        let (sessions, naming, now) = fixtures::design_center();
        let (report, term) = render_at(&sessions, &naming, now, 10, 150, 42);
        let buf = term.backend().buffer();

        let mut checked_any = false;
        for region in &report.regions {
            for tile in &region.tiles {
                if tile.dropped {
                    continue;
                }
                // wd-2 (session index into the project's sessions vec is
                // opaque here, so key off the nickname captured in the
                // report instead) is a `running` session in the fixture.
                if tile.regime != "dropped" && tile.h > 0 {
                    let cell = &buf[(tile.raw.x, tile.raw.y)];
                    // Every drawn tile's plate must be one of the four
                    // known state backgrounds — never left at the plain
                    // gutter/plate color, and never a project accent.
                    let known = [
                        palette::TILE_BG_QUESTION,
                        palette::TILE_BG_NEEDS_YOU,
                        palette::TILE_BG_RUNNING,
                        palette::TILE_BG_IDLE,
                    ];
                    assert!(
                        known.contains(&cell.bg),
                        "tile {} bg {:?} is not a known state color",
                        tile.session_nick,
                        cell.bg
                    );
                    checked_any = true;
                }
            }
        }
        assert!(checked_any, "must have inspected at least one drawn tile");
    }

    #[test]
    fn aggregate_chip_fires_when_too_many_projects_for_the_width() {
        let now = Timestamp::from_epoch_millis(0);
        let mut sessions = vec![];
        let mut naming = NamingClaimMap::new();
        let mut live = vec![];
        for i in 0..30 {
            let project = crate::snapshot::ProjectId::from_canonical(std::path::PathBuf::from(
                format!("/tmp/p{i}"),
            ));
            let session = crate::snapshot::SessionId::new(
                crate::snapshot::HarnessKind("fixture"),
                format!("s{i}"),
            );
            sessions.push(SessionSnapshot {
                session_id: session.clone(),
                project_id: project.clone(),
                parent_id: None,
                attention: crate::snapshot::AttentionState::Running { turn_started: now },
                current_action: None,
                wire_title: None,
                final_assistant_text: None,
                last_user_prompt: None,
                files_touched: vec![],
                recent_actions: vec![],
                created_at: now,
                last_updated: now,
            });
            live.push(crate::naming::LiveSession {
                project_id: project,
                session_id: session,
                created_at: now,
            });
        }
        naming.claim_batch(live);

        let (report, _term) = render_at(&sessions, &naming, now, 10, 40, 12);
        let aggregate = report
            .aggregate
            .expect("30 projects in a 40-wide viewport must aggregate, not vanish");
        let accounted = report.regions.len() + aggregate.project_count;
        assert_eq!(
            accounted, 30,
            "every project is either a region or counted in the aggregate chip"
        );
    }
}
