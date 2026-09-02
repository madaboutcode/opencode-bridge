//! Shared client library for opencode2's HTTP + SSE API. Scoped per
//! `docs/specs/dashboard/overview.md` R1.1: authentication (pairing),
//! health, the session-list/session-message HTTP calls, and the raw
//! event-stream primitive. No TUI dependency, no MCP dependency — this
//! crate knows nothing about either consumer. Today's consumer is
//! `opencode-bridge` (the MCP binary); the `dashboard` binary consumes it
//! too, starting from later M3 tasks.

pub mod error;
pub mod opencode;
pub mod sse;

pub use error::Result;
pub use opencode::{
    latest_assistant_error, latest_assistant_text, resolve_bin, AgentInfo, AgentModel, Client,
    Creds, FinalTurn, Message, MessageError, MessagePart, MessageTime, ModelInfo, ModelRef,
    SessionInfo, SessionTime,
};

// `pair` shadows nothing at the crate root but is re-exported separately
// from the type re-exports above for readability (it's the one free
// function alongside `resolve_bin`, everything else above is a type).
pub use opencode::pair;
