# Implementation Plan: Claude Dashboard M2 Structural Rewrites

## Purpose

Replace the Claude adapter's hand-synchronised attention representation with a
small set of authoritative turn facts and a single attention projection. Replace
the duplicated hand-written wire encode/decode mapping with serde-derived
serialization while preserving the existing JSON envelope, validation bounds,
and category-only failure behavior.

## File Tree

crates/dashboard/
  Cargo.toml                         # add direct serde derive dependency
  src/claude/state.rs                # derive attention from turn facts
  src/claude/hook.rs                 # derive envelope/event serialization
  src/claude/wire.rs                 # typed serde decode and validation
  src/snapshot.rs                    # state-model documentation correction
  src/shell/reclassify.rs            # matching Idle ownership comment
Cargo.lock                            # dependency lock update if needed
tasks/2026-09-05-claude-dashboard-fable-fixes/
  contracts/T04-turn-facts.md
  contracts/T05-serde-wire.md
  M2-design.md

## Data Models

### T04: Claude turn facts

`ClaudeTrackedSession` retains authoritative facts rather than a separately
mutated `AttentionState`:

- `turn_started: Option<Timestamp>` — the current turn's start basis, retained
  for later tool snapshots and cleared when the turn has ended.
- `turn_ended: Option<(Timestamp, bool)>` — the latest needs-you transition and
  whether it carries the question indicator.
- `pending_tool_use_id: Option<String>` — the correlation key for a pending
  permission or elicitation request; it is not itself the attention state.

The projection is ordered: `turn_ended` produces `NeedsYou`, otherwise
`turn_started` produces `Running`, otherwise the session is `Idle` at the
current snapshot receipt. The representation must preserve the M1 behavior
where a permission/elicitation or notification can show `NeedsYou` while a
turn-start basis remains available for a later running event.

The T04 contract's event-to-fact matrix is normative: every M1 path, every
SessionStart source, matching versus unmatched pending ids, timestamp basis,
question semantics, and the clearing of `turn_ended` on Running transitions
must be tested. `turn_ended` is a transient projection fact, not a sticky
terminal state.

`Fork` remains part of the existing `SessionStartSource` enum and is a reset
source, with the same fact clearing as `Startup`, `Clear`, and absent source.
When a matching pending clear has no retained start basis, it establishes
`turn_started` at the clear receipt; with a retained basis it preserves that
basis. Pending correlation is separate from the event's own transition: a
mismatched permission/elicitation result preserves NeedsYou, while a mismatched
post-tool event still performs its own Running transition and leaves the
unrelated pending id intact.

### T05: Serde wire model

The existing `ClaudeIpcEnvelope`, `ClaudeHookRecord`, `ClaudeEvent`,
`SessionStartSource`, `SessionEndReason`, and `ReceivedAt` types become the
serde-backed wire model. The JSON shape remains:

- envelope fields: `protocol_version`, `record`
- record fields: `session_id`, `cwd`, `event`, `received_at`
- event field: internally tagged `kind` with the existing snake-case values
- optional values omitted by the serializer and accepted as absent/null by the
  decoder, exactly as before

## Orchestration

### T04

1. Replace direct writes to `attention` with writes to the authoritative turn
   facts for every existing event transition.
2. Project the public `AttentionState` at snapshot construction using the
   current receipt as the Idle timestamp.
3. Preserve the M1 event-specific asymmetries, subagent routing, pending-id
   clearing, action-line behavior, and session lifecycle truth table.
4. Update only the state-model comments in `snapshot.rs` and
   `shell/reclassify.rs` that still say only OpenCode constructs `Idle`.

### T05

1. Add the direct serde derive dependency and derive serialization for the
   existing typed envelope/event model with explicit wire names and optional
   field behavior.
2. Make `ClaudeIpcEnvelope::to_wire` use the derived serializer, preserving the
   newline and envelope-size check.
3. Make `decode_envelope` deserialize the typed envelope after frame and
   protocol checks, then apply the existing typed bounds validation.
4. Perform a bounded raw `kind` preflight before typed serde: missing or
   non-string kind is `Malformed`, an unknown/empty final kind is
   `UnknownEvent`, and a known final kind proceeds through typed serde and
   typed bounds validation. Duplicate JSON keys retain serde_json's last-key-
   wins behavior at root, record, event, and kind objects. Raw JSON remains
   transient and never crosses the wire/state boundary.
5. Leave hook-side R14 extraction authoritative: serde does not replace
   allowlist selection, truncation, required-field validation, label bounds, or
   `tool_input` object-to-compact-text conversion.
6. Preserve all fifteen event variants, field names, exact compact key order,
   optional omission/null acceptance, ignored extra keys, newline and inclusive
   size bounds, category-only errors, duplicate/typed edge behavior, the
   existing serde_json recursion limit of 128 nested containers, and existing
   round-trip tests.

## Boundaries

### In Scope

- Derive attention from turn facts without changing M1 observable behavior.
- Replace the envelope serializer and per-event wire decoder with serde-backed
  typed conversion.
- Preserve the exact current wire JSON, bounds, error categories, and privacy
  behavior.
- Add or adjust regression tests that prove the structural rewrites, not only
  the existing suite.

### Out Of Scope

- New Claude events, fields, or wire-version negotiation.
- Narrowing the R14 field set or changing privacy/capture policy.
- Live hook registration, transcript capture, or end-to-end proof.
- Core snapshot schema changes or changes to OpenCode behavior.
- Broad legacy spec cleanup deferred at the M1 milestone.

### Must Not Change

- The fifteen-event allowlist and R14/R15 field names and size bounds.
- `SessionSnapshot`'s public attention variants and current action invariants.
- M1's notification, Stop, permission/elicitation, compaction, and subagent
  behaviors.
- Category-only rejection behavior and the rule that raw rejected payloads do
  not enter state or logs.

### Must Follow

- Use `code-quality`, `software-design`, and `writing-unit-tests` in the task
  contracts; use `writing-specs` only if an externally visible wire contract
  changes. This plan assumes no wire/spec delta because the JSON is preserved.
- Keep pure turn projection and typed conversion testable without sockets.
- Keep task ownership disjoint: T04 owns state and state-model comments; T05
  owns serde dependency and hook/wire conversion.

## Pre-Decisions

### Attention projection

Decision: How to remove the duplicated `attention` field without losing the
different M1 timestamp and question semantics.

Options: Keep hand-synchronised fields; replace them with one opaque enum;
store turn facts and project attention.

Choice: Store `turn_started`, `turn_ended` (timestamp plus question flag), and
`pending_tool_use_id`, then project attention with `turn_ended` precedence.

Rationale: These are the facts already needed by transitions. The precedence
preserves permission/notification `NeedsYou` while retaining a turn-start basis
for the next running event, without allowing a second mutable attention value
to drift from them.

### Unknown wire kinds

Decision: How to keep the existing `UnknownEvent` error category with serde.

Options: Collapse unknown kinds into `Malformed`; implement a fully custom
per-event deserializer; preflight the kind and let serde decode known kinds.

Choice: Preflight only the bounded raw `kind` string for classification, then
deserialize the known typed envelope and run typed validation.

Rationale: It removes the large duplicated field-mapping match while retaining
the observable error taxonomy and the current transient-raw-JSON privacy
boundary. The preflight is classification, not a second wire model.

### Reviewer strength

Decision: Reviewer for the high-stakes turn-state rewrite.

Choice: `luna-high`, explicitly selected by the user on 2026-09-05. T05 uses a
fresh `luna` reviewer because its contract is a bounded wire-preserving
mechanical rewrite.

## Testing Strategy

- **T04 feature test:** prove that every M1 attention outcome is produced by
  the fact projection, including Running, NeedsYou with both question values,
  and Idle; assert the facts and public projection stay aligned across every
  SessionStart source (including Fork reset), compact/resume, first-event
  matching clear with receipt fallback, retained-start matching clear, Stop,
  and top-level-pending/unrelated-subagent-tool flows.
- **T05 feature test:** serialize representative events through the derived
  model and decode them back, asserting byte-level JSON shape for optional
  omission and equality of all fifteen event variants and fields. Assert
  duplicate-key last-wins behavior at every object level, mandatory final-kind
  preflight categories, and serde_json's unchanged recursion boundary of 128
  nested containers versus boundary+1.
- Preserve and run all existing state, hook, wire, adapter, ingress, and
  runtime tests. Add negative cases for unknown kind, unknown version, bounds,
  ignored extras, and malformed known events where the implementation changes
  the path.
- Run `cargo test -p dashboard` and
  `cargo clippy -p dashboard --all-targets` at each task gate and after M2
  integration.

## Verification Checkpoints

| After | Verify | Fail action |
|---|---|---|
| T04 fact model | State tests cover all M1 attention paths and no direct attention writes remain | Rework the projection before accepting the task |
| T05 serde model | All fifteen round trips, optional omission, error categories, and bounds pass | Preserve the wire shape before proceeding |
| Both tasks | Full dashboard test/clippy gates and fresh-eyes M2 seam review pass | Return the affected task to its contract or escalate |

## Migration And Rollback

No persisted data or public JSON migration exists. The wire version remains 1
and the serialized envelope is unchanged. Each task has its own commit, so a
regression can be reverted independently; reverting T04 restores the prior
state representation and reverting T05 restores the hand-written conversion.

## Acceptance Criteria

- [ ] `ClaudeTrackedSession` no longer stores a separately mutable
  `AttentionState`.
- [ ] M1 attention and content behavior remains covered and unchanged through
  the derived projection.
- [ ] The serde-backed serializer emits the exact current envelope shape and
  newline/size behavior.
- [ ] The serde-backed decoder accepts all current valid envelopes, ignores
  extra keys, and preserves unknown-version, unknown-event, malformed, and
  out-of-bounds categories.
- [ ] `cargo test -p dashboard` is green and
  `cargo clippy -p dashboard --all-targets` is clean.
- [ ] T04 is reviewed by a fresh `luna-high`; T05 is reviewed by a fresh
  `luna`; M2 receives a fresh-eyes integration review before advisor sign-off.

## Tasks

### T04: Derive attention from turn facts

- **Files:** `crates/dashboard/src/claude/state.rs`,
  `crates/dashboard/src/snapshot.rs` (comments only),
  `crates/dashboard/src/shell/reclassify.rs` (comment only)
- **Depends on:** none
- **Agent:** `luna` implementer; `luna-high` reviewer
- **Verify:** state feature tests plus full dashboard test/clippy commands

### T05: Derive the Claude wire schema with serde

- **Files:** `crates/dashboard/Cargo.toml`, `Cargo.lock`,
  `crates/dashboard/src/claude/hook.rs`,
  `crates/dashboard/src/claude/wire.rs`
- **Depends on:** none; fan-out with T04
- **Agent:** `luna` implementer; fresh `luna` reviewer
- **Verify:** all wire round trips and rejection tests plus full dashboard
  test/clippy commands
