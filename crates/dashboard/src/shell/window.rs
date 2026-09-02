//! R8's window-control state (`docs/specs/dashboard/interactions.md`
//! R8/R8.1). Pure state machine, no I/O — `shell::app::App` is the only
//! caller, applying one [`WindowKey`] per keypress via [`WindowState::apply`].
//! Kept separate from key-mapping (`keys.rs`) so the six-key semantics
//! (clamp, no-auto-show-all) are testable without a terminal or a crossterm
//! `KeyEvent`.

pub const DEFAULT_MINUTES: u32 = 10;
pub const MIN_MINUTES: u32 = 1;
pub const MAX_MINUTES: u32 = 60;
const COARSE_STEP: u32 = 5;
const FINE_STEP: u32 = 1;

/// One R8 keypress, already resolved from whatever key actually produced it
/// (`keys.rs`'s job) — this type only knows the six effects R8 defines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowKey {
    GrowCoarse,
    ShrinkCoarse,
    GrowFine,
    ShrinkFine,
    Reset,
    ShowAll,
}

/// What `mosaic::draw`'s `window_minutes` cosmetic parameter and this
/// task's own R3 reclassification (`reclassify.rs`) both key off. `Minutes`
/// is the ordinary windowed mode; `All` is R8's `a` — never reachable via
/// `]`/`Shift+]` (`WindowState::apply` enforces that; only
/// `WindowKey::ShowAll` ever produces it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Window {
    Minutes(u32),
    All,
}

/// Tracks `W` (always a live 1-60 value, even while "show all" is active,
/// so `]`/`[` pressed from "show all" has a sane number to resume from —
/// see this task's report for that judgment call) and the show-all flag
/// separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowState {
    minutes: u32,
    show_all: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            minutes: DEFAULT_MINUTES,
            show_all: false,
        }
    }
}

impl WindowState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies one R8 keypress. Every case triggers an immediate recompute
    /// on the caller's very next `draw` call (`interactions.md` R8: "no
    /// animation, no debounce") — this function only ever mutates `self`;
    /// the recompute itself happens because `shell::app::App::render` reads
    /// `as_window()` fresh on every frame, never a cached value.
    pub fn apply(&mut self, key: WindowKey) {
        match key {
            WindowKey::GrowCoarse => self.grow(COARSE_STEP),
            WindowKey::ShrinkCoarse => self.shrink(COARSE_STEP),
            WindowKey::GrowFine => self.grow(FINE_STEP),
            WindowKey::ShrinkFine => self.shrink(FINE_STEP),
            WindowKey::Reset => {
                self.minutes = DEFAULT_MINUTES;
                self.show_all = false;
            }
            WindowKey::ShowAll => self.show_all = true,
        }
    }

    /// `]`/`Shift+]`: clamps at [`MAX_MINUTES`], never auto-transitions
    /// into show-all past it (`interactions.md` R8's explicit scenario).
    /// Also exits show-all — see the struct doc comment.
    fn grow(&mut self, step: u32) {
        self.minutes = (self.minutes + step).min(MAX_MINUTES);
        self.show_all = false;
    }

    /// `[`/`Shift+[`: clamps at [`MIN_MINUTES`].
    fn shrink(&mut self, step: u32) {
        self.minutes = self.minutes.saturating_sub(step).max(MIN_MINUTES);
        self.show_all = false;
    }

    /// The value `reclassify` and the R7.1 footer key off.
    pub fn as_window(&self) -> Window {
        if self.show_all {
            Window::All
        } else {
            Window::Minutes(self.minutes)
        }
    }

    /// The numeric `W` to feed `mosaic::draw`'s cosmetic `window_minutes`
    /// argument even while "show all" is active — `draw` only reads it for
    /// R9's empty-state copy, which needs *some* number even in that rare
    /// corner case (show-all with literally zero live sessions at all, not
    /// just zero within a window). See this task's report.
    pub fn minutes(&self) -> u32 {
        self.minutes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_window_is_10_minutes_not_show_all() {
        let w = WindowState::new();
        assert_eq!(w.as_window(), Window::Minutes(DEFAULT_MINUTES));
    }

    #[test]
    fn grow_and_shrink_use_the_documented_deltas() {
        let mut w = WindowState::new();
        w.apply(WindowKey::GrowCoarse);
        assert_eq!(w.as_window(), Window::Minutes(15));
        w.apply(WindowKey::ShrinkCoarse);
        assert_eq!(w.as_window(), Window::Minutes(10));
        w.apply(WindowKey::GrowFine);
        assert_eq!(w.as_window(), Window::Minutes(11));
        w.apply(WindowKey::ShrinkFine);
        assert_eq!(w.as_window(), Window::Minutes(10));
    }

    #[test]
    fn clamps_at_60_and_never_auto_transitions_to_show_all() {
        let mut w = WindowState::new();
        for _ in 0..20 {
            w.apply(WindowKey::GrowCoarse);
        }
        assert_eq!(w.as_window(), Window::Minutes(MAX_MINUTES));
        w.apply(WindowKey::GrowCoarse);
        assert_eq!(
            w.as_window(),
            Window::Minutes(MAX_MINUTES),
            "] past 60 must clamp, never auto-enter show-all"
        );
    }

    #[test]
    fn clamps_at_1_minute() {
        let mut w = WindowState::new();
        for _ in 0..20 {
            w.apply(WindowKey::ShrinkCoarse);
        }
        assert_eq!(w.as_window(), Window::Minutes(MIN_MINUTES));
    }

    #[test]
    fn a_enters_show_all_and_w_resets_out_of_it() {
        let mut w = WindowState::new();
        w.apply(WindowKey::ShowAll);
        assert_eq!(w.as_window(), Window::All);
        w.apply(WindowKey::Reset);
        assert_eq!(w.as_window(), Window::Minutes(DEFAULT_MINUTES));
    }

    #[test]
    fn window_keys_exit_show_all_and_resume_from_the_last_numeric_value() {
        let mut w = WindowState::new();
        w.apply(WindowKey::GrowCoarse); // 15m
        w.apply(WindowKey::ShowAll);
        assert_eq!(w.as_window(), Window::All);
        w.apply(WindowKey::GrowCoarse);
        assert_eq!(
            w.as_window(),
            Window::Minutes(20),
            "] from show-all must resume numeric windowing from the last W, not the default"
        );
    }
}
