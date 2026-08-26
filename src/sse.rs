//! Global SSE consumer (SPEC.md §1/§7): one connection to `GET /api/event`
//! for every session. Demuxes by `sessionID`, and on a tracked session's
//! terminal `session.execution.*` event fetches the final output and fires
//! the CC callback. Auto-reconnects with backoff and a read/idle timeout;
//! each (re)connect first reconciles any tracked session that went
//! terminal while disconnected. A separate periodic sweep runs the same
//! reconcile on a timer, independent of SSE health — SSE is a latency
//! optimization, not the correctness mechanism (SPEC.md §7).

use std::sync::Arc;
use std::time::Duration;

use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::Value;

use crate::error::Result;
use crate::registry::Status;
use crate::state::AppState;

/// A half-open TCP connection produces no bytes forever; force a
/// reconnect if we haven't seen anything (data or keepalive) in this long
/// (SPEC.md §7.4).
const SSE_READ_TIMEOUT: Duration = Duration::from_secs(60);

/// Backstop sweep interval, independent of SSE (SPEC.md §7.3). Converts
/// any missed event (half-open TCP, opencode restart, frame parse error)
/// from a hang into a bounded delay.
const PERIODIC_SWEEP_INTERVAL: Duration = Duration::from_secs(60);

/// Injected callback text is capped so a long assistant reply doesn't burn
/// the CC user's context (SPEC.md §7.9).
const NOTIFY_TEXT_CAP: usize = 3000;

pub async fn run(state: Arc<AppState>) {
    let mut backoff = Duration::from_millis(500);
    loop {
        eprintln!("[bridge] sse: connecting to /api/event");
        match connect_and_consume(&state).await {
            Ok(()) => {
                eprintln!("[bridge] sse: stream closed by server, reconnecting");
                backoff = Duration::from_millis(500);
            }
            Err(e) => {
                eprintln!("[bridge] sse: connection error: {e}, retrying in {backoff:?}");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(30));
            }
        }
    }
}

/// Independent backstop: sweeps tracked non-terminal sessions on a timer,
/// regardless of SSE connection health (SPEC.md §7.3).
pub async fn periodic_sweep(state: Arc<AppState>) {
    let mut ticker = tokio::time::interval(PERIODIC_SWEEP_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        reconcile(&state).await;
    }
}

async fn connect_and_consume(state: &Arc<AppState>) -> Result<()> {
    let resp = state.client.events().await?;
    eprintln!("[bridge] sse: connected");

    // Missed-event guard (SPEC.md §7.2): catch any tracked session that
    // went terminal while we were disconnected, before reading the fresh
    // stream.
    reconcile(state).await;

    // eventsource-stream (SPEC.md §7.8) handles multi-line `data:` fields,
    // `:` comment keepalives, and frames split across TCP chunks — a hand-
    // rolled `data: ` line splitter breaks on all three.
    let mut stream = resp.bytes_stream().eventsource();
    loop {
        let event = match tokio::time::timeout(SSE_READ_TIMEOUT, stream.next()).await {
            Ok(Some(Ok(event))) => event,
            Ok(Some(Err(e))) => return Err(format!("SSE stream error: {e}").into()),
            Ok(None) => {
                eprintln!("[bridge] sse: stream ended by server");
                return Ok(());
            }
            Err(_elapsed) => {
                return Err(
                    format!("no data for {SSE_READ_TIMEOUT:?} (half-open connection?)").into(),
                );
            }
        };
        handle_event_line(state, &event.data).await;
    }
}

/// For every tracked session still marked running, ask opencode directly
/// whether it already finished. Catches the reconnect gap and, via
/// `periodic_sweep`, any other missed-event scenario.
async fn reconcile(state: &Arc<AppState>) {
    for session_id in state.registry.running_session_ids() {
        match state.client.get_session(&session_id).await {
            Ok(info) => {
                let Some(outcome) = info.outcome else {
                    continue; // still running — nothing to reconcile
                };
                match Status::from_outcome(&outcome) {
                    Ok(status) => complete_session(state, &session_id, status).await,
                    Err(e) => {
                        // FALLBACK-OK: SPEC.md §1 enumerates exactly
                        // succeeded/failed/interrupted; an unrecognized value
                        // means the API contract changed. Log loudly and
                        // leave this one session tracked as running rather
                        // than crash the consumer for every other session.
                        eprintln!("[bridge] sse: {e}, leaving {session_id} tracked as running");
                    }
                }
            }
            Err(e) => {
                // FALLBACK-OK: SPEC.md §7.3 — sweeps are a best-effort
                // catch-up pass; a transient failure here is retried on the
                // next sweep (reconnect or periodic).
                eprintln!("[bridge] sse: reconcile failed to fetch session {session_id}: {e}");
            }
        }
    }
}

async fn handle_event_line(state: &AppState, json_str: &str) {
    let envelope: Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            // FALLBACK-OK: SPEC.md §1 doesn't enumerate malformed SSE
            // frames; one bad frame must not kill the consumer for every
            // tracked session.
            eprintln!("[bridge] sse: skipping unparseable event frame: {e}");
            return;
        }
    };
    let Some(event_type) = envelope.get("type").and_then(Value::as_str) else {
        return; // frame without a "type" field — not an event shape we understand
    };
    let Some(suffix) = event_type.strip_prefix("session.execution.") else {
        return; // not a turn-terminal event family — nothing to do
    };
    let Ok(status) = Status::from_outcome(suffix) else {
        return; // e.g. "session.execution.started" — not a terminal suffix
    };
    let Some(session_id) = envelope.pointer("/data/sessionID").and_then(Value::as_str) else {
        eprintln!("[bridge] sse: {event_type} event missing data.sessionID, ignoring");
        return;
    };
    complete_session(state, session_id, status).await;
}

async fn complete_session(state: &AppState, session_id: &str, status: Status) {
    let Some(tracked) = state.registry.claim_notification(session_id) else {
        return; // not tracked, already notified for this turn, or claimed
                // up front by a wait=true call (see tools.rs wait_and_finish)
    };

    let output = match state.client.final_output(session_id).await {
        Ok(o) => o,
        Err(e) => {
            // FALLBACK-OK: SPEC.md §7 — the terminal outcome is real and
            // must still be recorded even if fetching the message text
            // failed; we still notify, just without the output text.
            eprintln!("[bridge] sse: failed to fetch final output for {session_id}: {e}");
            None
        }
    };

    state
        .registry
        .set_result(session_id, status, output.clone());

    if tracked.notify {
        let outcome = status.as_str();
        let text = match &output {
            Some(text) => format!(
                "opencode session {session_id} finished ({outcome}): {}",
                cap_notify_text(text, session_id)
            ),
            None => format!("opencode session {session_id} finished ({outcome}) with no output"),
        };
        state.notifier.notify(&text).await;
    }
}

/// Truncates a long assistant reply before it goes into the CC inbox
/// (SPEC.md §7.9). A raw final text can be tens of KB.
fn cap_notify_text(text: &str, session_id: &str) -> String {
    if text.len() <= NOTIFY_TEXT_CAP {
        return text.to_string();
    }
    let mut end = NOTIFY_TEXT_CAP;
    while !text.is_char_boundary(end) {
        end -= 1; // don't split a multi-byte UTF-8 char
    }
    format!(
        "{}… (truncated; call opencode_sessions(session_id=\"{session_id}\") for full output)",
        &text[..end]
    )
}
