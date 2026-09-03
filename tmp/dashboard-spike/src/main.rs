mod app;
mod data;
mod squarify;
mod ui;

use std::io::stdout;
use std::panic;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), cursor::Show, LeaveAlternateScreen);
    }
}

fn main() {
    // Panic hook to restore terminal
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), cursor::Show, LeaveAlternateScreen);
        original_hook(info);
    }));

    enable_raw_mode().expect("failed to enable raw mode");
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide).expect("failed to enter alternate screen");

    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");

    let mut app = App::new();

    loop {
        terminal.draw(|frame| ui::draw(frame, &app)).expect("failed to draw");

        if event::poll(std::time::Duration::from_millis(250)).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(key)) => {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            KeyCode::Char('j') | KeyCode::Down | KeyCode::Right => app.select_next(),
                            KeyCode::Char('k') | KeyCode::Up | KeyCode::Left => app.select_prev(),
                            KeyCode::Char(']') => app.adjust_window(300), // +5m
                            KeyCode::Char('[') => app.adjust_window(-300), // -5m
                            KeyCode::Char('w') => app.reset_window(),
                            KeyCode::Char('a') => app.toggle_show_all(),
                            KeyCode::Enter => {
                                let label = app.selected_label();
                                app.status = Some(format!("zoom: {label} (not in this spike)"));
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Event::Resize(_, _)) => {
                    // Terminal resize — just continue the loop so the next draw uses the new size
                }
                _ => {}
            }
        }
    }
}
