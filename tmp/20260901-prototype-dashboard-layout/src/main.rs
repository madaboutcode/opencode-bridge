// Mosaic dashboard spike: area-proportional project regions, state-coloured session
// tiles, tile content that grows with tile size. Throwaway ratatui TUI — see
// BRIEF-v2.md. Not production code.

mod fixture;
mod ladder;
mod layout;
mod palette;
mod render;
mod squarify;

use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::{CrosstermBackend, TestBackend};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use fixture::Fixture;
use layout::RegionKind;

const SIM_WIDTH: u16 = 80;

fn main() -> io::Result<()> {
    if std::env::args().any(|a| a == "--dump") {
        dump_mode();
        return Ok(());
    }
    run_interactive()
}

struct App {
    width_sim: bool,
    real_fixture: bool,
    tick: usize,
}

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

fn run_interactive() -> io::Result<()> {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    let mut guard = TerminalGuard::new()?;
    guard.terminal.hide_cursor()?;

    let mut real = fixture::build_real();
    let mut stress = fixture::build_stress();
    let mut app = App { width_sim: false, real_fixture: true, tick: 0 };

    loop {
        let fixture: &Fixture = if app.real_fixture { &real } else { &stress };
        guard.terminal.draw(|f| {
            let area = f.area();
            let render_area = if app.width_sim { left_align(area, SIM_WIDTH) } else { area };
            render::draw(f, render_area, fixture, app.tick);
        })?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                let target = if app.real_fixture { &mut real } else { &mut stress };
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('w') => app.width_sim = !app.width_sim,
                    KeyCode::Char('f') => app.real_fixture = !app.real_fixture,
                    KeyCode::Char('+') => target.add_session(),
                    KeyCode::Char('-') => target.remove_session(),
                    KeyCode::Char('p') => target.add_project(),
                    KeyCode::Char('.') => target.tick_recent(),
                    _ => {}
                }
            }
        }
        app.tick = app.tick.wrapping_add(1);
    }

    Ok(())
}

fn left_align(area: Rect, width: u16) -> Rect {
    let w = width.min(area.width);
    Rect { x: area.x, y: area.y, width: w, height: area.height }
}

// ---------------------------------------------------------------------------
// --dump: renders REAL and STRESS at 150x42 and 80x36 via TestBackend, writes plain-text
// frames to renders/, and prints the report metrics BRIEF-v2.md's "Report" section asks
// for. This — not "cargo build succeeds" — is the evidence the spike is meant to produce.
// ---------------------------------------------------------------------------

fn dump_mode() {
    std::fs::create_dir_all("renders").expect("create renders/ dir");

    let real = fixture::build_real();
    let stress = fixture::build_stress();
    let sizes = [(150u16, 42u16), (80u16, 36u16)];

    for (name, fx) in [("real", &real), ("stress", &stress)] {
        for &(w, h) in &sizes {
            render_and_report(name, fx, w, h);
        }
    }

    println!("\n=== R5.7 evidence: `+` `+` `p` on REAL at 150x42 ===");
    println!("{}", r57_evidence());
}

fn render_and_report(name: &str, fx: &Fixture, w: u16, h: u16) {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("test backend");
    let mut report: Option<render::DrawReport> = None;
    let completed = term
        .draw(|f| {
            let area = f.area();
            report = Some(render::draw(f, area, fx, 0));
        })
        .expect("draw");
    let buf = completed.buffer;

    let text = buffer_text(buf, w, h);
    let path = format!("renders/{name}-{w}x{h}.txt");
    std::fs::write(&path, &text).unwrap_or_else(|e| panic!("write {path}: {e}"));

    let report = report.expect("report set");
    print_report(name, w, h, buf, &report);
}

fn buffer_text(buf: &Buffer, w: u16, h: u16) -> String {
    let mut out = String::with_capacity((w as usize + 1) * h as usize);
    for y in 0..h {
        for x in 0..w {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

const TILE_BGS: [ratatui::style::Color; 4] =
    [palette::TILE_BG_QUESTION, palette::TILE_BG_NEEDS_YOU, palette::TILE_BG_RUNNING, palette::TILE_BG_IDLE];

/// Classifies BODY cells (rows 1..H-2; header/footer excluded) into gutter/plate (no
/// tile), tile-blank (tile bg, space), tile-text (tile bg, non-space) — purely from the
/// rendered buffer's colours, so this measures what actually got drawn, not what the
/// layout code intended to draw.
fn classify_cells(buf: &Buffer, w: u16, h: u16) -> (usize, usize, usize) {
    if h < 2 {
        return (0, 0, 0);
    }
    let mut gutter = 0usize;
    let mut tile_blank = 0usize;
    let mut tile_text = 0usize;
    for y in 1..(h - 1) {
        for x in 0..w {
            let cell = &buf[(x, y)];
            if TILE_BGS.contains(&cell.bg) {
                if cell.symbol() == " " {
                    tile_blank += 1;
                } else {
                    tile_text += 1;
                }
            } else {
                gutter += 1;
            }
        }
    }
    (gutter, tile_blank, tile_text)
}

fn print_report(name: &str, w: u16, h: u16, buf: &Buffer, report: &render::DrawReport) {
    println!("\n=== {name} {w}x{h} ===");
    if report.too_small {
        println!("terminal too small at this size; nothing else drawn");
        return;
    }

    // 1. cell classification
    let (gutter, tile_blank, tile_text) = classify_cells(buf, w, h);
    let total = gutter + tile_blank + tile_text;
    let pct = |n: usize| if total == 0 { 0.0 } else { n as f64 / total as f64 * 100.0 };
    println!(
        "1. body cells ({total} total): gutter/plate {gutter} ({:.1}%)  tile-blank {tile_blank} ({:.1}%)  tile-text {tile_text} ({:.1}%)",
        pct(gutter),
        pct(tile_blank),
        pct(tile_text)
    );

    println!(
        "0. on-screen: {} projects, {} sessions (? {}  ● {}  ◐ {}  ○ {})",
        report.header.projects, report.header.sessions, report.header.q, report.header.need, report.header.run, report.header.idle
    );

    // 2. region list + worst aspect + B1 tag-row check
    println!("2. regions:");
    let mut worst: Option<&render::RegionReport> = None;
    for r in &report.regions {
        println!(
            "   {}: raw {},{} {}x{}  plate {},{} {}x{}  weight={} aspect={:.2}:1 kind={:?} tag_state_counts_shown={}",
            r.project_name,
            r.raw.x, r.raw.y, r.raw.width, r.raw.height,
            r.plate.x, r.plate.y, r.plate.width, r.plate.height,
            r.weight, r.aspect, r.kind, r.tag_counts_shown
        );
        if worst.map_or(true, |w2| r.aspect > w2.aspect) {
            worst = Some(r);
        }
    }
    if let Some(w2) = worst {
        println!("   WORST aspect: {} at {:.2}:1", w2.project_name, w2.aspect);
    }
    for r in &report.regions {
        if let Some(idle) = &r.idle_row {
            println!("   idle row [{}]: shown={:?} +N_idle={}", r.project_name, idle.chips_shown, idle.overflow_count);
        }
    }
    if !report.hidden_projects.is_empty() {
        println!("   hidden (footer, all-idle): {:?}", report.hidden_projects);
    }

    // 3. tiles in <12 width regimes + dropped regions/tiles
    println!("3. narrow (<12 wi) tiles and drops:");
    let mut any3 = false;
    for r in &report.regions {
        if r.kind == RegionKind::NotDrawn {
            println!("   DROPPED region: {} (raw {}x{} too small)", r.project_name, r.raw.width, r.raw.height);
            any3 = true;
        }
        for t in &r.tiles {
            if t.dropped {
                println!("   DROPPED tile: {} in {}", t.session_nick, r.project_name);
                any3 = true;
            } else if t.wi < 12 {
                println!("   {} in {}: wi={} h={} regime={}", t.session_nick, r.project_name, t.wi, t.h, t.regime);
                any3 = true;
            }
        }
    }
    if !any3 {
        println!("   none");
    }

    // 4. REAL 150x42: extended blocks rendered per tile + blank rows left
    if name == "real" && w == 150 && h == 42 {
        println!("4. REAL 150x42 per-tile detail:");
        for r in &report.regions {
            for t in &r.tiles {
                if t.dropped {
                    continue;
                }
                println!(
                    "   [{}] {} (raw {}x{} weight={}): wi={} h={} regime={} blocks={:?} blank_rows_left={}",
                    r.project_name, t.session_nick, t.raw.width, t.raw.height, t.weight, t.wi, t.h, t.regime, t.blocks_rendered, t.blank_rows_left
                );
            }
        }
    }
}

/// R5.7 evidence: applies `+` `+` `p` (two synthetic sessions on the last project, then a
/// new project) to the REAL fixture at 150x42 and reports, in words, which project
/// regions moved column vs. only resized. Computed via the same TestBackend/layout path
/// as an interactive run would use, at the two points in time before/after the keys —
/// deterministic and reproducible, rather than eyeballed from a live terminal.
fn r57_evidence() -> String {
    let mut fx = fixture::build_real();

    let before = {
        let mut term = Terminal::new(TestBackend::new(150, 42)).unwrap();
        let mut report = None;
        term.draw(|f| {
            let area = f.area();
            report = Some(render::draw(f, area, &fx, 0));
        })
        .unwrap();
        report.unwrap()
    };

    fx.add_session();
    fx.add_session();
    fx.add_project();

    let after = {
        let mut term = Terminal::new(TestBackend::new(150, 42)).unwrap();
        let mut report = None;
        term.draw(|f| {
            let area = f.area();
            report = Some(render::draw(f, area, &fx, 0));
        })
        .unwrap();
        report.unwrap()
    };

    let mut lines = vec![];
    for br in &before.regions {
        if let Some(ar) = after.regions.iter().find(|r| r.project_idx == br.project_idx) {
            let moved_column = br.raw.x != ar.raw.x;
            let resized_only = !moved_column && br.raw != ar.raw;
            let unchanged = br.raw == ar.raw;
            let verdict = if moved_column {
                "MOVED COLUMN"
            } else if resized_only {
                "resized only (same x)"
            } else if unchanged {
                "unchanged"
            } else {
                "unchanged"
            };
            lines.push(format!(
                "{}: {verdict} — before {:?} -> after {:?}",
                br.project_name, br.raw, ar.raw
            ));
        }
    }
    for ar in &after.regions {
        if !before.regions.iter().any(|r| r.project_idx == ar.project_idx) {
            lines.push(format!("{}: NEW region at {:?}", ar.project_name, ar.raw));
        }
    }
    lines.join("\n")
}
