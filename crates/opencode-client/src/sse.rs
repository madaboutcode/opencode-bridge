//! Raw transport for opencode's global SSE stream (`GET /api/event`,
//! SPEC.md §1/§7.8): opens via `crate::Client::events`, then decodes each
//! frame into a typed [`Event`]. This module understands only the wire
//! transport and the envelope shape opencode uses across event kinds
//! (`type`, `data.sessionID`, `durable.seq`) — reconnect policy, backoff,
//! and any domain interpretation of a specific `kind`/`data` payload (e.g.
//! treating `session.execution.*` as terminal states) belong to the
//! consumer, not this crate (R1.1: no MCP dependency here).

use std::time::Duration;

use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::Value;

use crate::error::Result;

/// One decoded frame from `/api/event`. `kind` is opencode's own event type
/// string, e.g. `"session.execution.succeeded"`. `session_id` and `seq` are
/// hoisted out of `data`/`durable` because most event kinds carry them at
/// the same two JSON pointers — callers interpret `kind` and the rest of
/// `data` for their own domain.
#[derive(Debug, Clone)]
pub struct Event {
    pub kind: String,
    pub session_id: Option<String>,
    pub seq: Option<i64>,
    pub data: Value,
}

/// Decodes frames off an already-open `/api/event` response one at a time.
pub struct EventStream {
    inner: eventsource_stream::EventStream<
        std::pin::Pin<Box<dyn futures_util::Stream<Item = reqwest::Result<bytes::Bytes>> + Send>>,
    >,
}

impl EventStream {
    /// `resp` must be the response returned by `Client::events()`. Consumes
    /// it, since only one reader can drain the underlying byte stream.
    pub fn new(resp: reqwest::Response) -> Self {
        let boxed: std::pin::Pin<
            Box<dyn futures_util::Stream<Item = reqwest::Result<bytes::Bytes>> + Send>,
        > = Box::pin(resp.bytes_stream());
        Self {
            inner: boxed.eventsource(),
        }
    }

    /// Reads the next frame and decodes it.
    ///
    /// - `Ok(Some(Ok(event)))` — a decoded event.
    /// - `Ok(Some(Err(e)))` — a frame whose `data:` payload wasn't valid
    ///   JSON; opencode's wire format doesn't document malformed frames, so
    ///   the caller decides whether/how to log this rather than the
    ///   transport silently dropping it.
    /// - `Ok(None)` — the server closed the stream.
    /// - `Err(_)` — a transport error, or `idle_timeout` elapsed with no
    ///   data at all (a half-open TCP connection otherwise produces no
    ///   bytes forever).
    ///
    /// A frame that parses as JSON but carries no recognizable `type` field
    /// is skipped internally and never surfaced — opencode's wire format
    /// doesn't document that shape as meaningful.
    pub async fn next(
        &mut self,
        idle_timeout: Duration,
    ) -> Result<Option<std::result::Result<Event, serde_json::Error>>> {
        loop {
            let frame = match tokio::time::timeout(idle_timeout, self.inner.next()).await {
                Ok(Some(Ok(frame))) => frame,
                Ok(Some(Err(e))) => return Err(format!("SSE stream error: {e}").into()),
                Ok(None) => return Ok(None),
                Err(_elapsed) => {
                    return Err(
                        format!("no data for {idle_timeout:?} (half-open connection?)").into(),
                    );
                }
            };
            match serde_json::from_str::<Value>(&frame.data) {
                Ok(envelope) => {
                    let Some(kind) = envelope.get("type").and_then(Value::as_str) else {
                        continue; // no "type" field — not a shape we understand, skip
                    };
                    let session_id = envelope
                        .pointer("/data/sessionID")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let seq = envelope.pointer("/durable/seq").and_then(Value::as_i64);
                    let kind = kind.to_string();
                    return Ok(Some(Ok(Event {
                        kind,
                        session_id,
                        seq,
                        data: envelope,
                    })));
                }
                Err(e) => return Ok(Some(Err(e))),
            }
        }
    }
}
