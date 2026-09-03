//! `dashboard`'s binary entry point (T12 — replaces T08's placeholder).
//! Pairs with the local opencode2 server the same way the existing MCP
//! bridge does (`opencode_client::pair`, paired via the local password
//! file, no MCP process required — `overview.md` R1.2), starts the
//! opencode adapter (T09), and hands its event channel to the interactive
//! shell (T12, `dashboard::shell::run`), which owns the terminal from that
//! point on.

use std::process::ExitCode;

use dashboard::{HarnessAdapter, OpencodeAdapter};

/// Icon mode: `--icons=plain` (or env `DASHBOARD_ICONS=plain`) opts out of
/// Nerd Font glyphs for terminals without a patched font. Nerd Font is the
/// default — most terminal setups that run a dashboard like this already
/// carry one (tmux/starship/lazygit users overwhelmingly do), and the
/// plain fallback exists for the terminals that don't rather than the
/// other way around. A CLI flag beats trying to auto-detect font support:
/// there's no reliable terminal-side signal for "this font is patched,"
/// so guessing would silently show tofu boxes instead of failing loud.
fn resolve_icon_mode() -> dashboard::mosaic::palette::IconMode {
    use dashboard::mosaic::palette::IconMode;
    let from_flag = std::env::args().any(|a| a == "--icons=plain" || a == "--no-nerd-font");
    let from_env = std::env::var("DASHBOARD_ICONS")
        .map(|v| v.eq_ignore_ascii_case("plain"))
        .unwrap_or(false);
    if from_flag || from_env {
        IconMode::Plain
    } else {
        IconMode::Nerd
    }
}

fn main() -> ExitCode {
    // T04: the exact first argument `claude-hook` selects the hook helper
    // before icon-mode resolution, OpenCode pairing, or any TUI startup.
    // The helper runs in its own short-lived runtime (claude.md R11/R16).
    if is_claude_hook_command() {
        return dashboard::claude::command::ClaudeHookCommand::run();
    }

    dashboard::mosaic::palette::set_icon_mode(resolve_icon_mode());

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("dashboard: failed to start async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Pairing/health-check happens before the terminal is ever touched, so
    // a connection failure prints a plain error to a plain terminal instead
    // of failing behind an already-entered alternate screen.
    let client = match rt.block_on(connect()) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("dashboard: {e}");
            return ExitCode::FAILURE;
        }
    };

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    // T04: bind the Claude listener and start the Claude adapter before
    // either the Claude or OpenCode adapter work begins. A bind/path failure
    // disables only Claude monitoring; the OpenCode dashboard continues
    // unchanged (FALLBACK-OK: claude.md R16 — listener unavailable is a
    // harmless, category-only outcome).
    let mut claude_handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    if let Err(error) = start_claude_listener(&rt, tx.clone(), &mut claude_handles) {
        eprintln!(
            "dashboard: claude listener unavailable ({})",
            error.as_str()
        );
    }

    let adapter = Box::new(OpencodeAdapter::new(client));
    // `HarnessAdapter::run` spawns its own background tasks via an internal
    // `tokio::spawn`, which needs an active runtime context on this thread
    // even though `main` isn't `async` itself. `rt.enter()` provides that
    // for just this call, scoped to this block so the guard (which borrows
    // `rt`) is gone before `rt` itself is dropped further down.
    let _adapter_handle = {
        let _entered = rt.enter();
        adapter.run(tx)
    };

    // Blocking, not async: the shell owns this thread with a classic
    // poll/tick terminal loop while the adapter's own tasks keep running on
    // the runtime's other worker threads (`shell::app`'s own doc comment
    // explains why).
    let result = dashboard::shell::run(rx);

    // T04: stop the Claude listener and adapter tasks before the runtime
    // drops so their socket cleanup and shutdown are deterministic
    // (claude.md R16).
    for handle in claude_handles {
        handle.abort();
    }

    // The dashboard is exiting either way; tear the adapter's background
    // tasks down with the runtime rather than leaving them orphaned.
    drop(rt);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("dashboard: {e}");
            ExitCode::FAILURE
        }
    }
}

/// T04: the hook helper selects on the exact first argument only
/// (`claude.md` R11).
fn is_claude_hook_command() -> bool {
    std::env::args().nth(1).as_deref() == Some(dashboard::claude::command::HOOK_COMMAND)
}

/// T04: bind the Claude user-scoped listener and start both Claude tasks
/// (adapter + listener) before the OpenCode adapter, then return the bind
/// category on failure — the OpenCode dashboard continues either way.
fn start_claude_listener(
    rt: &tokio::runtime::Runtime,
    sink: tokio::sync::mpsc::UnboundedSender<dashboard::SessionEvent>,
    handles: &mut Vec<tokio::task::JoinHandle<()>>,
) -> Result<(), dashboard::claude::listener::ListenerError> {
    let listener = dashboard::claude::listener::ClaudeListener::bind()?;
    let (claude_tx, claude_adapter) = dashboard::claude::ClaudeAdapter::channel();
    // Both `run` calls spawn background tasks, which needs an active runtime
    // context on this thread; `rt.enter()` provides it for this block.
    let _entered = rt.enter();
    // The adapter task pushes provider-neutral events onto the shared sink —
    // the same channel the OpenCode adapter uses, consumed by the shell.
    handles.push(Box::new(claude_adapter).run(sink));
    handles.push(listener.run(claude_tx));
    Ok(())
}

async fn connect() -> Result<opencode_client::Client, Box<dyn std::error::Error + Send + Sync>> {
    let bin = opencode_client::resolve_bin();
    let creds = opencode_client::pair(&bin).await?;
    let client = opencode_client::Client::new(bin, creds);
    client.health().await.map_err(|e| {
        format!("GET /api/health failed: {e} — is `opencode2 service start` running?")
    })?;
    Ok(client)
}
