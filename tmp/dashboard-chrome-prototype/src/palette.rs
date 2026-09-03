use ratatui::style::Color;

use crate::fixture::Status;

pub const BACKGROUND: Color = Color::Rgb(0x1a, 0x1b, 0x26);
pub const TEXT_PRIMARY: Color = Color::Rgb(0xc0, 0xca, 0xf5);
pub const TEXT_DIM: Color = Color::Rgb(0x56, 0x5f, 0x89);

pub const STATUS_RUNNING: Color = Color::Rgb(0x7a, 0xa2, 0xf7);
pub const STATUS_NEEDS_YOU_QUESTION: Color = Color::Rgb(0xf7, 0x76, 0x8e);
pub const STATUS_NEEDS_YOU_PLAIN: Color = Color::Rgb(0xe0, 0xaf, 0x68);
// Idle sessions never render as individual cards in this fixture set (they're always
// collapsed into "+N idle" / "N idle" chips), but the color/glyph are kept for
// completeness with the brief's status palette.
#[allow(dead_code)]
pub const STATUS_IDLE: Color = Color::Rgb(0x56, 0x5f, 0x89);

const PROJECT_COLORS: [Color; 6] = [
    Color::Rgb(0xbb, 0x9a, 0xf7), // purple
    Color::Rgb(0x9e, 0xce, 0x6a), // green
    Color::Rgb(0x7d, 0xcf, 0xff), // cyan
    Color::Rgb(0xff, 0x9e, 0x64), // orange
    Color::Rgb(0x7a, 0xa2, 0xf7), // blue
    Color::Rgb(0xf7, 0x76, 0x8e), // red
];

pub fn project_color(idx: usize) -> Color {
    PROJECT_COLORS[idx % PROJECT_COLORS.len()]
}

pub fn status_color(status: &Status) -> Color {
    match status {
        Status::Running { .. } => STATUS_RUNNING,
        Status::NeedsYouQuestion { .. } => STATUS_NEEDS_YOU_QUESTION,
        Status::NeedsYouPlain { .. } => STATUS_NEEDS_YOU_PLAIN,
    }
}

pub fn status_glyph(status: &Status) -> &'static str {
    match status {
        Status::Running { .. } => "▶",
        Status::NeedsYouQuestion { .. } => "⚠",
        Status::NeedsYouPlain { .. } => "●",
    }
}

#[allow(dead_code)]
pub fn idle_glyph() -> &'static str {
    "○"
}
