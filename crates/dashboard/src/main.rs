//! Skeleton binary. The Mosaic dashboard TUI (docs/specs/dashboard/overview.md)
//! lands here in later M3 tasks — T09 built the `HarnessAdapter` boundary,
//! core session model, and opencode adapter (`dashboard::adapter`,
//! `dashboard::snapshot`, `dashboard::opencode`), but wiring them into an
//! actual event loop / terminal UI is T12's job, not this task's — see
//! `tasks/2026-09-02-opencode-dashboard/contracts/T09-adapter-boundary.md`'s
//! "Must not touch" list.

// Not called yet — proves this crate's binary target links against its own
// library, which T12's main loop builds the real event loop on top of.
use dashboard as _;

fn main() {
    println!("dashboard: not yet implemented");
}
