//! Geometry only: project regions (one squarify call, 2:1 virtual space)
//! and, inside each drawn region, active-session tiles (a second squarify
//! call) plus the idle chip row. No drawing, no text — see `render.rs` /
//! `ladder.rs` for that. `layout.md` R5-R5.11.
//!
//! Ported from the verified spike
//! (`tmp/20260901-prototype-dashboard-layout/src/layout.rs`) and adapted to
//! read `crate::mosaic::view::{ProjectView, SessionView}` instead of the
//! spike's fixture types. Two behaviors the spike didn't implement are
//! added here, per the T11 contract's instruction to "fill in whatever the
//! spike doesn't already cover" against R9/R5.5/R5.6's exact wording:
//!
//! 1. **R5.5's 14x7 region-minimum threshold.** The spike's own
//!    `RegionKind` split only ever checked `plate.height == 1` (a 1-row
//!    sliver) — it never checked the spec's actual 14x7 number, so a
//!    region sized e.g. 14x6 would still get full tiles, not a
//!    project-summary tile. `RegionKind::Summary` below is gated on the
//!    real threshold.
//! 2. **R5.6's 3-tile-per-project cap and overflow chip.** The spike's
//!    `build_region_content` fed every active session into squarify with
//!    no cap at all. `MAX_TILES_PER_PROJECT` below caps it and folds the
//!    rest into the same bottom-row mechanism the spike already had for
//!    idle chips (see `RegionContent::bottom_row_rect`'s doc comment for
//!    why sharing that mechanism, rather than inventing new geometry, was
//!    the chosen design).
//!
//! A third gap — R9.2's "aggregated `+N projects` chip" degrade step for
//! when even project-summary tiles don't all fit — is also new here (the
//! spike had no third degrade step at all: an undersized region just
//! silently vanished). See [`layout_body`]'s doc comment.

use ratatui::layout::Rect;

use crate::mosaic::squarify::{self, TreemapItem};
use crate::mosaic::view::{ProjectView, SessionView, State};

/// `layout.md` R5.5.
pub const TILE_MIN_W: u16 = 12;
pub const TILE_MIN_H: u16 = 5;
pub const REGION_MIN_W: u16 = 14;
pub const REGION_MIN_H: u16 = 7;
pub const VIEWPORT_MIN_W: u16 = 40;
pub const VIEWPORT_MIN_H: u16 = 12;

/// The floor below which even a project-summary tag can't render at all —
/// where R9.2's third degrade step (the aggregated `+N projects` chip)
/// takes over from the second (project-summary tile). Measured on the
/// *raw* (pre-inset) allocation, matching the "don't draw a sliver" floor
/// the spike already used for its own not-drawn check (`plate.width < 6`,
/// i.e. `raw.width < 7` once the 1-cell inset is accounted for) — this
/// port routes what the spike silently dropped at that floor into the
/// aggregate chip instead of vanishing it.
const SUMMARY_FLOOR_W: u16 = 7;
const SUMMARY_FLOOR_H: u16 = 2;

/// `layout.md` R5.6.
const MAX_TILES_PER_PROJECT: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RegionKind {
    /// Below R5.5's 14x7 region minimum: name + status counts only, no
    /// individual session tiles (R9.2's second degrade step).
    Summary,
    /// Full tag row + session tiles + idle/overflow row.
    Full,
}

#[derive(Clone, Debug)]
pub struct RegionLayout {
    pub project_idx: usize,
    /// pre-inset allocated rect, straight from squarify + edge snap.
    pub raw: Rect,
    /// post-inset drawn rect (1 col right / 1 row bottom gutter).
    pub plate: Rect,
    pub weight: f64,
    pub kind: RegionKind,
}

impl RegionLayout {
    /// max(w, 2h) / min(w, 2h), using the raw (pre-inset) allocation — the
    /// number evidence reports track, since it measures what squarify
    /// actually gave the project, not the cosmetic gutter trim.
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
    /// The region's bottom row, shared by two R5.2/R5.6 concerns: idle
    /// chips (`○ nick · age`) and, when this project had more than
    /// [`MAX_TILES_PER_PROJECT`] active sessions, the R5.6 overflow chip
    /// (`+N sessions`) appended after them. The spec gives the idle row an
    /// exact height-threshold rule (R5.2: `h>=3` shows it, `h==2` folds
    /// into the tag row, `h==1` tag-row-only) but says nothing about a
    /// *separate* geometry for the overflow chip beyond "a small,
    /// unweighted... chip" — reusing the idle row's already-specified
    /// slot, rather than inventing new geometry the spec doesn't
    /// describe, is this port's chosen reading (flagged in the T11 report
    /// as a within-spec judgment call, not a literal rule).
    pub bottom_row_rect: Option<Rect>,
    pub tiles: Vec<TileLayout>,
    /// idle sessions for this project, most-recent-first, whether or not
    /// the bottom row is drawn (render decides how many chips fit).
    pub idle_sessions_sorted: Vec<usize>,
    /// Active sessions beyond the R5.6 cap, in the same priority order
    /// they were dropped from — render turns this into the `+N sessions`
    /// chip text.
    pub overflow_count: usize,
}

/// R9.2's third degrade step: a project whose squarify allocation came out
/// below even the project-summary floor gets folded into this single
/// aggregate chip instead of drawing an unreadable sliver or silently
/// vanishing (the spike's own gap — see this module's doc comment).
#[derive(Clone, Debug)]
pub struct AggregateChip {
    pub raw: Rect,
    pub plate: Rect,
    pub project_indices: Vec<usize>,
    pub session_count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct BodyLayout {
    pub regions: Vec<RegionContent>,
    pub aggregate: Option<AggregateChip>,
}

/// Runs squarify in a height-doubled virtual space (so rows come out
/// visually square: a terminal cell is roughly 2 rows tall per
/// column-wide), then snaps back by rounding *edges* — not sizes — so
/// adjacent regions share exact boundaries with no gap or overlap.
/// `squarify.rs` itself is untouched; the "round(x)" step is a no-op on x
/// and does the real work on y (dividing the doubled space back down by
/// 2).
fn squarify_2to1(items: &[(usize, f64)], area: Rect) -> Vec<(usize, Rect)> {
    if items.is_empty() || area.width == 0 || area.height == 0 {
        return vec![];
    }
    let virtual_rect = Rect {
        x: 0,
        y: 0,
        width: area.width,
        height: area.height.saturating_mul(2),
    };
    let treemap_items: Vec<TreemapItem> = items
        .iter()
        .map(|(idx, w)| TreemapItem {
            id: idx.to_string(),
            weight: *w,
        })
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
            let rect = Rect {
                x: area.x + x0,
                y: area.y + y0,
                width: x1 - x0,
                height: y1 - y0,
            };
            Some((orig_idx, rect))
        })
        .collect()
}

/// Sort key for tiling: question -> needs-you (longest wait first) -> running -> idle.
fn session_order_key(s: &SessionView) -> (u8, i64) {
    match s.state {
        State::Question => (0, -(s.wait_m.unwrap_or(0) as i64)),
        State::NeedsYou => (1, -(s.wait_m.unwrap_or(0) as i64)),
        State::Running => (2, 0),
        State::Idle => (3, 0),
    }
}

/// Content-demand weight (`visuals.md` R6.1: status is colour/glyph/order, never geometry).
fn tile_weight(s: &SessionView) -> f64 {
    let base = if s.state == State::Idle { 1.0 } else { 2.0 };
    base + s.subs.len() as f64
}

fn inset(r: Rect) -> Rect {
    Rect {
        x: r.x,
        y: r.y,
        width: r.width.saturating_sub(1),
        height: r.height.saturating_sub(1),
    }
}

/// Lays out every project region — and, per R9.2, the aggregate chip for
/// any that don't fit even as a summary — plus tiles/idle row within each
/// `Full`-kind region. `body` is the area below the header and above the
/// footer.
pub fn layout_body(projects: &[ProjectView], body: Rect) -> BodyLayout {
    let region_items: Vec<(usize, f64)> = projects
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.is_all_idle())
        .map(|(idx, p)| (idx, p.sessions.len() as f64))
        .collect();

    if region_items.is_empty() {
        return BodyLayout::default();
    }

    let weight_of = |idx: usize| -> f64 {
        region_items
            .iter()
            .find(|(i, _)| *i == idx)
            .map(|(_, w)| *w)
            .unwrap_or(0.0)
    };

    let mut raw_regions = squarify_2to1(&region_items, body);
    raw_regions.sort_by_key(|(idx, _)| *idx);

    let (fits, undersized): (Vec<_>, Vec<_>) = raw_regions
        .into_iter()
        .partition(|(_, r)| r.width >= SUMMARY_FLOOR_W && r.height >= SUMMARY_FLOOR_H);

    if undersized.is_empty() {
        let regions = fits
            .into_iter()
            .map(|(idx, raw)| build_region(projects, idx, raw, weight_of(idx)))
            .collect();
        return BodyLayout {
            regions,
            aggregate: None,
        };
    }

    // R9.2 third degrade step: re-run squarify with the projects that DID
    // fit at their own weight, plus one synthetic aggregate item whose
    // weight is the summed weight of the projects that didn't — so the
    // aggregate chip gets a proportional slice instead of either stealing
    // space from projects that fit or being computed from a stale first
    // pass. Bounded to one re-pass (not recursive): at this scale
    // (`overview.md` R5.8, design center ~4 projects) this path is a
    // stress-only edge case, and a single re-pass already satisfies
    // R9.2's priority order (readability, then project presence, then
    // proportionality, then filling the screen) — iterating further would
    // be tuning a path the delivery profile explicitly treats as
    // secondary, not the design point.
    const AGGREGATE_MARKER: usize = usize::MAX;
    let agg_weight: f64 = undersized.iter().map(|(idx, _)| weight_of(*idx)).sum();
    let mut items2: Vec<(usize, f64)> = fits
        .iter()
        .map(|(idx, _)| (*idx, weight_of(*idx)))
        .collect();
    items2.push((AGGREGATE_MARKER, agg_weight));

    let raw2 = squarify_2to1(&items2, body);
    let mut regions = Vec::new();
    let mut aggregate = None;
    for (idx, raw) in raw2 {
        if idx == AGGREGATE_MARKER {
            let plate = inset(raw);
            let project_indices: Vec<usize> = undersized.iter().map(|(i, _)| *i).collect();
            let session_count: usize = project_indices
                .iter()
                .map(|&i| projects[i].sessions.len())
                .sum();
            aggregate = Some(AggregateChip {
                raw,
                plate,
                project_indices,
                session_count,
            });
        } else {
            regions.push(build_region(projects, idx, raw, weight_of(idx)));
        }
    }
    regions.sort_by_key(|r| r.region.project_idx);
    BodyLayout { regions, aggregate }
}

fn build_region(
    projects: &[ProjectView],
    project_idx: usize,
    raw: Rect,
    weight: f64,
) -> RegionContent {
    let plate = inset(raw);
    // R5.5: "a project region needs at least 14x7 cells" — read as the
    // outer/raw allocation, paralleling how the same rule phrases the tile
    // minimum ("12x5 cells outer, 10x3 usable after inset").
    let kind = if raw.width < REGION_MIN_W || raw.height < REGION_MIN_H {
        RegionKind::Summary
    } else {
        RegionKind::Full
    };
    let region = RegionLayout {
        project_idx,
        raw,
        plate,
        weight,
        kind,
    };

    let tag_rect = Rect {
        x: plate.x,
        y: plate.y,
        width: plate.width,
        height: 1,
    };

    let project = &projects[project_idx];
    let mut idle_sorted: Vec<usize> = project
        .sessions
        .iter()
        .enumerate()
        .filter(|(_, s)| s.state == State::Idle)
        .map(|(i, _)| i)
        .collect();
    idle_sorted.sort_by_key(|&i| project.sessions[i].age_secs);

    if kind == RegionKind::Summary {
        return RegionContent {
            region,
            tag_rect,
            bottom_row_rect: None,
            tiles: vec![],
            idle_sessions_sorted: idle_sorted,
            overflow_count: 0,
        };
    }

    let mut active: Vec<(usize, &SessionView)> = project
        .sessions
        .iter()
        .enumerate()
        .filter(|(_, s)| s.state != State::Idle)
        .collect();
    active.sort_by_key(|(_, s)| session_order_key(s));

    // R5.6: at most 3 tiles per project; the rest collapse into the
    // overflow chip instead of being tiled.
    let overflow_count = active.len().saturating_sub(MAX_TILES_PER_PROJECT);
    active.truncate(MAX_TILES_PER_PROJECT);

    let ph = plate.height;
    let bottom_row_needed = ph >= 3 && (!idle_sorted.is_empty() || overflow_count > 0);
    let tile_rows = if bottom_row_needed { ph - 2 } else { ph - 1 };

    let bottom_row_rect = if bottom_row_needed {
        Some(Rect {
            x: plate.x,
            y: plate.y + ph - 1,
            width: plate.width,
            height: 1,
        })
    } else {
        None
    };
    let tile_area = Rect {
        x: plate.x,
        y: plate.y + 1,
        width: plate.width,
        height: tile_rows,
    };

    let tile_items: Vec<(usize, f64)> = active.iter().map(|(i, s)| (*i, tile_weight(s))).collect();
    let raw_tiles = squarify_2to1(&tile_items, tile_area);

    let weight_of = |idx: usize| -> f64 {
        tile_items
            .iter()
            .find(|(i, _)| *i == idx)
            .map(|(_, w)| *w)
            .unwrap_or(0.0)
    };

    let mut tiles: Vec<TileLayout> = raw_tiles
        .into_iter()
        .map(|(session_idx, raw)| TileLayout {
            session_idx,
            raw,
            plate: inset(raw),
            weight: weight_of(session_idx),
        })
        .collect();
    tiles.sort_by_key(|t| t.session_idx);

    RegionContent {
        region,
        tag_rect,
        bottom_row_rect,
        tiles,
        idle_sessions_sorted: idle_sorted,
        overflow_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::{ProjectId, SessionId};
    use std::path::PathBuf;

    fn project_with(name: &str, sessions: Vec<SessionView>) -> ProjectView {
        ProjectView {
            project_id: ProjectId::from_canonical(PathBuf::from(format!("/tmp/{name}"))),
            name: name.to_string(),
            sessions,
        }
    }

    fn session(state: State, wait_m: Option<u32>) -> SessionView {
        SessionView {
            session_id: SessionId::new(crate::snapshot::HarnessKind("test"), "s"),
            nick: "nick".into(),
            title: "title".into(),
            state,
            age: "1m".into(),
            age_secs: 60,
            wait_m,
            action: None,
            subs: vec![],
            recent: vec![],
            files: vec![],
            assistant_text: String::new(),
            user_prompt: String::new(),
        }
    }

    #[test]
    fn region_below_14x7_collapses_to_summary_not_full() {
        // A single project so it gets the whole body area; sized to land
        // well under 14x7 after inset.
        let project = project_with("solo", vec![session(State::Running, None)]);
        let body = Rect::new(0, 0, 12, 6);
        let layout = layout_body(&[project], body);
        assert_eq!(layout.regions.len(), 1);
        assert_eq!(layout.regions[0].region.kind, RegionKind::Summary);
        assert!(
            layout.regions[0].tiles.is_empty(),
            "summary regions draw no tiles"
        );
    }

    #[test]
    fn region_at_or_above_14x7_gets_full_tiles() {
        let project = project_with("solo", vec![session(State::Running, None)]);
        let body = Rect::new(0, 0, 20, 10);
        let layout = layout_body(&[project], body);
        assert_eq!(layout.regions.len(), 1);
        assert_eq!(layout.regions[0].region.kind, RegionKind::Full);
        assert_eq!(layout.regions[0].tiles.len(), 1);
    }

    #[test]
    fn more_than_three_active_sessions_caps_tiles_and_reports_overflow() {
        let sessions = vec![
            session(State::Question, Some(1)),
            session(State::NeedsYou, Some(20)),
            session(State::NeedsYou, Some(5)),
            session(State::Running, None),
            session(State::Running, None),
        ];
        let project = project_with("busy", sessions);
        let body = Rect::new(0, 0, 80, 24);
        let layout = layout_body(&[project], body);
        assert_eq!(layout.regions.len(), 1);
        let region = &layout.regions[0];
        assert_eq!(region.tiles.len(), 3, "at most 3 tiles per project (R5.6)");
        assert_eq!(region.overflow_count, 2);
    }

    #[test]
    fn tile_cap_priority_is_question_then_longest_wait_then_running() {
        let sessions = vec![
            session(State::Running, None),      // idx 0
            session(State::NeedsYou, Some(5)),  // idx 1
            session(State::Question, Some(1)),  // idx 2
            session(State::NeedsYou, Some(20)), // idx 3
            session(State::Running, None),      // idx 4
        ];
        let project = project_with("busy", sessions);
        let body = Rect::new(0, 0, 80, 24);
        let layout = layout_body(&[project], body);
        let mut shown: Vec<usize> = layout.regions[0]
            .tiles
            .iter()
            .map(|t| t.session_idx)
            .collect();
        shown.sort();
        // question(2), needs-you-20m(3), needs-you-5m(1) beat both running sessions.
        assert_eq!(shown, vec![1, 2, 3]);
    }

    #[test]
    fn idle_only_project_excluded_from_packing() {
        let sessions = vec![session(State::Idle, None)];
        let project = project_with("all-idle", sessions);
        let body = Rect::new(0, 0, 80, 24);
        let layout = layout_body(&[project], body);
        assert!(layout.regions.is_empty());
        assert!(layout.aggregate.is_none());
    }

    #[test]
    fn projects_packed_in_first_appearance_order_never_resorted_by_weight() {
        let heavy = project_with(
            "heavy",
            vec![
                session(State::Running, None),
                session(State::Running, None),
                session(State::Running, None),
            ],
        );
        let light = project_with("light", vec![session(State::Running, None)]);
        let body = Rect::new(0, 0, 80, 24);
        let layout = layout_body(&[light, heavy], body);
        // "light" was index 0 in the input despite lower weight; it must
        // still be region.project_idx == 0, not resorted after "heavy".
        let light_region = layout
            .regions
            .iter()
            .find(|r| r.region.project_idx == 0)
            .unwrap();
        assert_eq!(light_region.tiles.len(), 1);
    }

    #[test]
    fn too_many_projects_for_the_width_aggregate_instead_of_vanishing() {
        // 30 tiny single-session projects in a narrow terminal: squarify
        // cannot give every one of them even a 6x1 summary slot. None may
        // silently disappear — R9.2 requires they show up in the
        // aggregate chip instead.
        let projects: Vec<ProjectView> = (0..30)
            .map(|i| project_with(&format!("p{i}"), vec![session(State::Running, None)]))
            .collect();
        let body = Rect::new(0, 0, 40, 12);
        let layout = layout_body(&projects, body);
        let aggregate = layout
            .aggregate
            .expect("some projects must not fit and must aggregate");
        let accounted: usize = layout.regions.len() + aggregate.project_indices.len();
        assert_eq!(
            accounted, 30,
            "every project is either a region or in the aggregate — none vanish"
        );
    }
}
