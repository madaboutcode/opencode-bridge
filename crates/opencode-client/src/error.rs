//! Shared error type. Kept to a single alias so every module can propagate
//! with `?` without pulling in an error-handling crate (SPEC.md §3: keep
//! dependencies lean). Duplicated from opencode-bridge's own `error.rs`
//! rather than factored into a third crate — see the T08 migration contract.

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
