//! Hand-rolled MCP stdio transport (SPEC.md §4/§7.6). Newline-delimited
//! JSON-RPC 2.0 on stdin/stdout. Each request is dispatched on its own
//! task so a slow `wait=true` call never blocks another concurrent
//! `tools/call` (CC does issue them concurrently) or even blocks us from
//! reading the next stdin line. All tasks funnel their reply through one
//! writer task/channel so concurrent replies never interleave bytes on
//! stdout — stdout carries only protocol frames, every log line goes to
//! stderr via `eprintln!`.

use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use crate::state::AppState;
use crate::tools;

pub async fn serve_stdio(state: Arc<AppState>) {
    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();

    let (tx, rx) = mpsc::unbounded_channel::<String>();
    let writer = tokio::spawn(run_writer(rx));

    loop {
        let line = match lines.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => {
                eprintln!("[bridge] mcp: stdin closed, shutting down");
                break;
            }
            Err(e) => {
                eprintln!("[bridge] mcp: stdin read error: {e}");
                break;
            }
        };
        if line.trim().is_empty() {
            continue; // blank line — nothing to parse
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                // FALLBACK-OK: SPEC.md §4 doesn't cover malformed input
                // lines; there's no `id` to reply to, so log and keep
                // serving rather than tear down the whole stdio loop.
                eprintln!("[bridge] mcp: dropping unparseable line ({e}): {line}");
                continue;
            }
        };

        let state = state.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            if let Some(response) = handle_request(&state, request).await {
                // FALLBACK-OK: SPEC.md §7.6 — send only fails if the writer
                // task already exited (e.g. a prior stdout error); nothing
                // to recover, the process is shutting down anyway.
                let _ = tx.send(response.to_string());
            }
        });
    }

    // Drop our own sender; the channel closes once every in-flight
    // request task finishes and drops its clone, so this waits for
    // outstanding tools/call work to flush its reply before exiting.
    drop(tx);
    let _ = writer.await;
}

/// The only task that touches stdout — guarantees replies from concurrent
/// request tasks never interleave mid-line (SPEC.md §7.6/§7.7).
async fn run_writer(mut rx: mpsc::UnboundedReceiver<String>) {
    let mut stdout = tokio::io::stdout();
    while let Some(line) = rx.recv().await {
        if let Err(e) = write_line(&mut stdout, &line).await {
            eprintln!("[bridge] mcp: stdout write error: {e}");
            break;
        }
    }
}

async fn write_line(stdout: &mut tokio::io::Stdout, text: &str) -> std::io::Result<()> {
    stdout.write_all(text.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await
}

/// Returns `None` for JSON-RPC notifications (no `id`) — per spec, those
/// never get a response, success or error.
async fn handle_request(state: &Arc<AppState>, request: Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");

    match method {
        "initialize" => {
            // MCP negotiation: echo the client's requested protocolVersion
            // when it advertises one and we recognize it. This is the
            // spec-correct "we both support it, use it" path. We don't
            // currently negotiate DOWN — any client requesting a version
            // we don't recognize gets the default we know works. Future
            // versions we add support for can be appended to the allowlist
            // below.
            let requested = request
                .pointer("/params/protocolVersion")
                .and_then(Value::as_str);
            let negotiated = match requested {
                Some(v) if is_supported_protocol_version(v) => v.to_string(),
                _ => DEFAULT_PROTOCOL_VERSION.to_string(),
            };
            reply_opt(
                id,
                json!({
                    "protocolVersion": negotiated,
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "opencode-bridge", "version": "0.1.0"}
                }),
            )
        }
        "notifications/initialized" => None, // notification — no reply per JSON-RPC 2.0
        "tools/list" => reply_opt(id, json!({"tools": tools::definitions()})),
        "tools/call" => {
            // handle_tools_call owns its id (to embed in the JSON-RPC
            // reply); reply_opt needs the same id to decide whether to
            // emit at all (notification-style requests get no reply).
            let inner = handle_tools_call(state, id.clone(), &request).await;
            reply_opt(id, inner)
        }
        "" => {
            eprintln!("[bridge] mcp: request has no method: {request}");
            id.map(|id| error_reply(id, -32600, "Invalid Request"))
        }
        other => {
            eprintln!("[bridge] mcp: unknown method: {other}");
            id.map(|id| error_reply(id, -32601, "Method not found"))
        }
    }
}

/// Protocol versions we can serve. Add to this list as we support newer
/// MCP revisions. The default below is what we fall back to when a client
/// requests a version we don't recognize.
const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";

fn is_supported_protocol_version(v: &str) -> bool {
    // Today: only the version we shipped against. Extend when we add
    // support for newer MCP revisions — keeping this an explicit allowlist
    // (rather than "anything") makes a future spec-bump an intentional
    // decision.
    v == DEFAULT_PROTOCOL_VERSION
}

async fn handle_tools_call(state: &Arc<AppState>, id: Option<Value>, request: &Value) -> Value {
    let params = request.get("params").cloned().unwrap_or(Value::Null);
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    if name.is_empty() {
        return reply(
            id,
            json!({"content": [{"type": "text", "text": "tools/call missing \"name\""}], "isError": true}),
        );
    }
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match tools::call(state, name, args).await {
        Ok(result) => reply(
            id,
            json!({"content": [{"type": "text", "text": result.to_string()}]}),
        ),
        Err(e) => reply(
            id,
            json!({"content": [{"type": "text", "text": e.to_string()}], "isError": true}),
        ),
    }
}

/// Wraps `reply` to also enforce JSON-RPC 2.0's "no response for
/// notifications" rule. A request with no `id` is a notification and gets
/// no reply, even if the method is one that would otherwise expect a
/// response — except `notifications/initialized`, which we never reply to
/// explicitly.
fn reply_opt(id: Option<Value>, result: Value) -> Option<Value> {
    id.map(|id| reply(Some(id), result))
}

fn reply(id: Option<Value>, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error_reply(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}
