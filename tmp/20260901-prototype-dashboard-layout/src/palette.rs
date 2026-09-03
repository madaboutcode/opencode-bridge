use ratatui::style::Color;

use crate::fixture::State;

pub const GUTTER: Color = Color::Rgb(0x16, 0x16, 0x1e);
pub const PLATE: Color = Color::Rgb(0x1a, 0x1b, 0x26);

pub const TEXT_PRIMARY: Color = Color::Rgb(0xc0, 0xca, 0xf5);
pub const TEXT_SECONDARY: Color = Color::Rgb(0xa9, 0xb1, 0xd6);
pub const TEXT_DIM: Color = Color::Rgb(0x56, 0x5f, 0x89);
pub const SUBAGENT: Color = Color::Rgb(0x7d, 0xcf, 0xff);
pub const LIVE: Color = Color::Rgb(0x9e, 0xce, 0x6a);

pub const STATUS_RUNNING: Color = Color::Rgb(0x7a, 0xa2, 0xf7);
pub const STATUS_QUESTION: Color = Color::Rgb(0xf7, 0x76, 0x8e);
pub const STATUS_NEEDS_YOU: Color = Color::Rgb(0xe0, 0xaf, 0x68);
pub const STATUS_IDLE: Color = Color::Rgb(0x56, 0x5f, 0x89);

pub const TILE_BG_QUESTION: Color = Color::Rgb(0x3d, 0x22, 0x30);
pub const TILE_BG_NEEDS_YOU: Color = Color::Rgb(0x3a, 0x2e, 0x1b);
pub const TILE_BG_RUNNING: Color = Color::Rgb(0x1f, 0x2d, 0x52);
pub const TILE_BG_IDLE: Color = Color::Rgb(0x1e, 0x20, 0x30);

pub const TILE_TEXT_QUESTION: Color = STATUS_QUESTION;
pub const TILE_TEXT_NEEDS_YOU: Color = STATUS_NEEDS_YOU;
pub const TILE_TEXT_RUNNING: Color = STATUS_RUNNING;
pub const TILE_TEXT_IDLE: Color = STATUS_IDLE;

pub const TILE_BODY_QUESTION: Color = TEXT_PRIMARY;
pub const TILE_BODY_NEEDS_YOU: Color = TEXT_SECONDARY;
pub const TILE_BODY_RUNNING: Color = TEXT_SECONDARY;
pub const TILE_BODY_IDLE: Color = STATUS_IDLE;

const PROJECT_COLORS: [Color; 6] = [
    Color::Rgb(0xbb, 0x9a, 0xf7), // purple
    Color::Rgb(0x9e, 0xce, 0x6a), // green
    Color::Rgb(0x7d, 0xcf, 0xff), // cyan
    Color::Rgb(0xff, 0x9e, 0x64), // orange
    Color::Rgb(0x7a, 0xa2, 0xf7), // blue
    Color::Rgb(0x73, 0xda, 0xca), // teal
];

pub fn project_color(first_appearance_idx: usize) -> Color {
    PROJECT_COLORS[first_appearance_idx % PROJECT_COLORS.len()]
}

pub fn tile_bg(state: State) -> Color {
    match state {
        State::Question => TILE_BG_QUESTION,
        State::NeedsYou => TILE_BG_NEEDS_YOU,
        State::Running => TILE_BG_RUNNING,
        State::Idle => TILE_BG_IDLE,
    }
}

pub fn tile_text(state: State) -> Color {
    match state {
        State::Question => TILE_TEXT_QUESTION,
        State::NeedsYou => TILE_TEXT_NEEDS_YOU,
        State::Running => TILE_TEXT_RUNNING,
        State::Idle => TILE_TEXT_IDLE,
    }
}

pub fn tile_body(state: State) -> Color {
    match state {
        State::Question => TILE_BODY_QUESTION,
        State::NeedsYou => TILE_BODY_NEEDS_YOU,
        State::Running => TILE_BODY_RUNNING,
        State::Idle => TILE_BODY_IDLE,
    }
}

/// Running spinner frame, one per 250ms tick — the only motion.
pub fn running_glyph(tick: usize) -> &'static str {
    const FRAMES: [&str; 4] = ["◐", "◓", "◑", "◒"];
    FRAMES[tick % 4]
}

pub fn state_glyph(state: State, tick: usize) -> &'static str {
    match state {
        State::Question => "?",
        State::NeedsYou => "●",
        State::Running => running_glyph(tick),
        State::Idle => "○",
    }
}
