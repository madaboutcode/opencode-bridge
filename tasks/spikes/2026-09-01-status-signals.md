# Spike: observable opencode2 signals → dashboard statuses

Date: 2026-09-01
Sources: spike doc (2026-08-27), SPEC.md §1 + GET /api/session/{id}, src/sse.rs, live SSE sample

## Event types seen or documented

### Seen live (8-second sample, server was busy)

| Event type | Observed in sample |
|---|---|
| `server.connected` | yes |
| `session.reasoning.started` | yes |
| `session.reasoning.delta` | yes |
| `session.reasoning.ended` | yes |
| `session.tool.input.ended` | yes |
| `session.tool.called` | yes |
| `session.tool.progress` | yes |
| `session.tool.success` | yes |
| `session.step.ended` | yes |
| `session.usage.updated` | yes |
| `session.text.delta` | yes |
| `shell.created` | yes |
| `shell.exited` | yes |
| `shell.deleted` | yes |

### Documented but not seen in this sample

| Event type | Source |
|---|---|
| `session.created` | spike doc, SPEC.md |
| `session.inbox.enqueued` | SPEC.md §1 lifecycle |
| `session.execution.started` | spike doc, SPEC.md §1 lifecycle |
| `session.instructions.updated` | SPEC.md §1 lifecycle |
| `session.inbox.delivered` | SPEC.md §1 lifecycle |
| `session.text.started` | SPEC.md §1 lifecycle |
| `session.text.ended` | SPEC.md §1 lifecycle |
| `session.tool.input.started` | spike doc (spike B) |
| `session.execution.succeeded` | spike doc, SPEC.md §1 lifecycle |
| `session.execution.failed` | spike doc, SPEC.md §1 lifecycle |
| `session.execution.interrupted` | SPEC.md §1 lifecycle |

## REST facts relevant to status

`GET /api/session/{id}` returns:

- `outcome` — present only after terminal event: `"succeeded"`, `"failed"`, `"interrupted"`. **Absent** while running.
- `time.idle` — epoch-ms timestamp, present after turn goes idle (confirmed on an `"interrupted"` session: `time.idle: 1788265185945`). Not present while actively running.
- `time.created`, `time.updated` — always present.
- `cost`, `tokens` — always present, cumulative.

Running session sample (no `outcome`, no `time.idle`):
```json
{"time":{"created":1788265310212,"updated":1788265310228}}
```

Completed session sample (`outcome` + `time.idle` present):
```json
{"outcome":"interrupted","time":{"created":..., "updated":..., "idle":1788265185945}}
```

## What src/sse.rs already handles (event type names)

The bridge currently dispatches on exactly one family: **`session.execution.*`** (terminal suffixes `succeeded`/`failed`/`interrupted`). All other event types are parsed only for debug logging (`session.*` prefix → log type+sessionID+seq). The bridge does **not** act on reasoning, text, tool, or shell events — it ignores them (silently, after the debug log).

## Status-to-signal mapping

| candidate status | observable signals (event types / session fields) | confidence (observed / documented / inferred) | gap |
|---|---|---|---|
| **doing** | SSE: `session.tool.called`, `session.tool.progress`, `session.tool.success`, `session.tool.input.ended`, `shell.created`, `shell.exited`, `session.step.ended` (finish="tool-calls" means tool was the step, not final text). REST: `time.updated` advancing while no `outcome`. | **observed** — saw all of these in live sample; tool/shell events are the clearest "doing" signal | Bridge currently ignores all of these. To use for dashboard, the SSE consumer would need to track tool/shell events per session. |
| **thinking** | SSE: `session.reasoning.started`, `session.reasoning.delta`, `session.reasoning.ended`. These fire in bursts between tool calls and before text output. REST: no dedicated field — only inferable from `time.updated` advancing with no tool events nearby. | **observed** — saw reasoning started/delta/ended in live sample | Bridge currently ignores these. "Thinking" is distinguishable from "doing" only via SSE event types (reasoning vs tool/shell). |
| **waiting** | SSE: terminal `session.execution.succeeded` / `.failed` / `.interrupted` (the turn is done, server is idle, waiting for next prompt). REST: `outcome` field present on `GET /api/session/{id}`. `time.idle` timestamp present. | **observed** — confirmed on live session with `outcome:"interrupted"` + `time.idle` present | Bridge already handles this via `session.execution.*` terminal events. This is the only status the bridge currently reacts to (to fire the callback). Gap: the bridge marks it "done" for callback purposes but doesn't expose it as a dashboard "waiting" state. |
| **stalled** | **UNKNOWN** — the wire provides no signal that distinguishes "waiting for user input" (idle after success) from "stuck / hung / unresponsive". `time.idle` just records *when* it went idle. `time.updated` advancing on a running session means it's alive, but a frozen session also has a stale `time.updated` with no way to tell "healthy idle" from "dead". | **inferred (negative)** — no wire field exists for this | **This is a gap in the wire, not a gap in the bridge.** opencode2 does not emit a "health check" or "keepalive" event for running sessions. A stalled detection would require either: (a) a timeout heuristic (no SSE event for N seconds on a tracked running session), or (b) a server-side signal that doesn't exist yet. The bridge's `SSE_READ_TIMEOUT` (60s, for TCP half-open detection) is the closest analog but it's a connection-level timeout, not a per-session health signal. |
| **idle vs active** | REST: absence of `outcome` on `GET /api/session/{id}` = active (turn in progress). Presence of `outcome` = idle. `time.idle` confirms idle timestamp. SSE: `session.execution.started` begins the active period; any `session.execution.*` terminal ends it. | **observed** — confirmed live: running session has no `outcome`, completed session has `outcome` + `time.idle` | Clean binary split available via REST. SSE also gives this (started → terminal). No gap — this is the strongest signal pair on the wire. |

## Summary of what the wire actually gives

1. **doing vs thinking** — distinguishable only through SSE event types (tool/shell events = doing, reasoning events = thinking). No REST field differentiates them.
2. **idle vs active** — clean binary via REST (`outcome` present or not) and SSE (between `execution.started` and terminal `execution.*`).
3. **waiting** — confirmed by terminal `session.execution.*` + `outcome` on REST. Already handled by bridge.
4. **stalled** — **no wire signal exists.** Would require heuristic (time since last event on a running session) or a new server-side signal.
5. **The bridge currently only consumes terminal `session.execution.*` events.** All intermediate events (reasoning, text, tool, shell) flow through `handle_event_line` and are discarded after debug logging. To power a dashboard, the SSE consumer would need to be extended to track and expose intermediate state per session.

## Addendum 2026-09-01: live SSE tool-event payload shapes (captured, not assumed)

45s live capture from the running server (`/tmp/oc_event_sample.txt`, port 49374). Confirms field names for the "current action" line dashboard requirements (R6.3/R6.4):

- **`session.tool.called` has no tool-name field.** Its `data` is `{sessionID, assistantMessageID, id, input, executed}` — `input` is the raw args object, shape varies by tool, no `name` key.
- **`session.tool.input.started` is the only event carrying `name`**, keyed by the same `data.id` (call ID) that `tool.called`/`tool.success` use. Fires before `input.ended`/`tool.called`.
- To render a per-tool action line, a consumer must **join `input.started` (name) with `tool.called` (input args) by `id`**, scoped per `sessionID`. A single call_id → name map per tracked session, populated on `input.started` and read on `tool.called`, is enough — no need to persist past the call.
- Two tool names observed, with confirmed arg shapes:
  - `shell`: `input.command` (string), `input.workdir`, `input.timeout`.
  - `edit`: `input.path`, `input.oldString`, `input.newString`.
- Not observed in this window: `read`, `write`, `grep`, `glob`, `webfetch`, `todowrite`, `subagent`/`task`, or any other tool name. The wire-behavior spike (2026-08-27) separately confirmed `name:"subagent"` appears in *message content* tool parts (not SSE) with `state.content` wrapping `<subagent sessionID=... state=...>`; unconfirmed whether SSE `tool.input.started` uses the same name for subagent delegation on the live stream.
