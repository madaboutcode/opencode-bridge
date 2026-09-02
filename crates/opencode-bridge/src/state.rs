//! Shared state handed to the MCP loop, the tool handlers, and the SSE
//! consumer task. Wrapped in a single `Arc` by main.rs; no field needs its
//! own `Arc` since the `Client` is stateless (cheap to share by reference)
//! and `Registry` already has interior mutability.

use crate::notify::Notifier;
use crate::registry::Registry;
use opencode_client::Client;

pub struct AppState {
    pub client: Client,
    pub registry: Registry,
    pub notifier: Notifier,
    /// Label identifying "which CC session/bridge process this is"
    /// (SPEC.md §8). Used only for the session title tag and prompt
    /// metadata and `opencode_list` rediscovery — NEVER for deciding
    /// whether to notify (see the invariant on `Registry::claim_notification`).
    pub origin: String,
    /// Default working directory for new sessions when `opencode_run` gets no
    /// explicit `directory` (SPEC.md §5). Captured once at startup as the
    /// bridge process's cwd — i.e. the project CC launched the bridge from —
    /// so a bare `opencode_run` targets that project instead of opencode's
    /// server-side default ($HOME).
    pub default_dir: Option<String>,
}
