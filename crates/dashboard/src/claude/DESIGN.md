# Claude Adapter Design (T03, widened 2026-09-05)

Promoted from `tasks/2026-09-03-claude-dashboard-t03-adapter.design.md`
(contract version 1). Source of truth for the implementation in this
module: `mod.rs`, `state.rs`, `wire.rs`, `hook.rs`.

Widened 2026-09-05 (`tasks/2026-09-05-claude-dashboard-activity-capture.
spec-delta.md`, rounds 1 and 2): the allowlist grew from the original three
lifecycle-only events (`SessionStart`, `StopFailure`, `SessionEnd`) to
round 1's twelve-event activity contract, then to round 2's fifteen-event
contract (advisor review) in `docs/specs/dashboard/claude.md` R13/R14 — tool
calls, results, prompts, assistant text, subagent identity, and permission/
elicitation exit paths, each individually bounded. This file describes the
current, widened contract; where a design note below says "unchanged," it
means unchanged since the cited revision.

## Problem Model

`hook` produces a typed, versioned `ClaudeIpcEnvelope` containing exactly the
R14-allowlisted fields for whichever of the fifteen R13 events fired. `state`
turns those envelopes into the existing provider-neutral `SessionEvent`
stream without allowing Claude wire types, raw hook payloads, or transcript
data into the core.

Invariants: every Claude identity (top-level or subagent) uses
`HarnessKind("claude")`; every event other than `SubagentStop`/`SessionEnd`
produces one complete snapshot for the session it targets; `SubagentStop`/
`SessionEnd` each produce one `Gone` event and remove the targeted session;
snapshot content is exactly what R14's per-event field table allows; and a
malformed or unsupported wire envelope cannot kill the adapter or emit a
partial state.

## Discovery Memo

### Volatility Map

Claude event names and their allowlisted fields are the likely changing
surface — the 2026-09-05 widening from three to twelve to fifteen events is
exactly this kind of change. `hook.rs` owns extraction and truncation/label
validation, `wire.rs` owns the versioned local envelope's decode side,
containing that volatility before it reaches the shared snapshot model. The
core `SessionEvent`/`SessionSnapshot` shapes are stable and carry no Claude
fields.

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

Typed envelope -> protocol validation -> target session identity (top-level,
or a subagent when the event carries `agent_id`) -> project resolution
(shared from an already-tracked parent, or resolved independently) ->
attention/content state transition -> complete provider-neutral snapshot or
tombstone.

### State Ownership

The adapter owns only live, in-memory Claude session state and its
project-identity cache. The dashboard core owns the shared snapshot store and
nickname claims. No state is persisted and no history is replayed.

## Layer Stack

```text
listener/command boundary
  -> hook types and local envelope
  -> wire decoder and Claude state transition
  -> provider-neutral SessionEvent
  -> dashboard core LiveState and rendering
```

This module does not open the socket or start the process listener; the
runtime supplies typed envelopes through the adapter channel after decoding
bytes with the public wire decoder.

## Candidate Decompositions

- **Candidate A: One Claude Module** — receiver loop, wire decoder, state
  map, and snapshot mapping all in `mod.rs`. Simple initially, but transport
  decoding and state policy change for different reasons.
- **Candidate B: Adapter, Wire, And State Layers** — `mod.rs` owns the
  `HarnessAdapter` and channel loop, `wire.rs` owns versioned envelope
  decoding, `state.rs` owns pure session transitions and snapshot
  construction. Contains Claude protocol changes, makes state tests
  fixture-only, keeps the public surface small.
- **Candidate C: Listener-Owned Mapping** — the listener updates shared
  state directly; leaks Claude lifecycle semantics into runtime wiring and
  makes the adapter boundary untestable without the listener.

**Chosen: Candidate B.** Simplest decomposition preserving the real trust
boundary: wire decoding and Claude lifecycle knowledge stay inside the
Claude module, pure transitions are independently testable, and the runtime
only needs a sender plus a decoder. Round 1 added nine event kinds and
subagent routing without changing this shape — every new field lives inside
`hook`'s allowlisted `ClaudeEvent` variants, `wire` mirrors them 1:1, and
`state` gained a routing step (top-level vs. subagent target) ahead of the
same "ensure tracked, mutate, snapshot" flow. Round 2 added three more event
kinds (`PermissionDenied`, `Elicitation`, `ElicitationResult`) and one new
cross-cutting step in that same flow — a generic `tool_use_id`-based clear
that runs before an event's own mapping (see `claude::state` below) — again
without changing the layer shape.

## Component Contracts (as implemented)

### `claude::wire` — `wire.rs`

- Guards the exact protocol version (1), the exact allowlisted fields for
  whichever of the fifteen R13 event kinds the line carries, and the bounded
  required fields, before constructing a typed envelope.
- Expects one newline-delimited JSON envelope from the listener.
- Drops malformed, unknown-version, unknown-event, and out-of-bounds input
  with a category-only `DecodeError` that never carries rejected values or
  raw JSON.
- Does not retain raw JSON or expose rejected fields; does not re-validate
  "(bounded)" text fields' length (the overall envelope size bound already
  gates total wire size) but does re-validate label length, matching
  `hook`'s own discipline.

### `claude::state` — `state.rs`

- Guarantees `HarnessKind("claude")` for every session (top-level and
  subagent); every event but `SubagentStop`/`SessionEnd` yields exactly one
  complete snapshot for the session it targets; `SubagentStop`/`SessionEnd`
  each yield exactly one `Gone` tombstone (also for never-seen ids).
- A subagent session is keyed `"{top_native_id}:{agent_id}"`, carries
  `parent_id = Some(top)`, and shares the parent's already-resolved
  `project_id` when the parent is tracked (resolved independently from the
  event's own `cwd` otherwise — event ordering is not guaranteed, R17).
- A tracked session's `pending_tool_use_id` (set by `PermissionRequest`/
  `Elicitation`, alongside `NeedsYou { question: true }`) clears on any
  subsequent accepted event whose own `tool_use_id` (`ClaudeEvent::
  tool_use_id`) matches it — `PermissionDenied`, `ElicitationResult`,
  `PostToolUse`, and `PostToolUseFailure` all qualify, whichever arrives
  first. This is a generic rule keyed on `tool_use_id` equality, not a
  per-event-pair mapping: round 1's bug was assuming a *specific* next event
  clears a specific prior one, which breaks the moment ordering isn't what
  was assumed (a tool's own `PreToolUse` for a given `tool_use_id` fires
  *before* the permission check, never after, so it is never "the next"
  event a naive pairing would wait for). The clearing step (`state.rs`'s
  `clear_pending_tool_use`) runs before the triggering event's own specific
  mapping, so one event can both clear the pending state and carry its own
  effect in the same step.
- Expects a validated envelope in receipt order.
- On project resolution failure, uses the existing documented degraded
  identity fallback and continues processing.
- Does not read transcripts, infer unobserved events, or emit raw hook data;
  does not implement adapter-side expiry (final staleness policy remains
  deferred, R17).

Implementation note vs. the original three-event data model:
`ClaudeTrackedSession` now stores `attention`, `turn_started`,
`current_action`, `recent_actions`, `last_user_prompt`,
`final_assistant_text`, `parent_id`, and (round 2) `pending_tool_use_id` in
addition to `project_id`/`created_at` — the widened event set maps each
event to a specific mutation of this state rather than a constant snapshot,
so there is real state to carry between events (e.g. a tool event's
`Running{turn_started}` reuses whatever `UserPromptSubmit`/`SubagentStart`
last set; `pending_tool_use_id` is the one round-2 field, read and cleared by
`clear_pending_tool_use` before an event's own mapping runs). `last_updated`
is still never stored: it is always the current event's own receipt time,
passed straight to snapshot construction.

### `ClaudeAdapter` — `mod.rs`

- Guarantees serialized receipt-order processing from its input channel and
  provider-neutral events on the shared sink; `HarnessKind("claude")`.
- Expects the runtime to provide a typed envelope channel
  (`ClaudeAdapter::channel`) and to own socket listening and startup order.
- Stops cleanly when its input channel closes; a bad decoded record (a
  non-version-1 protocol, which the decoder makes unreachable from the wire)
  is a category-only drop that cannot panic the task or emit partial state.
- Does not own the listener, configuration, persistence, session control, or
  authenticated completeness claims.

## Assumptions And Fallbacks

- `hook`'s public envelope types remain the sole wire model. Protocol
  changes update the decoder and tests, not shared snapshots.
- `ProjectIdentityCache<GitDirResolver>` is the correct project resolver. If
  a session directory disappears, its documented degraded identity is used
  rather than terminating the adapter (`FALLBACK-OK` cited at the call sites
  in `state.rs`).
- The five-minute staleness recommendation from the original spike is
  provisional. This module records local receipt time in `last_updated` and
  leaves active-window reclassification to the core; it does not invent
  adapter-side expiry or removal. Final policy remains deferred (R17).
- `Notification`'s many sub-types are deliberately not mapped to distinct
  attention/content changes — only `last_updated` advances. A future need to
  distinguish them is a new mapping decision, not a bug in this one.
- `wire_title`/`files_touched` have no evidence-backed source field on any
  of the fifteen events and stay `None`/empty; inventing a mapping for
  either would violate the same evidence discipline that unblocked this
  widening.
- `PermissionDenied`/`ElicitationResult` have no defined effect beyond the
  generic `pending_tool_use_id` clear — no attention/content mapping of
  their own is invented for them, matching the same "don't map what isn't
  evidenced" discipline applied to `Notification`.

## Validation Scenarios (all covered by tests)

- A real envelope delivered through a test Unix socket is decoded, admitted
  by the adapter, and yields a Claude snapshot with canonical project
  identity (`tests/claude_adapter.rs` feature test).
- Each of the fifteen R13 events round-trips its own R14 fields end to end:
  `hook::parse_hook_input` -> `serialize_envelope` -> `wire::decode_envelope`
  (`hook.rs`/`wire.rs` unit tests), and into a snapshot's attention/content
  fields (`state.rs` unit tests).
- A bounded field longer than `MAX_FIELD_BYTES` is truncated with a marker,
  not dropped; a label longer than `MAX_LABEL_LEN`, or missing where
  required, drops the whole event (`hook.rs` unit tests).
- A large-but-under-cap raw payload (e.g. a 1.5 MiB `PostToolUse` with a
  large `tool_response`) is accepted and truncated, not whole-dropped
  (`hook.rs` unit test, round 2).
- The generic `tool_use_id` exit path clears on both the approval path
  (`PermissionRequest` then a matching `PostToolUse`) and the denial path
  (`PermissionRequest` then a matching `PermissionDenied`), and a mismatched
  `tool_use_id` does not clear an unrelated pending permission (`state.rs`
  unit tests, round 2).
- `transcript_path`/`agent_transcript_path` never survive any event kind,
  and any field not named in R14's table is discarded even on a known event
  (`hook.rs`/`wire.rs` unit tests, `tests/claude_ingress.rs`).
- A tool event carrying `agent_id` creates and updates a distinct subagent
  session, sharing the parent's project identity; `SubagentStop` tombstones
  it independently of the parent; a subagent tool event that arrives before
  `SubagentStart` creates the subagent defensively (`state.rs` unit tests).
- A `StopFailure`/`SessionEnd` on the top-level session behaves exactly as
  before this widening (`state.rs` unit tests + integration).
- Malformed, unknown-version, and unknown-event wire input emits no event
  and does not terminate the adapter (wire unit tests + adapter integration).
- A missing project directory degrades one snapshot without stopping the
  task (fixture resolver in `state.rs`; real resolver in integration).
- Same native id across Claude and OpenCode stays distinct.

## Open Questions

Operational proof — authenticated lifecycle ordering under a real registered
hook, startup-gap/foreground discovery, async-hook viability for successful
sessions, and exit-path reliability — remains open per `claude.md` R17's
`[REVIEW]`; the event/field schema itself is now evidence-backed and is not
part of that remaining gap.