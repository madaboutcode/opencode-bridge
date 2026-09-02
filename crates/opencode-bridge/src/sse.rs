//! Global SSE consumer (SPEC.md §1/§7): one connection to `GET /api/event`
//! for every session. Demuxes by `sessionID`, and on a tracked session's
//! terminal `session.execution.*` event fetches the final output and fires
//! the CC callback. Auto-reconnects with backoff and a read/idle timeout;
//! each (re)connect first reconciles any tracked session that went
//! terminal while disconnected. A separate periodic sweep runs the same
//! reconcile on a timer, independent of SSE health — SSE is a latency
//! optimization, not the correctness mechanism (SPEC.md §7).
//!
//! The raw transport (opening `/api/event`, decoding eventsource frames
//! into a typed event, the idle-read timeout) lives in
//! `opencode_client::sse` — this module owns everything MCP-specific:
//! reconnect/backoff policy, the tracked-session registry, interpreting
//! `session.execution.*` as a terminal `Status`, and firing the CC
//! callback.

use std::sync::Arc;
use std::time::Duration;

use opencode_client::sse::{Event, EventStream};

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
        linfo!("sse", "connecting to /api/event");
        match connect_and_consume(&state).await {
            Ok(()) => {
                linfo!("sse", "stream closed by server, reconnecting");
                backoff = Duration::from_millis(500);
            }
            Err(e) => {
                lwarn!("sse", "connection error: {e}, retrying in {backoff:?}");
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
    linfo!("sse", "connected");

    // Missed-event guard (SPEC.md §7.2): catch any tracked session that
    // went terminal while we were disconnected, before reading the fresh
    // stream.
    reconcile(state).await;

    // opencode_client::sse (SPEC.md §7.8) handles multi-line `data:`
    // fields, `:` comment keepalives, frames split across TCP chunks, and
    // the idle-read timeout — a hand-rolled `data: ` line splitter breaks
    // on all three.
    let mut stream = EventStream::new(resp);
    loop {
        match stream.next(SSE_READ_TIMEOUT).await? {
            Some(Ok(event)) => handle_event(state, event).await,
            Some(Err(e)) => {
                // FALLBACK-OK: SPEC.md §1 doesn't enumerate malformed SSE
                // frames; one bad frame must not kill the consumer for
                // every tracked session.
                lwarn!("sse", "skipping unparseable event frame: {e}");
            }
            None => {
                linfo!("sse", "stream ended by server");
                return Ok(());
            }
        }
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
                    Ok(status) => {
                        // Terminal state found by the backstop, not the live
                        // stream — source=reconcile distinguishes the two in
                        // the log (a missed SSE event shows up only here).
                        linfo!(
                            "sse",
                            "terminal via reconcile session={session_id} outcome={outcome} source=reconcile"
                        );
                        complete_session(state, &session_id, status).await
                    }
                    Err(e) => {
                        // FALLBACK-OK: SPEC.md §1 enumerates exactly
                        // succeeded/failed/interrupted; an unrecognized value
                        // means the API contract changed. Log loudly and
                        // leave this one session tracked as running rather
                        // than crash the consumer for every other session.
                        lwarn!("sse", "{e}, leaving {session_id} tracked as running");
                    }
                }
            }
            Err(e) => {
                // FALLBACK-OK: SPEC.md §7.3 — sweeps are a best-effort
                // catch-up pass; a transient failure here is retried on the
                // next sweep (reconnect or periodic).
                lwarn!("sse", "reconcile failed to fetch session {session_id}: {e}");
            }
        }
    }
}

async fn handle_event(state: &AppState, event: Event) {
    // DEBUG: record every session-scoped frame (type + sessionID + durable
    // seq) so the full ordered event sequence for a session is
    // reconstructable from the log. This is the instrument for the
    // "completion detected while the agent kept going" question: if a
    // session emits execution.succeeded and then MORE frames, the sequence
    // shows it. Gated to debug so the default run stays cheap.
    if event.kind.starts_with("session.") {
        ldebug!(
            "event",
            "type={} session={} seq={}",
            event.kind,
            event.session_id.as_deref().unwrap_or("-"),
            seq_str(&event)
        );
    }

    let Some(suffix) = event.kind.strip_prefix("session.execution.") else {
        return; // not a turn-terminal event family — nothing to do
    };
    let Ok(status) = Status::from_outcome(suffix) else {
        return; // e.g. "session.execution.started" — not a terminal suffix
    };
    let Some(session_id) = event.session_id.as_deref() else {
        lwarn!(
            "sse",
            "{} event missing data.sessionID, ignoring",
            event.kind
        );
        return;
    };
    // INFO: a terminal event reached us on the live stream. Logged distinctly
    // from the complete_session decision below so the log shows both the
    // ordering (event → decision) and the source (sse vs reconcile).
    linfo!(
        "sse",
        "terminal event type={suffix} session={session_id} seq={} source=sse",
        seq_str(&event)
    );
    complete_session(state, session_id, status).await;
}

/// `durable.seq` as a string (or "-"), for the event-sequence log lines.
/// opencode stamps a monotonic per-session seq on each frame; logging it
/// lets a reader order events and spot gaps even if lines interleave.
fn seq_str(event: &Event) -> String {
    event
        .seq
        .map(|s| s.to_string())
        .unwrap_or_else(|| "-".to_string())
}

async fn complete_session(state: &AppState, session_id: &str, status: Status) {
    let Some(tracked) = state.registry.claim_notification(session_id) else {
        // not tracked, already notified for this turn, or claimed up front
        // by a wait=true call (see tools.rs wait_and_finish). Logged so a
        // "why didn't I get notified?" / "notified twice?" question can be
        // answered from the log alone.
        ldebug!(
            "sse",
            "complete skipped session={session_id} status={} (untracked or already claimed)",
            status.as_str()
        );
        return;
    };

    let turn = match state.client.final_turn(session_id).await {
        Ok(t) => t,
        Err(e) => {
            // FALLBACK-OK: SPEC.md §7 — the terminal outcome is real and
            // must still be recorded even if fetching the message text
            // failed; we still notify, just without the output/error text.
            lwarn!(
                "sse",
                "failed to fetch final turn for {session_id}: {e} — notifying without body"
            );
            opencode_client::FinalTurn::default()
        }
    };

    state
        .registry
        .set_result(session_id, status, turn.text.clone());

    linfo!(
        "sse",
        "complete session={session_id} status={} notify={} has_text={} has_error={}",
        status.as_str(),
        tracked.notify,
        turn.text.is_some(),
        turn.error.is_some()
    );

    if tracked.notify {
        let outcome = status.as_str();
        // Prefer real output; else the failure reason (so the caller learns
        // *why* an empty-output turn failed — e.g. a provider 402); else a
        // bare "no output" note.
        let text = match (&turn.text, &turn.error) {
            (Some(text), _) => format!(
                "opencode session {session_id} finished ({outcome}): {}",
                cap_notify_text(text, session_id)
            ),
            (None, Some(error)) => format!(
                "opencode session {session_id} finished ({outcome}): {}",
                cap_notify_text(error, session_id)
            ),
            (None, None) => {
                format!("opencode session {session_id} finished ({outcome}) with no output")
            }
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
