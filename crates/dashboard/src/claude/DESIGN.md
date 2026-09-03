# Claude Adapter Design (T03)

Promoted from `tasks/2026-09-03-claude-dashboard-t03-adapter.design.md`
(contract version 1). Source of truth for the implementation in this
module: `mod.rs`, `state.rs`, `wire.rs`, consuming T02's unchanged `hook.rs`.

## Problem Model

T02 produces a typed, versioned `ClaudeIpcEnvelope` containing only the
observed Claude lifecycle metadata. T03 turns those envelopes into the
existing provider-neutral `SessionEvent` stream without allowing Claude wire
types, raw hook payloads, or transcript data into the core.

Invariants: every Claude identity uses `HarnessKind("claude")`; every
accepted non-terminal envelope produces one complete snapshot; `SessionEnd`
produces one `Gone` event and removes the session; snapshots contain only
metadata supported by T01c; and a malformed or unsupported wire envelope
cannot kill the adapter or emit a partial state.

## Discovery Memo

### Volatility Map

Claude event names and optional metadata are the likely changing surface.
T02's hook module owns extraction and `wire.rs` owns the versioned local
envelope, containing that volatility before it reaches the shared snapshot
model. The core `SessionEvent`/`SessionSnapshot` shapes are stable and carry
no Claude fields.

### Core And Shell

Pure logic is envelope validation (`wire.rs`) and the event-to-session-state
transition (`state.rs`). The shell is the Tokio receiver loop in `mod.rs`
and project identity resolution through the existing filesystem/git cache.
Snapshot construction is the single output boundary between them.

### Failure Domains

Malformed or unsupported envelopes are category-only drops. A project
directory that cannot be resolved falls back to the existing uncanonicalized
project identity behavior (FALLBACK-OK, same as
`opencode/reconcile.rs::resolve_project_id`) so one bad session cannot stop
other sessions or the adapter. A closed consumer channel ends the adapter
task. No failure may affect Claude execution.

### Data Lifecycle

Typed envelope -> protocol validation -> Claude session identity -> project
resolution -> lifecycle state transition -> complete provider-neutral
snapshot or tombstone.

### State Ownership

The adapter owns only live, in-memory Claude session state and its
project-identity cache. The dashboard core owns the shared snapshot store and
nickname claims. No state is persisted and no history is replayed.

## Layer Stack

```text
T04 listener/command boundary
  -> T02 hook types and local envelope
  -> T03 wire decoder and Claude state transition
  -> provider-neutral SessionEvent
  -> dashboard core LiveState and rendering
```

T03 does not open the socket or start the process listener. T04 supplies
typed envelopes through the adapter channel after decoding bytes with the
public T03 decoder.

## Candidate Decompositions

- **Candidate A: One Claude Module** — receiver loop, wire decoder, state
  map, and snapshot mapping all in `mod.rs`. Simple initially, but transport
  decoding and state policy change for different reasons.
- **Candidate B: Adapter, Wire, And State Layers** — `mod.rs` owns the
  `HarnessAdapter` and channel loop, `wire.rs` owns versioned T02 envelope
  decoding, `state.rs` owns pure session transitions and snapshot
  construction. Contains Claude protocol changes, makes state tests
  fixture-only, keeps the public surface small.
- **Candidate C: Listener-Owned Mapping** — T04 updates shared state
  directly; leaks Claude lifecycle semantics into runtime wiring and makes
  the adapter boundary untestable without the listener.

**Chosen: Candidate B.** Simplest decomposition preserving the real trust
boundary: wire decoding and Claude lifecycle knowledge stay inside the
Claude module, pure transitions are independently testable, and T04 only
needs a sender plus a decoder.

## Component Contracts (as implemented)

### `claude::wire` — `wire.rs`

- Guards the exact protocol version (1), bounded required fields, and the
  allowlisted event metadata before constructing a typed T02 envelope.
- Expects one newline-delimited JSON envelope from the T04 listener.
- Drops malformed, unknown-version, unknown-event, and out-of-bounds input
  with a category-only `DecodeError` that never carries rejected values or
  raw JSON.
- Does not retain raw JSON or expose rejected fields.

### `claude::state` — `state.rs`

- Guarantees `HarnessKind("claude")`, complete metadata-only snapshots, and
  `SessionEnd` tombstones (also for never-seen native ids).
- Expects a validated T02 envelope in receipt order.
- On project resolution failure, uses the existing documented degraded
  identity fallback and continues processing.
- Does not read transcripts, infer unverified lifecycle events, or emit raw
  hook data; does not implement adapter-side expiry (T05 owns final
  staleness policy).

Implementation note vs. the plan's data model: the plan listed `attention`
and `last_updated` as tracked fields. Attention is constant for the two
snapshot events and `last_updated` is always the incoming receipt time, so
`ClaudeTrackedSession` stores only `project_id` and `created_at`; both other
fields are derived per snapshot. No behavior difference.

### `ClaudeAdapter` — `mod.rs`

- Guarantees serialized receipt-order processing from its input channel and
  provider-neutral events on the shared sink; `HarnessKind("claude")`.
- Expects T04 to provide a typed envelope channel (`ClaudeAdapter::channel`)
  and to own socket listening and startup order.
- Stops cleanly when its input channel closes; a bad decoded record (a
  non-version-1 protocol, which the decoder makes unreachable from the wire)
  is a category-only drop that cannot panic the task or emit partial state.
- Does not own the listener, configuration, persistence, session control, or
  authenticated completeness claims.

## Assumptions And Fallbacks

- T02's public envelope types remain the sole wire model. Protocol changes
  update the decoder and tests, not shared snapshots.
- `ProjectIdentityCache<GitDirResolver>` is the correct project resolver. If
  a session directory disappears, its documented degraded identity is used
  rather than terminating the adapter (`FALLBACK-OK` cited at the one call
  site in `state.rs`).
- The five-minute T01 staleness recommendation is provisional. T03 records
  local receipt time in `last_updated` and leaves active-window
  reclassification to the core; it does not invent adapter-side expiry or
  removal. T05 owns the final policy.
- Only `SessionStart`, `StopFailure`, and `SessionEnd` are supported. Other
  lifecycle behavior remains in the four T05 deferrals.

## Validation Scenarios (all covered by tests)

- A real T02 envelope delivered through a test Unix socket is decoded,
  admitted by the adapter, and yields a Claude snapshot with canonical
  project identity and no content fields (`tests/claude_adapter.rs` feature
  test).
- A `StopFailure` updates an existing session to `NeedsYou { question:
  false }` without carrying error details (`state.rs` unit tests +
  integration).
- A `SessionEnd` removes the session and emits `Gone`, including when it is
  the first event for that native id.
- Duplicate starts preserve creation time; first-events pin it.
- Malformed, unknown-version, and unknown-event wire input emits no event
  and does not terminate the adapter (wire unit tests + adapter integration).
- A missing project directory degrades one snapshot without stopping the
  task (fixture resolver in `state.rs`; real resolver in integration).
- Same native id across Claude and OpenCode stays distinct.

## Open Questions

T05 must determine successful-turn event support, async viability,
startup-gap behavior, exit-path reliability, subagent identity, and the
final stale-session policy. T04 must decide listener startup order and
process-level command dispatch without changing this adapter contract.