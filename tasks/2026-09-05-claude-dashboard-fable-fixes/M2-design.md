# M2 Design — Claude Dashboard Structural Rewrites

**Status:** amended by advisor adjudication; pending fresh implementation
re-review and gate sealing. The implementation plan is
`tasks/2026-09-05-claude-dashboard-fable-m2.plan.md`.

## Problem Model

M1 made the Claude tile behavior truthful, but the adapter still has two
structural liabilities:

1. `ClaudeTrackedSession` stores `attention`, `turn_started`, and
   `pending_tool_use_id` as independently mutated values. They describe
   overlapping lifecycle facts, so a future event arm can update one and leave
   the others inconsistent.
2. `hook.rs` hand-builds the versioned envelope and `wire.rs` hand-decodes each
   event variant. The two matches are parallel wire schemas that can drift as
   the event set changes.

The release requirement is behavior preservation: ordinary M1 workflows must
remain truthful while the internal sources of truth become smaller and easier
to check. The JSON protocol is already shipped inside this repository and must
remain byte-shape compatible.

## Discovery Memo

### Volatility Map

Claude event names and fields are the volatile boundary. They belong in the
Claude hook/wire types and should be expressed once. Attention transition rules
are also Claude-specific, but the resulting `AttentionState` remains the
provider-neutral boundary.

### Core And Shell

`ClaudeState::process`, its turn-fact projection, and typed conversion are pure
logic. Socket listening and hook delivery remain outside this design. Serde is
used at the wire edge only; raw JSON does not enter state or shared snapshots.

### Failure Domains

Malformed or unsupported wire input must remain a category-only rejection and
must not stop the adapter. A bad transition must affect only the targeted
in-memory session. No failure in either rewrite may affect Claude execution or
other sessions.

### Data Lifecycle

Hook payload -> bounded typed event -> versioned JSON envelope -> typed envelope
decode -> Claude turn facts -> one attention projection -> complete shared
snapshot or tombstone.

### State Ownership

`ClaudeState` owns live per-session facts and is the only mutator. The
projection constructs the public attention value at snapshot time. The serde
types own wire naming and shape; no downstream layer knows the raw JSON map.

## Layer Stack

```text
hook validation and typed event values
  -> serde-derived versioned envelope
  -> Claude state facts and transition rules
  -> one AttentionState projection
  -> provider-neutral SessionSnapshot
```

The serializer and decoder are both at the wire edge. The state layer consumes
typed envelopes and cannot observe serialization details.

## Candidate Decompositions

### Candidate A: Keep the current fields and add assertions

Add debug/test assertions after each event to check that `attention` agrees
with `turn_started` and pending state. This is the smallest diff but leaves two
mutable representations and makes correctness depend on every caller remembering
the assertion convention. It does not solve the structural smell.

### Candidate B: Store facts and project attention

Store `turn_started`, `turn_ended` with its question bit,
`pending_tool_use_id`, and `idle_since`. Every event mutates facts; snapshot
construction calls one projector. `idle_since` is the timestamp of the current
Idle projection and is preserved on no-op paths so the rewrite remains
byte-for-byte compatible with M1 without storing a public `AttentionState`.
This is the selected design: it removes the duplicated public state while
keeping every independently meaningful timestamp and correlation fact.

### Candidate C: Replace all facts with one lifecycle enum

Represent Running, NeedsYou, and Idle as one enum carrying every detail. This
would make public attention direct, but it hides the distinction between a
pending correlation key and the visible state and makes the M1 asymmetry around
turn-start preservation harder to express. It also invites adding rendering
concerns to the lifecycle enum.

**Chosen state design:** Candidate B with the private `idle_since` fact. It is
the simplest design that removes the actual duplication without losing an
independently meaningful fact or changing M1 Idle timestamps.

### Wire Candidate A: Custom serde-backed wire wrapper

Create a separate wire enum and convert it to the domain enum. This gives full
control over unknown variants but introduces a second type with nearly the same
fifteen fields, recreating the duplication in a new location.

### Wire Candidate B: Derive on the existing typed model with a small preflight

Derive `Serialize`/`Deserialize` on the existing envelope/event types, use
serde's internal `kind` tag and explicit field attributes, and inspect only the
raw kind before typed deserialization to retain `UnknownEvent` classification.
This removes the large field mapping while retaining the existing error and
privacy contract. This is selected.

### Wire Candidate C: Deserialize into `serde_json::Value` and keep conversion

Retain the existing generic map as the source of truth and use serde only for
individual fields. This changes little and provides no structural benefit; it
is rejected.

## Chosen Component Contracts

### T04 — turn facts and projection

**Guarantees**

- No separately stored `AttentionState` exists in a tracked Claude session.
- The projection returns the same public attention value as M1 for every
  accepted event scenario.
- `turn_ended` takes precedence over a retained `turn_started` basis;
  otherwise a start basis yields Running; a tool event with no prior basis
  retains its receipt as `turn_started`; with neither, the session is Idle at
  its retained `idle_since` timestamp, or at the current receipt for a newly
  admitted session.
- Every Running transition clears `turn_ended`; matching pending ids clear both
  pending state and `turn_ended`. If no `turn_started` exists at a matching
  clear, the clear receipt becomes the Running start basis; otherwise the
  retained basis is preserved. Unmatched permission/elicitation results
  preserve NeedsYou, but unmatched post-tool events still perform their own
  Running transition while retaining the unrelated pending id.
- Pending correlation clears only on a matching tool-use id and does not by
  itself manufacture a NeedsYou state.

**Expects**

- Events have passed hook/wire validation and arrive in the adapter's receipt
  order.
- Snapshot construction provides the current receipt timestamp.

**Failure behavior**

- Existing project-identity fallback and tombstone behavior remain unchanged.
- No malformed wire value is introduced by this component.

**Does not**

- Change Claude event fields, wire encoding, OpenCode behavior, or the public
  `AttentionState` enum.
- Implement expiry, live hook registration, or persistence.

### T05 — serde-derived wire conversion

**Guarantees**

- The serializer emits the current protocol version, envelope keys, event kind
  values, declaration-order keys, compact formatting, optional omission, and
  trailing newline exactly as before.
- The decoder accepts the same valid values, ignores unknown keys, and retains
  the existing bounds, duplicate/typed edge behavior, category-only errors,
  and serde_json's default recursion behavior: 127 nested containers are
  accepted and 128 are rejected by the locked dependency.
- Raw JSON is transient and never appears in a typed state value or error.
- Hook-side R14 extraction, truncation, required-field validation, label
  bounds, and `tool_input` conversion remain authoritative and are not bypassed
  by the serde model.

**Expects**

- A single newline-delimited frame no larger than the existing envelope bound.
- Event names and fields match the sealed R13/R14 contract.

**Failure behavior**

- Unknown protocol version, unknown event kind, malformed shape, and bounds
  violations map to their existing `DecodeError` categories.
- Serialization over the envelope bound returns `EnvelopeSerializeError`.

**Does not**

- Add wire versions, events, fields, or a migration layer.
- Parse hook payloads or own socket I/O.

## Pressure Test

- **Happy path:** each M1 state test reaches the same snapshot through the new
  projection; each event round-trips through the derived envelope.
- **Malformed input:** typed deserialization fails before state sees a value;
  the mandatory raw preflight is bounded and dropped in the same call.
- **Duplicate keys:** serde_json's last value wins at root, record, event, and
  kind objects; the final kind determines whether preflight returns
  `UnknownEvent` or permits typed serde.
- **Unknown kind:** the preflight returns `UnknownEvent` without attempting to
  construct a partial domain event; missing/non-string kind is `Malformed`.
- **Recursion:** the locked serde_json default accepts 127 nested containers
  and rejects 128 as category-only `Malformed`; no parser limit is changed.
- **Future field:** serde ignores an extra key, preserving the existing
  forward-tolerant decoder behavior.
- **Rollback:** T04 and T05 have disjoint commits and can be reverted
  independently without data migration.

## Assumptions And Open Questions

- The repository's wire compatibility requirement means the current JSON shape
  is the contract even though the transport is local.
- `serde_json`'s default unknown-field behavior is retained intentionally;
  strict rejection of extras would be a separate contract decision.
- A bounded raw kind preflight is mandatory for classification: missing or
  non-string final kind is `Malformed`, unknown/empty final kind is
  `UnknownEvent`, and known final kind proceeds through typed serde and typed
  validation. Duplicate JSON keys use serde_json's existing last-key-wins
  behavior at root, record, event, and kind objects. The preflight must not
  recreate per-event field mapping or leak raw payloads.

## Validation Scenarios

- Running after `UserPromptSubmit` and tool events retains the original turn
  start timestamp.
- Permission/elicitation and Notification question states project correctly,
  including a later matching clear and a later tool event.
- An existing Idle timestamp survives tracked Resume/Compact, unmatched
  permission/elicitation results, and no-op notifications, while a new Idle
  transition records the current receipt.
- A first tool event retains its receipt as `turn_started`, so tracked
  Resume/Compact preserves its Running projection; an existing start basis is
  still retained unchanged.
- A permission/elicitation first event followed by its matching clear uses the
  clear receipt as the Running start basis; a retained basis remains retained.
- An unmatched permission/elicitation result preserves NeedsYou, while an
  unmatched post-tool event becomes Running without clearing an unrelated
  pending id, including when that id belongs to the top-level session and the
  tool targets a subagent.
- Stop, StopFailure, compact/resume, and subagent Stop preserve their M1
  timestamp and question semantics.
- All fifteen event variants serialize and deserialize with their R14 fields.
- Optional values are omitted on serialization and accepted absent/null on
  decode.
- Unknown versions, kinds, malformed shapes, extra fields, oversized labels,
  invalid timestamps, and oversized frames retain their existing outcomes.
