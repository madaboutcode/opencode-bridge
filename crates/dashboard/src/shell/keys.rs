//! Key -> [`Action`] mapping (`docs/specs/dashboard/interactions.md`
//! R7.1/R8). Pure: takes a crossterm `KeyEvent` and the current
//! help-overlay-open flag, returns what to do — no terminal, no `App`
//! mutation here, so R7.1/R8's key semantics are testable without a live
//! terminal (T12 contract, AC3/AC5).

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::shell::nav::Direction;
use crate::shell::window::WindowKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Quit,
    ToggleHelp,
    Nav(Direction),
    Window(WindowKey),
    Noop,
}

/// R8.1: exactly the R7.1 + R8 bindings below, no rebinding, no config
/// file.
///
/// `Shift+]`/`Shift+[` are detected by the *character* a terminal actually
/// sends for that chord on a US layout (`}`/`{`) rather than a modifier
/// flag on `]`/`[`: most terminals report the already-shifted symbol
/// character with no `KeyModifiers::SHIFT` alongside it (that's a
/// legacy-vt100-vs-kitty-keyboard-protocol difference outside this task's
/// control — the kitty protocol *would* report the modifier too, but isn't
/// assumed here), so keying off the character is the portable choice. See
/// this task's report for this judgment call and its non-US-layout
/// limitation.
///
/// Enter is intentionally unmapped below (falls through to `Action::Noop`)
/// — Amendment 3 cuts session zoom from v1; T12 contract AC7.
pub fn map_key(key: KeyEvent, help_open: bool) -> Action {
    if help_open {
        return match key.code {
            KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Esc => Action::ToggleHelp,
            _ => Action::Noop,
        };
    }
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('j') | KeyCode::Down | KeyCode::Right => Action::Nav(Direction::Next),
        KeyCode::Char('k') | KeyCode::Up | KeyCode::Left => Action::Nav(Direction::Prev),
        KeyCode::Char(']') => Action::Window(WindowKey::GrowCoarse),
        KeyCode::Char('}') => Action::Window(WindowKey::GrowFine),
        KeyCode::Char('[') => Action::Window(WindowKey::ShrinkCoarse),
        KeyCode::Char('{') => Action::Window(WindowKey::ShrinkFine),
        KeyCode::Char('w') => Action::Window(WindowKey::Reset),
        KeyCode::Char('a') => Action::Window(WindowKey::ShowAll),
        _ => Action::Noop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn q_and_esc_quit_from_main_screen() {
        assert_eq!(
            map_key(key(KeyCode::Char('q'), KeyModifiers::NONE), false),
            Action::Quit
        );
        assert_eq!(
            map_key(key(KeyCode::Esc, KeyModifiers::NONE), false),
            Action::Quit
        );
    }

    #[test]
    fn ctrl_c_quits() {
        assert_eq!(
            map_key(key(KeyCode::Char('c'), KeyModifiers::CONTROL), false),
            Action::Quit
        );
    }

    #[test]
    fn plain_c_without_control_does_nothing() {
        assert_eq!(
            map_key(key(KeyCode::Char('c'), KeyModifiers::NONE), false),
            Action::Noop
        );
    }

    #[test]
    fn help_overlay_intercepts_q_esc_and_question_mark_as_close_not_quit() {
        assert_eq!(
            map_key(key(KeyCode::Char('q'), KeyModifiers::NONE), true),
            Action::ToggleHelp
        );
        assert_eq!(
            map_key(key(KeyCode::Esc, KeyModifiers::NONE), true),
            Action::ToggleHelp
        );
        assert_eq!(
            map_key(key(KeyCode::Char('?'), KeyModifiers::NONE), true),
            Action::ToggleHelp
        );
    }

    #[test]
    fn help_overlay_swallows_every_other_key() {
        assert_eq!(
            map_key(key(KeyCode::Char(']'), KeyModifiers::NONE), true),
            Action::Noop
        );
        assert_eq!(
            map_key(key(KeyCode::Char('j'), KeyModifiers::NONE), true),
            Action::Noop
        );
    }

    #[test]
    fn navigate_keys_map_to_next_and_prev() {
        for code in [KeyCode::Char('j'), KeyCode::Down, KeyCode::Right] {
            assert_eq!(
                map_key(key(code, KeyModifiers::NONE), false),
                Action::Nav(Direction::Next)
            );
        }
        for code in [KeyCode::Char('k'), KeyCode::Up, KeyCode::Left] {
            assert_eq!(
                map_key(key(code, KeyModifiers::NONE), false),
                Action::Nav(Direction::Prev)
            );
        }
    }

    #[test]
    fn window_keys_map_to_the_documented_deltas() {
        assert_eq!(
            map_key(key(KeyCode::Char(']'), KeyModifiers::NONE), false),
            Action::Window(WindowKey::GrowCoarse)
        );
        assert_eq!(
            map_key(key(KeyCode::Char('['), KeyModifiers::NONE), false),
            Action::Window(WindowKey::ShrinkCoarse)
        );
        assert_eq!(
            map_key(key(KeyCode::Char('}'), KeyModifiers::NONE), false),
            Action::Window(WindowKey::GrowFine)
        );
        assert_eq!(
            map_key(key(KeyCode::Char('{'), KeyModifiers::NONE), false),
            Action::Window(WindowKey::ShrinkFine)
        );
        assert_eq!(
            map_key(key(KeyCode::Char('w'), KeyModifiers::NONE), false),
            Action::Window(WindowKey::Reset)
        );
        assert_eq!(
            map_key(key(KeyCode::Char('a'), KeyModifiers::NONE), false),
            Action::Window(WindowKey::ShowAll)
        );
    }

    #[test]
    fn enter_is_unbound() {
        assert_eq!(
            map_key(key(KeyCode::Enter, KeyModifiers::NONE), false),
            Action::Noop
        );
    }

    #[test]
    fn help_key_opens_from_main_screen() {
        assert_eq!(
            map_key(key(KeyCode::Char('?'), KeyModifiers::NONE), false),
            Action::ToggleHelp
        );
    }
}
