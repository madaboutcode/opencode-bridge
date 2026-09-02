//! T10 — the naming/claim-map scheme, `docs/specs/dashboard/visuals.md`
//! R6.8. Pure logic, no I/O: [`claim_map::NamingClaimMap`] owns the
//! project→category and session→word claim state; [`wordlist::CATEGORIES`]
//! is the frozen 10-category/60-word Appendix it draws from.
//!
//! Consumes only T09's `SessionId`/`ProjectId`/`Timestamp`
//! (`crate::snapshot`) as input — no redefinition of those types, and no
//! dependency on T11's render code, which doesn't exist yet.

pub mod claim_map;
pub mod wordlist;

pub use claim_map::{CategoryAssignment, LiveSession, NamingClaimMap, SessionNickname};
pub use wordlist::{Category, CATEGORIES};
