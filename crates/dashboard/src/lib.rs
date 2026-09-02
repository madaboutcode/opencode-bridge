//! Library surface for the dashboard crate. `docs/specs/dashboard/
//! overview.md` is the design; T09 (this crate's first real task) builds
//! the `HarnessAdapter` boundary, the core session/project model, and the
//! opencode adapter — everything under `adapter`/`snapshot`/
//! `project_identity` is harness-agnostic; everything under `opencode` is
//! not and stays physically separate (`code-quality`'s encapsulation
//! rule). `naming` (T10) is the session/project nickname claim-map
//! (`visuals.md` R6.8) — also harness-agnostic, pure logic, consuming only
//! `snapshot`'s types. Rendering (T11) and the main event loop (T12) land
//! in later modules this file will grow to include.

pub mod adapter;
pub mod naming;
pub mod opencode;
pub mod project_identity;
pub mod snapshot;

pub use adapter::{HarnessAdapter, SessionEvent};
pub use naming::{CategoryAssignment, LiveSession, NamingClaimMap, SessionNickname};
pub use opencode::OpencodeAdapter;
pub use snapshot::{AttentionState, HarnessKind, ProjectId, SessionId, SessionSnapshot, Timestamp};
