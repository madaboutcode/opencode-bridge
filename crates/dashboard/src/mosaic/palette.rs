//! Tokyo Night palette and per-state color/glyph mapping — `visuals.md` R6
//! (palette), R6.1/R6.7 (attention-state color/glyph, never area), R5.11
//! (project accent placement). Ported verbatim from the verified spike
//! (`tmp/20260901-prototype-dashboard-layout/src/palette.rs`); only the
//! `State` import changed, from the spike's fixture type to
//! `crate::mosaic::view::State`.

use std::sync::OnceLock;

use ratatui::style::Color;

use crate::mosaic::view::State;

/// Selects between real Nerd Font glyphs (default — needs a patched font
/// installed in the terminal) and plain-Unicode fallbacks (safe on any
/// terminal, no font dependency). Set once at startup from a CLI flag/env
/// var (`main.rs`), read on every glyph lookup below. A process-wide
/// `OnceLock` rather than a threaded parameter: unlike session data (which
/// `layout.md` R5.4 requires recomputed fresh every frame, never cached),
/// this is an immutable-for-the-process rendering preference, the same
/// category as the terminal's color support — threading it through every
/// ladder/render function signature would be pure mechanical churn for a
/// value that never changes after startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconMode {
    Nerd,
    Plain,
}

static ICON_MODE: OnceLock<IconMode> = OnceLock::new();

/// Called exactly once, before the first frame renders (`main.rs`). A
/// second call is a programming error, not a runtime condition — panics
/// rather than silently ignoring a would-be mode change mid-run.
pub fn set_icon_mode(mode: IconMode) {
    ICON_MODE
        .set(mode)
        .expect("set_icon_mode called more than once");
}

/// Defaults to `Nerd` if `set_icon_mode` was never called (unit tests,
/// and any future caller that doesn't go through `main.rs`).
pub fn icon_mode() -> IconMode {
    *ICON_MODE.get().unwrap_or(&IconMode::Nerd)
}

pub const GUTTER: Color = Color::Rgb(0x16, 0x16, 0x1e);
pub const PLATE: Color = Color::Rgb(0x1a, 0x1b, 0x26);

pub const TEXT_PRIMARY: Color = Color::Rgb(0xc0, 0xca, 0xf5);
pub const TEXT_SECONDARY: Color = Color::Rgb(0xa9, 0xb1, 0xd6);
pub const TEXT_DIM: Color = Color::Rgb(0x56, 0x5f, 0x89);
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

/// R5.11: applied only to the project name text in its region's tag row —
/// never a tile background, tile border, or region border. Callers in
/// `render.rs` are the only ones that may use this, and only for that one
/// element.
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

/// Running spinner frame, one per 250ms tick — the only motion in plain
/// mode. `tick` is derived by the caller from the render-time clock
/// (`now.epoch_millis() / 250`), not tracked as state here.
///
/// Nerd mode uses a single static glyph (`nf-fa-circle_notch`, U+F1CE)
/// instead of rotating: the glyph itself already reads as a spinner
/// mid-turn, so cycling it every 250ms would just flicker rather than
/// convey motion — the blue `STATUS_RUNNING` color is what says "active"
/// in that mode.
pub fn running_glyph(tick: usize) -> &'static str {
    match icon_mode() {
        IconMode::Plain => {
            const FRAMES: [&str; 4] = ["◐", "◓", "◑", "◒"];
            FRAMES[tick % 4]
        }
        IconMode::Nerd => "\u{f1ce}",
    }
}

/// Per-state glyph, both modes. Plain-mode glyphs are the original
/// symbols (unchanged, so anyone without a patched font sees exactly what
/// shipped before this). Nerd-mode glyphs are real Nerd Font Font Awesome
/// codepoints, verified against the upstream glyph table (not guessed):
/// question-circle (f059), exclamation-circle (f06a), circle-notch
/// (f1ce, via `running_glyph`), circle-o (f10c). All four are variants of
/// the same circular base shape in nerd mode — a deliberate Gestalt choice
/// (same frame, different fill/mark) so the eye learns one shape family
/// and discriminates state by what's inside it, the same way `htop`'s
/// process-state letters or `k9s`'s pod-status glyphs share a column.
/// Every state is also still distinguishable by glyph shape alone in
/// monochrome (not color-only), for the ~8% of users with color vision
/// deficiency — `visuals.md` R6.1's color-and-glyph redundancy rule.
pub fn state_glyph(state: State, tick: usize) -> &'static str {
    match (icon_mode(), state) {
        (IconMode::Plain, State::Question) => "?",
        (IconMode::Plain, State::NeedsYou) => "●",
        (IconMode::Plain, State::Idle) => "○",
        (IconMode::Nerd, State::Question) => "\u{f059}",
        (IconMode::Nerd, State::NeedsYou) => "\u{f06a}",
        (IconMode::Nerd, State::Idle) => "\u{f10c}",
        (_, State::Running) => running_glyph(tick),
    }
}

/// The subagent-list connector (`ladder.rs::subagent_line`), always
/// rendered in `TEXT_DIM` regardless of the subagent's own state — it's
/// structural chrome (like a tree-drawing character), not a data signal.
pub fn connector_glyph() -> &'static str {
    match icon_mode() {
        IconMode::Plain => "↳",
        IconMode::Nerd => "\u{f149}", // nf-fa-level_down
    }
}

/// The header mark (`render.rs::draw_header`, " {glyph} opencode "). No
/// official opencode logo glyph exists in Nerd Fonts; `nf-cod-terminal`
/// (a terminal/code icon) is the closest fit for "a tool that watches
/// coding agents running in a terminal."
pub fn header_glyph() -> &'static str {
    match icon_mode() {
        IconMode::Plain => "◆",
        IconMode::Nerd => "\u{eb50}", // nf-cod-terminal
    }
}
