//! opencode-bridge: an MCP stdio server that gives Claude Code tools to
//! drive opencode2 over its HTTP + SSE API (SPEC.md §0). Startup: discover
//! the paired opencode2 server, verify it's reachable, spawn the SSE
//! consumer and its periodic backstop sweep, then serve the MCP stdio loop
//! until stdin closes.

mod error;
mod mcp;
mod notify;
mod opencode;
mod registry;
mod sse;
mod state;
mod tools;

use std::sync::Arc;

use error::Result;
use state::AppState;

#[tokio::main]
async fn main() {
    // Cheap --version/--help handling before we touch opencode discovery.
    // CONTRIBUTING.md and bug-report templates reference these flags.
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("opencode-bridge {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("opencode-bridge {}", env!("CARGO_PKG_VERSION"));
        println!("MCP stdio server that drives opencode2 over HTTP + SSE.");
        println!();
        println!("USAGE:");
        println!("    opencode-bridge [--version] [--help]");
        println!();
        println!("ENV:");
        println!("    OPENCODE2_BIN                   path to opencode2 binary");
        println!("    CLAUDE_CODE_MESSAGING_SOCKET    AF_UNIX inbox for CC callbacks");
        println!("    CLAUDE_CODE_MESSAGING_TOKEN     auth token for the inbox");
        return;
    }

    if let Err(e) = run().await {
        eprintln!("[bridge] fatal: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let bin = opencode::resolve_bin();
    let creds = opencode::pair(&bin).await?;
    eprintln!("[bridge] discovered opencode2 at {}", creds.base_url);

    let client = opencode::Client::new(bin, creds);
    client.health().await.map_err(|e| {
        format!("GET /api/health failed: {e} — is `opencode2 service start` running?")
    })?;
    eprintln!("[bridge] opencode2 health check OK");

    let notifier = notify::Notifier::from_env();
    if notifier.enabled() {
        eprintln!("[bridge] CC callback channel enabled");
    } else {
        eprintln!("[bridge] CC callback channel disabled (no CLAUDE_CODE_MESSAGING_SOCKET) — running notify-less");
    }

    let origin = derive_origin();
    eprintln!("[bridge] origin = {origin}");

    // The bridge's cwd is where CC launched it — the user's project dir.
    // Use it as the default session directory so tasks land in the project
    // rather than opencode's server-side $HOME default.
    let default_dir = std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(str::to_string));
    match &default_dir {
        Some(d) => eprintln!("[bridge] default session directory = {d}"),
        None => eprintln!(
            "[bridge] cwd unreadable — new sessions use opencode's server-side default dir"
        ),
    }

    let state = Arc::new(AppState {
        client,
        registry: registry::Registry::new(),
        notifier,
        origin,
        default_dir,
    });

    tokio::spawn(sse::run(state.clone()));
    tokio::spawn(sse::periodic_sweep(state.clone()));

    mcp::serve_stdio(state).await;
    Ok(())
}

/// Label for "which CC session (or bridge process) this is" (SPEC.md §8).
/// A LABEL, never a capability — see the invariant documented on
/// `registry::Registry::claim_notification`. Used only for the session
/// title tag, prompt metadata, and `opencode_list` rediscovery.
fn derive_origin() -> String {
    let socket = std::env::var("CLAUDE_CODE_MESSAGING_SOCKET").unwrap_or_default();
    if socket.is_empty() {
        // FALLBACK-OK: SPEC.md §8 — "Falls back to a per-process random id
        // if the socket env is empty." The bridge's own OS pid is unique
        // per process, which is all "random" needs to mean here — origin
        // is cosmetic in this case anyway, since notify() is a no-op
        // without a socket (see notify::Notifier).
        return format!("noproc{}", std::process::id());
    }
    // e.g. "/tmp/cc-socks/89211.sock" -> "89211" (SPEC.md §8: "the path
    // embeds the CC pid").
    let stem = std::path::Path::new(&socket)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&socket);
    if !stem.is_empty() && stem.chars().all(|c| c.is_ascii_digit()) {
        stem.to_string()
    } else {
        // FALLBACK-OK: SPEC.md §8 — "Use that pid (or a hash of the socket
        // path)" when the filename isn't a bare pid.
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        socket.hash(&mut hasher);
        format!("h{:x}", hasher.finish())
    }
}
