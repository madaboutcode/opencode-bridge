//! Shared error type. Kept to a single alias so every module can propagate
//! with `?` without pulling in an error-handling crate (SPEC.md §3: keep
//! dependencies lean).

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
