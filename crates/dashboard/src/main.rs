//! `dashboard`'s binary entry point (T12 — replaces T08's placeholder).
//! Pairs with the local opencode2 server the same way the existing MCP
//! bridge does (`opencode_client::pair`, paired via the local password
//! file, no MCP process required — `overview.md` R1.2), starts the
//! opencode adapter (T09), and hands its event channel to the interactive
//! shell (T12, `dashboard::shell::run`), which owns the terminal from that
//! point on.

use std::process::ExitCode;

use dashboard::{HarnessAdapter, OpencodeAdapter};

fn main() -> ExitCode {
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

async fn connect() -> Result<opencode_client::Client, Box<dyn std::error::Error + Send + Sync>> {
    let bin = opencode_client::resolve_bin();
    let creds = opencode_client::pair(&bin).await?;
    let client = opencode_client::Client::new(bin, creds);
    client.health().await.map_err(|e| {
        format!("GET /api/health failed: {e} — is `opencode2 service start` running?")
    })?;
    Ok(client)
}
