//! The Mosaic layout and card rendering — `docs/specs/dashboard/layout.md`
//! (R5-R5.11, R9-R9.2) and `docs/specs/dashboard/visuals.md` (R6,
//! R6.1-R6.3, R6.7, R6.8's display side). Promoted from the verified spike
//! `tmp/20260901-prototype-dashboard-layout/` (T11 contract): the
//! squarify algorithm, two-pass region/tile packing, and content-scaling
//! ladder are ported faithfully from there; this module's own work is
//! wiring them to T09's real [`crate::snapshot::SessionSnapshot`] and
//! T10's real [`crate::naming::NamingClaimMap`] instead of the spike's
//! fixture data, plus the R9/R9.2 degrade coverage the spike didn't have
//! (see `render.rs`'s doc comment).
//!
//! Reads only `crate::snapshot`'s and `crate::naming`'s public surface —
//! no opencode wire type, no harness-specific knowledge anywhere in this
//! module (T11 contract, acceptance criterion 6).
//!
//! Module layout mirrors the spike's file split, since that split is what
//! was actually verified:
//! - [`squarify`] — the treemap algorithm itself, untouched.
//! - [`view`] — turns real T09/T10 data into the render-time model the
//!   rest of this module consumes (plays the role the spike's `fixture.rs`
//!   played, except sourced from real data instead of hand-written cases).
//! - [`layout`] — geometry: project regions, session tiles, idle row,
//!   R5.6's tile cap, R9.2's degrade hierarchy.
//! - [`ladder`] — the tile-content regime table (R5.3).
//! - [`palette`] — Tokyo Night colors and per-state color/glyph mapping
//!   (R6, R6.1, R6.7).
//! - [`render`] — draws everything to a `ratatui::Frame`; [`draw`] below is
//!   this module's public entry point, re-exported for T12's main loop.

pub mod fixtures;
pub mod ladder;
pub mod layout;
pub mod palette;
pub mod render;
pub mod squarify;
pub mod view;

pub use layout::LayoutCache;
pub use render::{draw, DrawReport};
