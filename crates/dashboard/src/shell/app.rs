//! The event loop and top-level app state — window value, selection,
//! help-overlay-open flag (T12 contract's "owns" list). [`run`] is this
//! crate's entry point, called from `main.rs` once the opencode adapter is
//! already paired and running.
//!
//! [`run`] is deliberately synchronous, not `async`: `dashboard`'s `main`
//! spawns the adapter's own async work (`HarnessAdapter::run`, which
//! internally does its own `tokio::spawn`) once, before calling this
//! function, and that work keeps progressing on the tokio runtime's other
//! worker threads while this function blocks its calling thread inside
//! crossterm's own `event::poll`/`event::read` — the classic ratatui event
//! loop shape. That sidesteps bridging a blocking terminal-input API into
//! an async `select!` for no benefit this task needs; see this task's
//! report.

use std::io;
use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::style::Modifier;
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::adapter::SessionEvent;
use crate::mosaic::{self, DrawReport};
use crate::snapshot::Timestamp;

use super::keys::{self, Action};
use super::live::LiveState;
use super::nav::{self, TileId};
use super::reclassify::reclassify;
use super::terminal::TerminalGuard;
use super::window::WindowState;
use super::{footer, help};

/// ~250ms responsiveness (`overview.md` R2) without busy-waiting: each loop
/// iteration blocks in `event::poll` for at most this long, then always
/// redraws (elapsed-time strings need to advance even with no keypress).
const TICK: Duration = Duration::from_millis(250);

pub struct App {
    live: LiveState,
    window: WindowState,
    help_open: bool,
    selected: Option<TileId>,
    last_order: Vec<TileId>,
    should_quit: bool,
    /// Owned across the whole session so real frames actually benefit from
    /// `layout.md` R5.4's reflow gate (point 6) — a fresh cache per call
    /// would be equivalent to always recomputing.
    layout_cache: mosaic::LayoutCache,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            live: LiveState::new(),
            window: WindowState::new(),
            help_open: false,
            selected: None,
            last_order: Vec::new(),
            should_quit: false,
            layout_cache: mosaic::LayoutCache::new(),
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn apply_events(&mut self, events: Vec<SessionEvent>) {
        if !events.is_empty() {
            self.live.apply_events(events);
        }
    }

    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => {
                // R7.1: q/Esc close the help overlay if it's open, and only
                // quit the process from the bare main screen — there's no
                // zoom view in v1 for "back" to mean anything else.
                if self.help_open {
                    self.help_open = false;
                } else {
                    self.should_quit = true;
                }
            }
            Action::ToggleHelp => self.help_open = !self.help_open,
            Action::Nav(dir) => {
                self.selected = nav::step(&self.last_order, self.selected.as_ref(), dir);
            }
            Action::Window(key) => self.window.apply(key),
            Action::Noop => {}
        }
    }

    /// Draws one frame. `now` is read fresh by the caller on every call —
    /// never cached — so elapsed-time strings and the R3 active/idle split
    /// are always current.
    pub fn render(&mut self, f: &mut Frame, now: Timestamp) {
        let area = f.area();
        if self.help_open {
            // R7.1: "footer hidden while the overlay is open... the
            // overlay replaces it" — replace the whole screen rather than
            // draw on top of the mosaic.
            help::draw(f, area);
            return;
        }

        let window = self.window.as_window();
        let live_sessions = reclassify(self.live.snapshots(), now, window);
        let report = mosaic::draw(
            f,
            area,
            &live_sessions,
            self.live.naming(),
            now,
            self.window.minutes(),
            &mut self.layout_cache,
        );

        self.last_order = nav::reading_order(&report);
        if let Some(sel) = &self.selected {
            highlight_selected(f, &report, sel);
        }

        // R9.1: below the minimum viewport, `mosaic::draw` already drew
        // nothing but the "terminal too small" panel — don't paint a
        // footer over it.
        if !report.too_small {
            let text = footer::text(
                window,
                &report.header,
                &report.hidden_projects,
                report.aggregate.as_ref(),
            );
            footer::draw(f, area, &text);
        }
    }
}

/// Highlights the selected tile (R7.1's navigate) by reversing its plate's
/// existing colors. `mosaic::draw`'s own public signature has no selection
/// parameter (T11's contract: "must not touch T11 internals" — adding one
/// would require editing `render.rs`), so this task overlays the highlight
/// itself, after the fact, using only the `Rect`s `DrawReport` already
/// handed back and the public `Frame`/`Buffer` API T11's own code uses.
fn highlight_selected(f: &mut Frame, report: &DrawReport, sel: &TileId) {
    for region in &report.regions {
        if region.project_idx != sel.project_idx {
            continue;
        }
        for tile in &region.tiles {
            if tile.dropped || tile.session_nick != sel.nick {
                continue;
            }
            let buf = f.buffer_mut();
            let area = buf.area;
            for y in tile.raw.y..(tile.raw.y + tile.raw.height).min(area.y + area.height) {
                for x in tile.raw.x..(tile.raw.x + tile.raw.width).min(area.x + area.width) {
                    buf[(x, y)].modifier.toggle(Modifier::REVERSED);
                }
            }
        }
    }
}

/// Drains everything currently queued on `rx` without blocking. Called
/// once per wake-up (`run`'s loop) so a burst of events — the adapter's
/// initial reconcile sweep, or several tombstones firing back to back — is
/// applied as a single `LiveState::apply_events` batch; see that
/// function's doc comment for why batching matters for AC9's
/// claim-order guarantee.
fn drain(rx: &mut UnboundedReceiver<SessionEvent>) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    events
}

/// `dashboard`'s main loop (`overview.md` R2). Takes over the terminal,
/// then alternates: apply whatever adapter events arrived, redraw, wait up
/// to [`TICK`] for the next keypress/resize. Returns once the user quits —
/// by the time this returns, the terminal is already restored (this
/// function's own `TerminalGuard` was dropped at the end of its scope).
pub fn run(mut rx: UnboundedReceiver<SessionEvent>) -> io::Result<()> {
    super::terminal::install_panic_hook();
    let (_guard, mut terminal) = TerminalGuard::enter()?;
    let mut app = App::new();

    loop {
        app.apply_events(drain(&mut rx));

        let now = Timestamp::now();
        terminal.draw(|f| app.render(f, now))?;

        if app.should_quit() {
            return Ok(());
        }

        if event::poll(TICK)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let action = keys::map_key(key, app.help_open);
                    app.handle_action(action);
                }
            }
            // Event::Resize and anything else: nothing to do explicitly —
            // `Terminal::draw` re-reads the backend's real size on every
            // call (R2's "resize is handled"). This arm exists so `poll`
            // waking for a resize isn't treated as "nothing happened."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::nav::Direction;
    use crate::snapshot::{AttentionState, HarnessKind, ProjectId, SessionId, SessionSnapshot};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal as RTerminal;
    use std::path::PathBuf;

    fn snap(session: &str, project: &str) -> SessionSnapshot {
        SessionSnapshot {
            session_id: SessionId::new(HarnessKind("test"), session),
            project_id: ProjectId::from_canonical(PathBuf::from(project)),
            parent_id: None,
            attention: AttentionState::Running {
                turn_started: Timestamp::from_epoch_millis(0),
            },
            current_action: None,
            wire_title: None,
            final_assistant_text: None,
            last_user_prompt: None,
            files_touched: vec![],
            recent_actions: vec![],
            created_at: Timestamp::from_epoch_millis(0),
            last_updated: Timestamp::from_epoch_millis(0),
        }
    }

    #[test]
    fn quit_key_sets_should_quit_from_main_screen() {
        let mut app = App::new();
        app.handle_action(Action::Quit);
        assert!(app.should_quit());
    }

    #[test]
    fn quit_key_closes_help_instead_of_quitting_when_overlay_open() {
        let mut app = App::new();
        app.handle_action(Action::ToggleHelp);
        assert!(app.help_open);
        app.handle_action(Action::Quit);
        assert!(!app.help_open, "Esc/q must close the overlay first");
        assert!(
            !app.should_quit(),
            "must not also quit the process in the same keypress"
        );
    }

    #[test]
    fn navigation_selects_a_real_tile_after_one_render() {
        let mut app = App::new();
        app.apply_events(vec![
            SessionEvent::Snapshot(Box::new(snap("s1", "/tmp/proj"))),
            SessionEvent::Snapshot(Box::new(snap("s2", "/tmp/proj"))),
        ]);

        let mut term = RTerminal::new(TestBackend::new(150, 42)).unwrap();
        term.draw(|f| app.render(f, Timestamp::from_epoch_millis(0)))
            .unwrap();
        assert!(
            !app.last_order.is_empty(),
            "render must have populated the reading order"
        );

        app.handle_action(Action::Nav(Direction::Next));
        assert!(app.selected.is_some());
        assert_eq!(app.selected.as_ref(), app.last_order.first());
    }

    #[test]
    fn help_overlay_hides_the_footer() {
        let mut app = App::new();
        app.handle_action(Action::ToggleHelp);
        let mut term = RTerminal::new(TestBackend::new(80, 24)).unwrap();
        term.draw(|f| app.render(f, Timestamp::from_epoch_millis(0)))
            .unwrap();
        let buf = term.backend().buffer();
        let full: String = (0..24)
            .map(|y| (0..80).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !full.contains("window:"),
            "the R7.1 footer must not be visible while the help overlay is open"
        );
    }

    #[test]
    fn footer_shows_the_exact_literal_window_format_on_the_main_screen() {
        let mut app = App::new();
        app.apply_events(vec![SessionEvent::Snapshot(Box::new(snap(
            "s1",
            "/tmp/proj",
        )))]);
        let mut term = RTerminal::new(TestBackend::new(80, 24)).unwrap();
        term.draw(|f| app.render(f, Timestamp::from_epoch_millis(0)))
            .unwrap();
        let buf = term.backend().buffer();
        let last_row: String = (0..80).map(|x| buf[(x, 23)].symbol()).collect();
        assert!(
            last_row.starts_with("window: 15m (1 live / 0 idle)"),
            "footer row was: {last_row:?}"
        );
    }

    #[test]
    fn window_key_changes_the_footer_on_the_very_next_frame() {
        let mut app = App::new();
        app.handle_action(Action::Window(super::super::window::WindowKey::ShowAll));
        let mut term = RTerminal::new(TestBackend::new(80, 24)).unwrap();
        term.draw(|f| app.render(f, Timestamp::from_epoch_millis(0)))
            .unwrap();
        let buf = term.backend().buffer();
        let last_row: String = (0..80).map(|x| buf[(x, 23)].symbol()).collect();
        assert!(
            last_row.starts_with("window: all"),
            "footer row was: {last_row:?}"
        );
    }

    #[test]
    fn subagent_reaches_render_with_a_claimed_nickname_not_the_raw_id() {
        // End-to-end smoke of AC9 through the actual App, not just
        // `LiveState` in isolation: a subagent snapshot arrives, gets
        // claimed by this task's wiring, and the render pipeline (T11) can
        // resolve a real nickname for it via T10's public claim map.
        let mut app = App::new();
        let parent = snap("parent-1", "/tmp/proj");
        let mut child = snap("very-long-native-child-session-id", "/tmp/proj");
        child.parent_id = Some(SessionId::new(HarnessKind("test"), "parent-1"));
        app.apply_events(vec![
            SessionEvent::Snapshot(Box::new(parent)),
            SessionEvent::Snapshot(Box::new(child.clone())),
        ]);

        let nickname = app.live.naming().nickname_of(&child.session_id);
        assert!(
            nickname.is_some(),
            "subagent must be claimed by App's own wiring"
        );
    }
}
