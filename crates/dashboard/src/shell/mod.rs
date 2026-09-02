//! T12 — the interactive shell: main event loop, window controls, keyboard
//! navigation (`docs/specs/dashboard/overview.md` R2, R3/R3.1,
//! `interactions.md` R7.1/R8). This module (plus `main.rs`) replaces T08's
//! placeholder binary with the real running dashboard — it wires T09
//! (`crate::adapter`/`crate::opencode`), T10 (`crate::naming`), and T11
//! (`crate::mosaic`) together, calling only their public interfaces.
//!
//! - [`terminal`] — R2's takeover/restore (raw mode, alternate screen,
//!   panic guard).
//! - [`window`] — R8's `W`/show-all state machine.
//! - [`nav`] — R7.1's on-screen reading order and wraparound stepping.
//! - [`reclassify`] — R3/R3.1's active-window reclassification, computed
//!   here (not by T09 — see `snapshot.rs`'s `AttentionState::Idle` doc
//!   comment) from `SessionSnapshot::last_updated`.
//! - [`live`] — the live session map and T10 claim-map wiring, both
//!   directions (AC9).
//! - [`keys`] — crossterm `KeyEvent` -> `Action` mapping.
//! - [`footer`] / [`help`] — this task's own interaction chrome (R7.1's
//!   footer literal format, the `?` overlay).
//! - [`app`] — ties the above into `App` and the event loop; [`app::run`]
//!   is this crate's entry point, called from `main.rs`.

pub mod app;
pub mod footer;
pub mod help;
pub mod keys;
pub mod live;
pub mod nav;
pub mod reclassify;
pub mod terminal;
pub mod window;

pub use app::run;
