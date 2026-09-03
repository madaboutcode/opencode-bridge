// Geometry only: project regions (one squarify call, 2:1 virtual space) and, inside each
// drawn region, active-session tiles (a second squarify call) plus the idle chip row.
// No drawing, no text — see render.rs / ladder.rs for that. BRIEF-v2.md "Geometry".

use ratatui::layout::Rect;

use crate::fixture::{Fixture, Session, State};
use crate::squarify::{self, TreemapItem};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RegionKind {
    NotDrawn,
    TagOnly,
    TagAndTiles,
}

#[derive(Clone, Debug)]
pub struct RegionLayout {
    pub project_idx: usize,
    /// pre-inset allocated rect, straight from squarify + edge snap.
    pub raw: Rect,
    /// post-inset drawn rect (1 col right / 1 row bottom gutter). Zero-sized if not drawn.
    pub plate: Rect,
    pub weight: f64,
    pub kind: RegionKind,
}

impl RegionLayout {
    /// max(w, 2h) / min(w, 2h), using the raw (pre-inset) allocation — the number the
    /// report tracks, since it measures what squarify actually gave the project, not the
    /// cosmetic gutter trim.
    pub fn aspect_ratio(&self) -> f64 {
        let w = self.raw.width as f64;
        let h = (self.raw.height as f64) * 2.0;
        if w <= 0.0 || h <= 0.0 {
            return f64::INFINITY;
        }
        w.max(h) / w.min(h)
    }
}

#[derive(Clone, Debug)]
pub struct TileLayout {
    pub session_idx: usize,
    pub raw: Rect,
    /// post-inset content rect (1 col right / 1 row bottom). Zero-sized if degenerate.
    pub plate: Rect,
    pub weight: f64,
}

#[derive(Clone, Debug)]
pub struct RegionContent {
    pub region: RegionLayout,
    pub tag_rect: Rect,
    pub idle_row_rect: Option<Rect>,
    pub tiles: Vec<TileLayout>,
    /// idle sessions for this project, most-recent-first, whether or not the idle row is
    /// drawn (render decides how many chips fit).
    pub idle_sessions_sorted: Vec<usize>,
}

/// Runs squarify in a height-doubled virtual space (so rows come out visually square: a
/// terminal cell is roughly 2 rows tall per column-wide), then snaps back by rounding
/// *edges* — not sizes — so adjacent regions share exact boundaries with no gap or
/// overlap. squarify.rs itself is untouched; it already returns integer-cell rects in
/// whatever space it's given, so the "round(x)" step in the brief is a no-op on x and
/// does the real work on y (dividing the doubled space back down by 2).
fn squarify_2to1(items: &[(usize, f64)], area: Rect) -> Vec<(usize, Rect)> {
    if items.is_empty() || area.width == 0 || area.height == 0 {
        return vec![];
    }
    let virtual_rect = Rect { x: 0, y: 0, width: area.width, height: area.height.saturating_mul(2) };
    let treemap_items: Vec<TreemapItem> = items
        .iter()
        .map(|(idx, w)| TreemapItem { id: idx.to_string(), weight: *w, label: String::new() })
        .collect();
    let raw = squarify::squarify(&treemap_items, virtual_rect);
    raw.into_iter()
        .filter_map(|(item_i, r)| {
            let orig_idx = items[item_i].0;
            let x0 = r.x;
            let x1 = r.x + r.width;
            let y0 = ((r.y as f64) / 2.0).round() as u16;
            let y1 = (((r.y + r.height) as f64) / 2.0).round() as u16;
            if x1 <= x0 || y1 <= y0 {
                return None;
            }
            let rect = Rect { x: area.x + x0, y: area.y + y0, width: x1 - x0, height: y1 - y0 };
            Some((orig_idx, rect))
        })
        .collect()
}

/// Sort key for tiling: question -> needs-you (longest wait first) -> running -> idle.
/// Question is sorted by wait too (not specified either way by the brief; the REAL/STRESS
/// fixtures never have two questions in one project so this is untestable, but sorting it
/// the same way as needs-you is the least surprising extension).
fn session_order_key(s: &Session) -> (u8, i64) {
    match s.state {
        State::Question => (0, -(s.wait_m.unwrap_or(0) as i64)),
        State::NeedsYou => (1, -(s.wait_m.unwrap_or(0) as i64)),
        State::Running => (2, 0),
        State::Idle => (3, 0),
    }
}

/// Content-demand weight (R6.1: status is colour/glyph/order, never geometry).
fn tile_weight(s: &Session) -> f64 {
    let base = if s.state == State::Idle { 1.0 } else { 2.0 };
    base + s.subs.len() as f64
}

fn inset(r: Rect) -> Rect {
    Rect { x: r.x, y: r.y, width: r.width.saturating_sub(1), height: r.height.saturating_sub(1) }
}

/// Lays out every drawn/not-drawn project region and, for TagAndTiles regions, the tiles
/// and idle row within it. `body` is rows 1..H-2 (header/footer already excluded).
pub fn layout_body(fixture: &Fixture, body: Rect) -> Vec<RegionContent> {
    let region_items: Vec<(usize, f64)> = fixture
        .projects
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.is_all_idle())
        .map(|(idx, p)| (idx, p.sessions.len() as f64))
        .collect();

    let mut raw_regions = squarify_2to1(&region_items, body);
    raw_regions.sort_by_key(|(idx, _)| *idx);

    let weight_of = |idx: usize| -> f64 {
        region_items.iter().find(|(i, _)| *i == idx).map(|(_, w)| *w).unwrap_or(0.0)
    };

    raw_regions
        .into_iter()
        .map(|(project_idx, raw)| {
            let plate = inset(raw);
            let kind = if plate.width < 6 || plate.height < 1 {
                RegionKind::NotDrawn
            } else if plate.height == 1 {
                RegionKind::TagOnly
            } else {
                RegionKind::TagAndTiles
            };
            let region = RegionLayout { project_idx, raw, plate, weight: weight_of(project_idx), kind };
            build_region_content(fixture, region)
        })
        .collect()
}

fn build_region_content(fixture: &Fixture, region: RegionLayout) -> RegionContent {
    let plate = region.plate;
    if region.kind == RegionKind::NotDrawn {
        return RegionContent {
            region,
            tag_rect: Rect::default(),
            idle_row_rect: None,
            tiles: vec![],
            idle_sessions_sorted: vec![],
        };
    }

    let tag_rect = Rect { x: plate.x, y: plate.y, width: plate.width, height: 1 };

    let project = &fixture.projects[region.project_idx];
    let mut idle_sorted: Vec<usize> = project
        .sessions
        .iter()
        .enumerate()
        .filter(|(_, s)| s.state == State::Idle)
        .map(|(i, _)| i)
        .collect();
    idle_sorted.sort_by_key(|&i| project.sessions[i].age_secs());

    if region.kind == RegionKind::TagOnly {
        return RegionContent { region, tag_rect, idle_row_rect: None, tiles: vec![], idle_sessions_sorted: idle_sorted };
    }

    let ph = plate.height;
    let idle_row_reserved = ph >= 3 && !idle_sorted.is_empty();
    let tile_rows = if idle_row_reserved { ph - 2 } else { ph - 1 };

    let idle_row_rect = if idle_row_reserved {
        Some(Rect { x: plate.x, y: plate.y + ph - 1, width: plate.width, height: 1 })
    } else {
        None
    };
    let tile_area = Rect { x: plate.x, y: plate.y + 1, width: plate.width, height: tile_rows };

    let mut active: Vec<(usize, &Session)> = project
        .sessions
        .iter()
        .enumerate()
        .filter(|(_, s)| s.state != State::Idle)
        .collect();
    active.sort_by_key(|(_, s)| session_order_key(s));

    let tile_items: Vec<(usize, f64)> = active.iter().map(|(i, s)| (*i, tile_weight(s))).collect();
    let raw_tiles = squarify_2to1(&tile_items, tile_area);

    let weight_of = |idx: usize| -> f64 {
        tile_items.iter().find(|(i, _)| *i == idx).map(|(_, w)| *w).unwrap_or(0.0)
    };

    let mut tiles: Vec<TileLayout> = raw_tiles
        .into_iter()
        .map(|(session_idx, raw)| TileLayout { session_idx, raw, plate: inset(raw), weight: weight_of(session_idx) })
        .collect();
    tiles.sort_by_key(|t| t.session_idx);

    RegionContent { region, tag_rect, idle_row_rect, tiles, idle_sessions_sorted: idle_sorted }
}
