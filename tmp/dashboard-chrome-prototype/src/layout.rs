// Generic left-to-right, wrap-when-out-of-width flow packing. Used at two levels:
// project boxes flowing across the screen, and cards flowing inside a project box.
// This is a simple first-fit row-wrap (like CSS flex-wrap), not an optimal bin packer —
// good enough for a static design-comparison mockup.

pub struct BoxSpec {
    pub width: u16,
    pub height: u16,
}

/// Groups item indices into rows such that each row's total width (items + gap between
/// them) fits within `available_width`. Always places at least one item per row even if
/// it alone exceeds `available_width` (caller is expected to have already clamped specs
/// that are wider than the screen).
pub fn flow_rows(available_width: u16, gap: u16, specs: &[BoxSpec]) -> Vec<Vec<usize>> {
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut current_row: Vec<usize> = Vec::new();
    let mut current_width: u16 = 0;

    for (i, spec) in specs.iter().enumerate() {
        let needed = if current_row.is_empty() {
            spec.width
        } else {
            current_width + gap + spec.width
        };
        if !current_row.is_empty() && needed > available_width {
            rows.push(std::mem::take(&mut current_row));
            current_width = 0;
        }
        if current_row.is_empty() {
            current_width = spec.width;
        } else {
            current_width += gap + spec.width;
        }
        current_row.push(i);
    }
    if !current_row.is_empty() {
        rows.push(current_row);
    }
    rows
}
