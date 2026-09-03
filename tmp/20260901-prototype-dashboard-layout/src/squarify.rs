use ratatui::layout::Rect;

/// A leaf item for the squarified treemap.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TreemapItem {
    pub id: String,
    pub weight: f64,
    pub label: String,
}

/// Compute squarified treemap layout for the given items within `area`.
/// Returns a Vec of (item_index, Rect) pairs.
/// Items with zero or negative weight are ignored.
pub fn squarify(items: &[TreemapItem], area: Rect) -> Vec<(usize, Rect)> {
    if items.is_empty() || area.width == 0 || area.height == 0 {
        return vec![];
    }

    // Filter to only positive weight items, keeping original indices
    let valid_items: Vec<(usize, &TreemapItem)> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.weight > 0.0)
        .collect();

    if valid_items.is_empty() {
        return vec![];
    }

    let total_weight: f64 = valid_items.iter().map(|(_, item)| item.weight).sum();
    let total_area = (area.width as f64) * (area.height as f64);
    let mut result = Vec::new();
    squarify_recurse(
        &valid_items,
        0..valid_items.len(),
        area,
        total_area,
        total_weight,
        &mut result,
    );
    result
}

fn squarify_recurse(
    items: &[(usize, &TreemapItem)],
    indices: std::ops::Range<usize>,
    rect: Rect,
    _total_area: f64,
    remaining_weight: f64,
    result: &mut Vec<(usize, Rect)>,
) {
    let count = indices.len();
    if count == 0 {
        return;
    }
    if count == 1 {
        let (orig_idx, _) = items[indices.start];
        result.push((orig_idx, rect));
        return;
    }

    // Wide rect → vertical strip on the left (carved from width, spans full height)
    // Tall rect → horizontal strip on top  (carved from height, spans full width)
    let is_wide = rect.width >= rect.height;
    // For aspect-ratio computation:
    //   strip_dim  = strip_fraction * (dimension strip is carved from)
    //   tile_long  = (item_weight / row_weight) * (dimension tiles stack along)
    let strip_base = if is_wide { rect.width } else { rect.height } as f64;
    let tile_base = if is_wide { rect.height } else { rect.width } as f64;

    // ── Find the best split point (worst aspect ratio) ──
    let mut worst_ratio = f64::INFINITY;
    let mut split = indices.start + 1;
    let mut row_weight = 0.0f64;

    for i in indices.clone() {
        row_weight += items[i].1.weight;
        let strip_fraction = row_weight / remaining_weight;
        let strip_dim = strip_fraction * strip_base;

        let mut best_worst = f64::NEG_INFINITY;
        for j in indices.start..=i {
            let tile_long = (items[j].1.weight / row_weight) * tile_base;
            let ratio = if strip_dim > tile_long {
                strip_dim / tile_long
            } else {
                tile_long / strip_dim
            };
            best_worst = best_worst.max(ratio);
        }

        if best_worst <= worst_ratio {
            worst_ratio = best_worst;
            split = i + 1;
        } else {
            break;
        }
    }

    // ── Layout the chosen row ──
    let row_weight_sum: f64 = items[indices.start..split]
        .iter()
        .map(|(_, item)| item.weight)
        .sum();
    let row_fraction = row_weight_sum / remaining_weight;
    let is_last_row = split == indices.end;

    if is_wide {
        // ── Vertical strip on the LEFT ──
        let strip_width = if is_last_row {
            rect.width
        } else {
            let exact = row_fraction * (rect.width as f64);
            let sw = exact.round() as u16;
            let max_w = rect.width.saturating_sub(1).max(1);
            sw.min(max_w).max(1)
        };

        let mut laid_y = rect.y;

        for idx in indices.start..split {
            let (orig_idx, item) = items[idx];
            let is_last_tile = idx == split - 1;
            let tile_height = if is_last_tile {
                rect.y + rect.height - laid_y
            } else {
                let exact = (item.weight / row_weight_sum) * (rect.height as f64);
                let h = exact.round() as u16;
                let remaining_tiles = (split - idx - 1) as u16;
                let space_left = rect.y + rect.height - laid_y;
                h.min(space_left.saturating_sub(remaining_tiles).max(1))
                    .max(1)
            };

            result.push((orig_idx, Rect {
                x: rect.x,
                y: laid_y,
                width: strip_width,
                height: tile_height,
            }));
            laid_y += tile_height;
        }

        if !is_last_row {
            let remaining_rect = Rect {
                x: rect.x + strip_width,
                y: rect.y,
                width: rect.width - strip_width,
                height: rect.height,
            };
            squarify_recurse(
                items,
                split..indices.end,
                remaining_rect,
                _total_area,
                remaining_weight - row_weight_sum,
                result,
            );
        }
    } else {
        // ── Horizontal strip on TOP ──
        let strip_height = if is_last_row {
            rect.height
        } else {
            let exact = row_fraction * (rect.height as f64);
            let sh = exact.round() as u16;
            let max_h = rect.height.saturating_sub(1).max(1);
            sh.min(max_h).max(1)
        };

        let mut laid_x = rect.x;

        for idx in indices.start..split {
            let (orig_idx, item) = items[idx];
            let is_last_tile = idx == split - 1;
            let tile_width = if is_last_tile {
                rect.x + rect.width - laid_x
            } else {
                let exact = (item.weight / row_weight_sum) * (rect.width as f64);
                let w = exact.round() as u16;
                let remaining_tiles = (split - idx - 1) as u16;
                let space_left = rect.x + rect.width - laid_x;
                w.min(space_left.saturating_sub(remaining_tiles).max(1))
                    .max(1)
            };

            result.push((orig_idx, Rect {
                x: laid_x,
                y: rect.y,
                width: tile_width,
                height: strip_height,
            }));
            laid_x += tile_width;
        }

        if !is_last_row {
            let remaining_rect = Rect {
                x: rect.x,
                y: rect.y + strip_height,
                width: rect.width,
                height: rect.height - strip_height,
            };
            squarify_recurse(
                items,
                split..indices.end,
                remaining_rect,
                _total_area,
                remaining_weight - row_weight_sum,
                result,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let items: Vec<TreemapItem> = vec![];
        let result = squarify(&items, Rect::new(0, 0, 80, 24));
        assert!(result.is_empty());
    }

    #[test]
    fn single_item() {
        let items = vec![TreemapItem { id: "a".into(), weight: 1.0, label: "A".into() }];
        let result = squarify(&items, Rect::new(0, 0, 80, 24));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, Rect::new(0, 0, 80, 24));
    }

    #[test]
    fn equal_weights_similar_aspect() {
        let items: Vec<TreemapItem> = (0..4)
            .map(|i| TreemapItem { id: format!("s{i}"), weight: 1.0, label: format!("S{i}") })
            .collect();
        let result = squarify(&items, Rect::new(0, 0, 80, 20));
        assert_eq!(result.len(), 4);
        // All tiles should have reasonable aspect ratios (< 4)
        for (_, rect) in &result {
            let ratio = if rect.width > rect.height {
                rect.width as f64 / rect.height as f64
            } else {
                rect.height as f64 / rect.width as f64
            };
            assert!(ratio < 4.0, "aspect ratio {ratio} too high for rect {rect:?}");
        }
    }

    #[test]
    fn unequal_weights() {
        let items = vec![
            TreemapItem { id: "a".into(), weight: 3.0, label: "A".into() },
            TreemapItem { id: "b".into(), weight: 1.0, label: "B".into() },
        ];
        let result = squarify(&items, Rect::new(0, 0, 80, 20));
        assert_eq!(result.len(), 2);
        // First item should be larger
        let area0 = result[0].1.width as f64 * result[0].1.height as f64;
        let area1 = result[1].1.width as f64 * result[1].1.height as f64;
        assert!(area0 > area1);
    }

    #[test]
    fn zero_weight_items_ignored() {
        let items = vec![
            TreemapItem { id: "a".into(), weight: 0.0, label: "A".into() },
            TreemapItem { id: "b".into(), weight: 1.0, label: "B".into() },
        ];
        let result = squarify(&items, Rect::new(0, 0, 80, 20));
        // Only one item gets a rect
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn no_sliver_bars() {
        let items: Vec<TreemapItem> = (0..3)
            .map(|i| TreemapItem { id: format!("s{i}"), weight: 1.0, label: format!("S{i}") })
            .collect();
        let result = squarify(&items, Rect::new(0, 0, 80, 24));
        assert_eq!(result.len(), 3);
        for (_, r) in &result {
            assert!(r.width >= 10, "width {} too small in {:?}", r.width, r);
            assert!(r.height >= 6, "height {} too small in {:?}", r.height, r);
            let ratio = if r.width > r.height {
                r.width as f64 / r.height as f64
            } else {
                r.height as f64 / r.width as f64
            };
            assert!(ratio < 5.0, "aspect ratio {ratio} too high for {r:?}");
        }
    }

    #[test]
    fn covers_parent() {
        let items: Vec<TreemapItem> = (0..3)
            .map(|i| TreemapItem { id: format!("s{i}"), weight: 1.0, label: format!("S{i}") })
            .collect();
        let parent_area = 80.0 * 24.0;
        let result = squarify(&items, Rect::new(0, 0, 80, 24));
        assert_eq!(result.len(), 3);
        let total_tile_area: f64 = result
            .iter()
            .map(|(_, r)| r.width as f64 * r.height as f64)
            .sum();
        assert!(
            total_tile_area >= parent_area - 3.0,
            "total tile area {total_tile_area} is less than parent {parent_area} minus tolerance"
        );
        // Every tile inside parent bounds (u16 is always ≥ 0)
        for (_, r) in &result {
            assert!(r.x + r.width <= 80, "{:?} overflows parent width", r);
            assert!(r.y + r.height <= 24, "{:?} overflows parent height", r);
        }
    }

    #[test]
    fn wide_parent_not_columns() {
        let items = vec![
            TreemapItem { id: "a".into(), weight: 3.0, label: "A".into() },
            TreemapItem { id: "b".into(), weight: 1.0, label: "B".into() },
        ];
        let result = squarify(&items, Rect::new(0, 0, 80, 20));
        assert_eq!(result.len(), 2);
        for (_, r) in &result {
            assert!(r.width != 1, "got a 1-cell-wide sliver column: {r:?}");
        }
    }
}
