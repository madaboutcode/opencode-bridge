//! Render-evidence harness for T11 (`layout.md`/`visuals.md`'s Mosaic
//! layout, promoted from `tmp/20260901-prototype-dashboard-layout/`).
//!
//! Same pattern the spike's own `main.rs --dump` used: draw into a
//! `ratatui::TestBackend`, write the plain-text buffer to `renders/`, and
//! print the structural report metrics that prove what got drawn — this,
//! not "cargo build succeeds," is the evidence the T11 contract's
//! acceptance criterion 5 asks for. Plain-text dumps carry no color, so
//! `crates/dashboard/src/mosaic/render.rs`'s test module verifies
//! color-dependent rules directly against the rendered buffer instead —
//! this harness is for structure and copy text, not color.
//!
//! Not a `--dump`-flagged binary like the spike's `main.rs`, because this
//! crate's `main.rs`/event loop belongs to T12 (T11 contract's "must not
//! touch" list) — a Cargo example is the harness that doesn't require
//! touching it.
//!
//! Run with: `cargo run -p dashboard --example mosaic_dump`

use dashboard::mosaic::{draw, fixtures, render::DrawReport};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

/// Absolute so the output lands in this crate's own `renders/` regardless
/// of the cwd `cargo run`/`cargo run -p dashboard` happens to use (the
/// workspace root when invoked from there, not this crate's directory).
fn renders_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("renders")
}

fn main() {
    std::fs::create_dir_all(renders_dir()).expect("create renders/ dir");

    // 1. R5.8 design center: 4 projects, 8 top-level sessions (one with a
    //    subagent), one idle-only project hidden in the footer.
    let (sessions, naming, now) = fixtures::design_center();
    render_and_report("design-center", &sessions, &naming, now, 10, 150, 42);

    // 2. R9: zero sessions active in the current window.
    let (sessions, naming, now) = fixtures::zero_active();
    render_and_report("zero-active", &sessions, &naming, now, 10, 150, 42);

    // 3. R9.1: below the 40x12 minimum viewport.
    let (sessions, naming, now) = fixtures::design_center();
    render_and_report("below-40x12", &sessions, &naming, now, 10, 35, 10);

    // 4. Sliver check: one low-weight project against a much heavier one —
    //    squarify must not draw the light project as an unreadable
    //    sliver.
    let (sessions, naming, now) = fixtures::single_low_weight_project();
    render_and_report(
        "single-low-weight-project",
        &sessions,
        &naming,
        now,
        10,
        150,
        42,
    );
}

fn render_and_report(
    name: &str,
    sessions: &[dashboard::SessionSnapshot],
    naming: &dashboard::NamingClaimMap,
    now: dashboard::Timestamp,
    window_minutes: u32,
    w: u16,
    h: u16,
) {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("test backend");
    let mut report: Option<DrawReport> = None;
    let mut layout_cache = dashboard::mosaic::LayoutCache::new();
    let completed = term
        .draw(|f| {
            let area = f.area();
            report = Some(draw(
                f,
                area,
                sessions,
                naming,
                now,
                window_minutes,
                &mut layout_cache,
            ));
        })
        .expect("draw");
    let buf = completed.buffer;

    let text = buffer_text(buf, w, h);
    let path = renders_dir().join(format!("{name}-{w}x{h}.txt"));
    std::fs::write(&path, &text).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));

    let report = report.expect("report set");
    print_report(name, w, h, &report);
    println!("  -> wrote {}", path.display());
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

fn print_report(name: &str, w: u16, h: u16, report: &DrawReport) {
    println!("\n=== {name} {w}x{h} ===");
    if report.too_small {
        println!("R9.1: terminal too small at this size; nothing else drawn");
        return;
    }
    if let Some(empty) = &report.empty_state {
        println!(
            "R9: zero active — window {}m, {} older sessions hidden",
            empty.window_minutes, empty.hidden_idle_count
        );
        return;
    }

    println!(
        "header: {} projects, {} sessions (? {}  ● {}  running {}  ○ {})",
        report.header.projects,
        report.header.sessions,
        report.header.q,
        report.header.need,
        report.header.run,
        report.header.idle
    );
    if !report.hidden_projects.is_empty() {
        println!("footer hidden (all-idle): {:?}", report.hidden_projects);
    }
    if let Some(agg) = &report.aggregate {
        println!(
            "R9.2 aggregate chip: {} projects ({} sessions) at {:?}",
            agg.project_count, agg.session_count, agg.raw
        );
    }

    println!("regions:");
    for r in &report.regions {
        println!(
            "  {}: raw {},{} {}x{}  plate {},{} {}x{}  weight={} aspect={:.2}:1 kind={:?} tag_counts_shown={}",
            r.project_name,
            r.raw.x, r.raw.y, r.raw.width, r.raw.height,
            r.plate.x, r.plate.y, r.plate.width, r.plate.height,
            r.weight, r.aspect, r.kind, r.tag_counts_shown
        );
        if let Some(bottom) = &r.bottom_row {
            println!(
                "    bottom row: idle_count={} session_overflow={}",
                bottom.idle_overflow_count, bottom.session_overflow_count
            );
        }
        for t in &r.tiles {
            if t.dropped {
                println!("    DROPPED tile: {}", t.session_nick);
                continue;
            }
            println!(
                "    [{}] raw {}x{} weight={}: wi={} h={} regime={} blocks={:?} blank_rows_left={}",
                t.session_nick,
                t.raw.width,
                t.raw.height,
                t.weight,
                t.wi,
                t.h,
                t.regime,
                t.blocks_rendered,
                t.blank_rows_left
            );
        }
    }
}
