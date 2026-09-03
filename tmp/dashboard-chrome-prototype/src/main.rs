mod fixture;
mod layout;
mod palette;
mod render;

use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::{Frame, Terminal};

use fixture::Fixture;
use render::ChromeOption;

#[derive(Clone, Copy)]
enum FixtureKind {
    Sparse,
    Busy,
}

impl FixtureKind {
    fn build(self) -> Fixture {
        match self {
            FixtureKind::Sparse => fixture::sparse(),
            FixtureKind::Busy => fixture::busy(),
        }
    }
}

#[derive(Clone, Copy)]
enum Screen {
    Intro,
    Option(ChromeOption, FixtureKind, u16),
}

const SCREENS: [Screen; 10] = [
    Screen::Intro,
    Screen::Option(ChromeOption::A, FixtureKind::Sparse, 120),
    Screen::Option(ChromeOption::A, FixtureKind::Busy, 120),
    Screen::Option(ChromeOption::A, FixtureKind::Busy, 80),
    Screen::Option(ChromeOption::B, FixtureKind::Sparse, 120),
    Screen::Option(ChromeOption::B, FixtureKind::Busy, 120),
    Screen::Option(ChromeOption::B, FixtureKind::Busy, 80),
    Screen::Option(ChromeOption::C, FixtureKind::Sparse, 120),
    Screen::Option(ChromeOption::C, FixtureKind::Busy, 120),
    Screen::Option(ChromeOption::C, FixtureKind::Busy, 80),
];

/// Restores the terminal on drop (normal exit or panic unwind), so a crash never leaves
/// the user's shell stuck in raw mode / the alternate screen.
struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(TerminalGuard { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn main() -> io::Result<()> {
    // Ensure the terminal is restored even if we panic mid-render.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    let mut guard = TerminalGuard::new()?;
    guard.terminal.hide_cursor()?;

    let mut idx: usize = 0;
    loop {
        guard.terminal.draw(|f| draw_screen(f, idx))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('n') | KeyCode::Char(' ') | KeyCode::Right => {
                        idx = (idx + 1) % SCREENS.len();
                    }
                    KeyCode::Char('p') | KeyCode::Left => {
                        idx = (idx + SCREENS.len() - 1) % SCREENS.len();
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn draw_screen(f: &mut Frame, idx: usize) {
    let area = f.area();
    // Fill background so the "80 cols" frames visibly float on the real terminal size.
    f.render_widget(Block::new().style(Style::new().bg(palette::BACKGROUND)), area);

    if area.height < 2 {
        return;
    }
    let content_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height - 1,
    };
    let footer_area = Rect {
        x: area.x,
        y: area.y + area.height - 1,
        width: area.width,
        height: 1,
    };

    match SCREENS[idx] {
        Screen::Intro => {
            render::intro::draw(f, pad(content_area));
        }
        Screen::Option(option, fixture_kind, width) => {
            let fixture = fixture_kind.build();
            let render_area = centered(content_area, width);
            render::render_dashboard(f, render_area, option, &fixture);
        }
    }

    draw_footer(f, footer_area, idx);
}

fn pad(area: Rect) -> Rect {
    Rect {
        x: area.x + 1,
        y: area.y,
        width: area.width.saturating_sub(2),
        height: area.height,
    }
}

/// Simulates a narrower terminal: a centered Rect exactly `width` cols wide (clamped to
/// what's actually available), leaving the rest of the real terminal as background margin.
fn centered(area: Rect, width: u16) -> Rect {
    let w = width.min(area.width);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    Rect { x, y: area.y, width: w, height: area.height }
}

fn draw_footer(f: &mut Frame, area: Rect, idx: usize) {
    let text = match SCREENS[idx] {
        Screen::Intro => format!(
            "[frame {idx}/9] Intro — already decided — n/p navigate, q quit"
        ),
        Screen::Option(option, fixture_kind, width) => {
            let fixture = fixture_kind.build();
            format!(
                "[frame {idx}/9] {} — {}, {width} cols — n/p navigate, q quit",
                option.label(),
                fixture.label
            )
        }
    };
    let line = Line::from(Span::styled(text, Style::new().fg(palette::TEXT_DIM)));
    f.render_widget(Paragraph::new(line), area);
}
