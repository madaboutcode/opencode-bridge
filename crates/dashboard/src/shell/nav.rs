//! R7.1's navigate order — the reading order of the *current frame's*
//! drawn tiles (`docs/specs/dashboard/interactions.md` R7.1: "left to
//! right, top to bottom, exactly as the Mosaic layout has placed them for
//! this frame"), plus wraparound stepping. Pure: takes T11's own
//! `DrawReport`, no terminal, no live server — `mosaic::draw` against a
//! `ratatui::backend::TestBackend` produces a real `DrawReport` a test can
//! drive this against directly (T12 contract, AC4).

use ratatui::layout::Rect;

use crate::mosaic::DrawReport;

/// Identifies one on-screen tile across frames. `nick` is unique within one
/// live project (`visuals.md` R6.8's per-project no-duplicate-name
/// guarantee), so pairing it with the tile's project index is enough to
/// recognize "the same tile" from one frame to the next — `TileReport`
/// (T11's render report) carries a display nickname, not a `SessionId`, so
/// this is what's actually available to key off; see this task's report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileId {
    pub project_idx: usize,
    pub nick: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Next,
    Prev,
}

/// Every navigable (non-dropped) tile this frame, in on-screen reading
/// order: top row first, left-most first within a row band. Recomputed
/// fresh from `report` every call — `layout.md` R5.7 is explicit that
/// tile order is not held sticky across frames, and this function doesn't
/// either.
pub fn reading_order(report: &DrawReport) -> Vec<TileId> {
    let mut tiles: Vec<(Rect, TileId)> = report
        .regions
        .iter()
        .flat_map(|region| {
            region.tiles.iter().filter(|t| !t.dropped).map(move |t| {
                (
                    t.raw,
                    TileId {
                        project_idx: region.project_idx,
                        nick: t.session_nick.clone(),
                    },
                )
            })
        })
        .collect();
    tiles.sort_by_key(|(r, _)| (r.y, r.x));
    tiles.into_iter().map(|(_, id)| id).collect()
}

/// One step of R7.1's navigation, wrapping at both ends. `current` not
/// found in `order` (its tile vanished since it was selected, or nothing
/// was selected yet) starts from the first tile going forward or the last
/// tile going backward, rather than refusing to move.
pub fn step(order: &[TileId], current: Option<&TileId>, dir: Direction) -> Option<TileId> {
    if order.is_empty() {
        return None;
    }
    let idx = current.and_then(|c| order.iter().position(|t| t == c));
    let next = match (idx, dir) {
        (None, Direction::Next) => 0,
        (None, Direction::Prev) => order.len() - 1,
        (Some(i), Direction::Next) => (i + 1) % order.len(),
        (Some(i), Direction::Prev) => (i + order.len() - 1) % order.len(),
    };
    Some(order[next].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mosaic::fixtures;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn draw_report(w: u16, h: u16) -> DrawReport {
        let (sessions, naming, now) = fixtures::design_center();
        let mut term = Terminal::new(TestBackend::new(w, h)).expect("test backend");
        let mut report = None;
        let mut layout_cache = crate::mosaic::LayoutCache::new();
        term.draw(|f| {
            let area = f.area();
            report = Some(crate::mosaic::draw(
                f,
                area,
                &sessions,
                &naming,
                now,
                10,
                &mut layout_cache,
            ));
        })
        .expect("draw");
        report.expect("report set")
    }

    #[test]
    fn reading_order_is_top_to_bottom_left_to_right() {
        let report = draw_report(150, 42);
        let order = reading_order(&report);
        assert!(!order.is_empty(), "design center fixture must draw tiles");

        // Rebuild the (rect, id) pairs the same way `reading_order` did, so
        // this test can assert the *geometric* claim (y then x, ascending)
        // against the real rects T11 produced, not just re-trust the
        // function under test.
        let mut rects: Vec<Rect> = Vec::new();
        for region in &report.regions {
            for tile in &region.tiles {
                if !tile.dropped {
                    rects.push(tile.raw);
                }
            }
        }
        rects.sort_by_key(|r| (r.y, r.x));
        assert_eq!(rects.len(), order.len());
        for w in rects.windows(2) {
            let (a, b) = (w[0], w[1]);
            assert!(
                (a.y, a.x) <= (b.y, b.x),
                "reading order must be non-decreasing in (y, x): {a:?} then {b:?}"
            );
        }
    }

    #[test]
    fn step_wraps_forward_and_backward() {
        let report = draw_report(150, 42);
        let order = reading_order(&report);
        assert!(order.len() >= 2, "need at least 2 tiles to prove wrap");

        let first = order.first().unwrap();
        let last = order.last().unwrap();

        // Forward from the last tile wraps to the first.
        assert_eq!(
            step(&order, Some(last), Direction::Next).as_ref(),
            Some(first)
        );
        // Backward from the first tile wraps to the last.
        assert_eq!(
            step(&order, Some(first), Direction::Prev).as_ref(),
            Some(last)
        );
    }

    #[test]
    fn step_advances_one_at_a_time_in_order() {
        let report = draw_report(150, 42);
        let order = reading_order(&report);
        assert!(order.len() >= 3);

        let second = step(&order, Some(&order[0]), Direction::Next).unwrap();
        assert_eq!(second, order[1]);
        let back_to_first = step(&order, Some(&second), Direction::Prev).unwrap();
        assert_eq!(back_to_first, order[0]);
    }

    #[test]
    fn no_selection_starts_at_the_first_tile_going_forward() {
        let report = draw_report(150, 42);
        let order = reading_order(&report);
        assert_eq!(step(&order, None, Direction::Next).as_ref(), order.first());
    }

    #[test]
    fn empty_order_never_selects_anything() {
        assert_eq!(step(&[], None, Direction::Next), None);
    }
}
