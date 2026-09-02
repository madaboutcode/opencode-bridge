//! Library surface for the dashboard crate. `docs/specs/dashboard/
//! overview.md` is the design; T09 (this crate's first real task) builds
//! the `HarnessAdapter` boundary, the core session/project model, and the
//! opencode adapter — everything under `adapter`/`snapshot`/
//! `project_identity` is harness-agnostic; everything under `opencode` is
//! not and stays physically separate (`code-quality`'s encapsulation
//! rule). Rendering (T11), naming/claim-map (T10), and the main event loop
//! (T12) land in later modules this file will grow to include.

pub mod adapter;
pub mod opencode;
pub mod project_identity;
pub mod snapshot;

pub use adapter::{HarnessAdapter, SessionEvent};
pub use opencode::OpencodeAdapter;
pub use snapshot::{AttentionState, HarnessKind, ProjectId, SessionId, SessionSnapshot, Timestamp};
