//! Skeleton binary. The Mosaic dashboard TUI (docs/specs/dashboard/overview.md)
//! lands here in later M3 tasks — this crate exists now only to prove the
//! workspace shape: it builds, runs, and links against `opencode-client`.

// Not called yet — proves this crate is wired to opencode-client, the
// library later M3 tasks build the session-fetching path on top of.
use opencode_client as _;

fn main() {
    println!("dashboard: not yet implemented");
}
