# T03 Claude Adapter Design

## Problem Model

T02 produces a typed, versioned `ClaudeIpcEnvelope` containing only the
observed Claude lifecycle metadata. T03 must turn those envelopes into the
existing provider-neutral `SessionEvent` stream without allowing Claude wire
types, raw hook payloads, or transcript data into the core.

The invariants are: every Claude identity uses `HarnessKind("claude")`; every
accepted non-terminal envelope produces one complete snapshot; `SessionEnd`
produces one `Gone` event and removes the session; snapshots contain only
metadata supported by T01c; and a malformed or unsupported wire envelope cannot
kill the adapter or emit a partial state.

## Discovery Memo

### Volatility Map

Claude event names and optional metadata are the likely changing surface. T02's
hook module owns extraction and the T03 wire decoder owns the versioned local
envelope, containing that volatility before it reaches the shared snapshot
model. The core `SessionEvent` and `SessionSnapshot` shapes are stable and must
not acquire Claude fields.

### Core And Shell

Pure logic is envelope validation and the event-to-session-state transition.
The shell is the Tokio receiver loop and project identity resolution through
the existing filesystem/git cache. Snapshot construction is the single output
boundary between them.

### Failure Domains

Malformed or unsupported envelopes are category-only drops. A project directory
that cannot be resolved falls back to the existing uncanonicalized project
identity behavior so one bad session cannot stop other adapters. A closed
consumer channel ends the adapter task. No failure may affect Claude execution.

### Data Lifecycle

Typed envelope -> protocol validation -> Claude session identity -> project
resolution -> lifecycle state transition -> complete provider-neutral snapshot
or tombstone.

### State Ownership

The Claude adapter owns only live, in-memory Claude session state and its
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

T03 does not open the socket or start the process listener. T04 supplies typed
envelopes through the adapter channel after decoding bytes with the public T03
decoder.

## Candidate Decompositions

### Candidate A: One Claude Module

Put the receiver loop, wire decoder, state map, and snapshot mapping in
`claude/mod.rs`. This is simple initially, but transport decoding and state
policy would change for different reasons and the module would become a second
large boundary file.

### Candidate B: Adapter, Wire, And State Layers

Keep `mod.rs` responsible for the `HarnessAdapter` and channel loop, `wire.rs`
responsible for decoding the versioned envelope into T02 types, and `state.rs`
responsible for pure session transitions and snapshot construction. This
contains Claude protocol changes, makes state tests fixture-only, and keeps the
public surface small. It costs two small modules but each has one reason to
change.

### Candidate C: Listener-Owned Mapping

Have T04's listener update shared `SessionEvent` state directly and leave T03
as a thin type wrapper. This minimizes T03 code but leaks Claude lifecycle
semantics into runtime wiring and makes the adapter boundary impossible to test
without the listener.

## Recommendation

Choose Candidate B. It is the simplest decomposition that preserves the real
trust boundary: wire decoding and Claude lifecycle knowledge stay together in
the Claude module, pure transitions are independently testable, and T04 only
needs a sender plus a decoder. Candidate A loses a useful change boundary;
Candidate C violates adapter ownership and makes runtime code know the event
semantics.

## Component Contracts

### `claude::wire`

- Guarantees exact protocol version, bounded required fields, and allowlisted
  event metadata before constructing a typed envelope.
- Expects one newline-delimited JSON envelope from the T04 listener.
- Drops malformed, unknown-version, unknown-event, and out-of-bounds input with
  a category-only error.
- Does not retain raw JSON or expose rejected fields.

### `claude::state`

- Guarantees `HarnessKind("claude")`, complete metadata-only snapshots, and
  `SessionEnd` tombstones.
- Expects a validated T02 envelope.
- On project resolution failure, uses the existing documented degraded identity
  fallback and continues processing.
- Does not read transcripts, infer unverified lifecycle events, or emit raw hook
  data.

### `ClaudeAdapter`

- Guarantees serialized receipt-order processing from its input channel and
  provider-neutral events on the shared sink.
- Expects T04 to provide a typed envelope channel and to own socket listening.
- Stops cleanly when its input channel closes; a bad envelope cannot panic the
  task.
- Does not own the listener, configuration, persistence, session control, or
  authenticated completeness claims.

## Assumptions And Fallbacks

- T02's public envelope types remain the sole wire model. If the protocol
  changes, update the decoder and tests, not shared snapshots.
- The existing `ProjectIdentityCache<GitDirResolver>` is the correct project
  resolver. If a session directory disappears, use its documented degraded
  identity rather than terminating all adapter tasks.
- The five-minute T01 staleness recommendation is provisional. T03 records
  local receipt time in `last_updated` and leaves active-window reclassification
  to the core; it does not invent adapter-side expiry or removal. T05 owns the
  final policy.
- Only `SessionStart`, `StopFailure`, and `SessionEnd` are supported. Other
  lifecycle behavior remains in the four T05 deferrals.

## Validation Scenarios

- A real T02 envelope delivered through a test Unix socket is decoded, admitted
  by T03, and yields a Claude snapshot with canonical project identity and no
  content fields.
- A `StopFailure` updates an existing session to `NeedsYou` without carrying
  error details.
- A `SessionEnd` removes the session and emits `Gone`, including when it is the
  first event for that native id.
- Duplicate Claude and OpenCode native IDs remain distinct.
- Malformed, unknown-version, and unknown-event wire input emits no event and
  does not terminate the adapter.
- A missing project directory degrades one snapshot without stopping the task.

## Open Questions

T05 must determine successful-turn event support, async viability, startup-gap
behavior, exit-path reliability, subagent identity, and the final stale-session
policy. T04 must decide listener startup order and process-level command
dispatch without changing this adapter contract.
