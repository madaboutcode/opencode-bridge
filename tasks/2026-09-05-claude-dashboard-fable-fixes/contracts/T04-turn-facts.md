# T04 — Turn facts and single attention projection

**Contract version** — 4 (advisor-adjudicated amendment; Review Frame v3
pending fresh implementation review)

**Reviewer binding** — `luna-high` (fresh review)

**Context** — Replace the independently mutated Claude `attention` field with
authoritative turn facts while preserving every M1 snapshot, routing, content,
and action behavior. The public `AttentionState` remains the provider-neutral
boundary; this is an internal state-model rewrite, not a lifecycle redesign.

**Boundaries** — owns only `crates/dashboard/src/claude/state.rs` logic and
tests, plus the state-model comments in `crates/dashboard/src/snapshot.rs` and
`crates/dashboard/src/shell/reclassify.rs`. It may remove direct attention
writes and add the pure projection needed by `state.rs`.

Must not touch `hook.rs`, `wire.rs`, `Cargo.toml`, `Cargo.lock`, public
`AttentionState` variants or fields, snapshot schema, OpenCode behavior, or
any action/content/routing rule. Do not add expiry, persistence, new events,
or a new public lifecycle type.

**Delivery profile** — `tasks/2026-09-05-claude-dashboard-fable-fixes/delivery-profile.md`
version 2; task override: none.

**Skills to apply** — `software-design`, `code-quality`, and
`writing-unit-tests`.

## Authoritative Facts

The tracked session stores only these attention-driving facts:

- `turn_started: Option<Timestamp>`: the current turn's start basis. User
  prompt and subagent start set it; stop and `idle_prompt` clear it. Tool
  events reuse it, setting it to their receipt when absent. A matching pending
  clear also uses its receipt as the basis when none is retained.
- `turn_ended: Option<(Timestamp, bool)>`: the latest needs-you transition and
  its question bit. It is a projection input, not a second attention value.
- `pending_tool_use_id: Option<String>`: the outstanding permission or
  elicitation correlation key. It does not itself imply `NeedsYou`.
- `idle_since: Option<Timestamp>`: the timestamp of the current Idle
  projection. New sessions and reset paths set it to their receipt; paths that
  enter Running or NeedsYou clear it; no-op and preserved-projection paths
  retain it.

The projection at snapshot receipt `r` is exact and ordered:

1. `Some((t, q))` -> `NeedsYou { question: q, turn_ended: t }`.
2. Otherwise `Some(t)` -> `Running { turn_started: t }`.
3. Otherwise -> `Idle { last_update: idle_since.unwrap_or(r) }`.

An event that is specified below as producing Running clears `turn_ended`.
Clearing a matching pending id also clears `turn_ended` and establishes
`turn_started` at that event's receipt only when no start basis exists; it
preserves an existing basis otherwise. An unmatched id does not clear the
pending id by correlation. Pending correlation is independent of the event's
own transition, so an unmatched post-tool event may still clear `turn_ended`
through its own Running mapping.
The projection must not mutate facts.

## Event-to-Fact Matrix

Each row describes the existing M1 path, including facts, projection, and
non-attention behavior that must remain unchanged. `r` is the event receipt.

| M1 event path | `turn_started` | `turn_ended` | `idle_since` | pending id | projected result | preserved behavior |
|---|---|---|---|---|---|---|
| `SessionStart` untracked, any source | `None` | `None` | set `r` | `None` | `Idle(last_update=r)` | admission, `created_at`, routing unchanged |
| tracked `SessionStart` with `Startup`, `Clear`, `Fork`, or absent source | clear | clear | set `r` | clear | `Idle(r)` | reset semantics unchanged |
| tracked `SessionStart` with `Resume` or `Compact` | preserve | preserve | preserve | preserve | prior projection | attention/facts remain unchanged byte-for-byte |
| `UserPromptSubmit` | set to `r` | clear | clear | clear | `Running(r)` | records prompt; clears stale final answer |
| `PreToolUse` | preserve, else set to `r` | clear, including after a matching pending id | clear | matching id clears; unmatched/absent preserves | `Running(start basis)` | routes by `agent_id`; only this path updates current/recent action |
| `PostToolUse` | preserve, else set to `r` | clear, including after a matching pending id | clear | matching clears; unmatched/absent preserves | `Running(start basis)` | routes by `agent_id`; never changes current/recent action |
| `PostToolUseFailure` | preserve, else set to `r` | clear, including after a matching pending id | clear | matching clears; unmatched/absent preserves | `Running(start basis)` | same routing/action preservation as `PostToolUse` |
| `PermissionRequest` | preserve | set `(r, true)` | clear | set event id | `NeedsYou(true, r)` | top-level target; synthesized permission text unchanged |
| `PermissionDenied` matching pending id | preserve, or set to `r` if absent | clear | clear | clear | `Running(start basis or r)` | clears final answer on match; no unrelated content changes |
| `PermissionDenied` unmatched/absent pending | preserve | preserve | preserve | preserve | prior projection | event still emits its normal snapshot; no own Running transition |
| `Elicitation` | preserve | set `(r, true)` | clear | set event id | `NeedsYou(true, r)` | top-level target; request text unchanged |
| `ElicitationResult` matching pending id | preserve, or set to `r` if absent | clear | clear | clear | `Running(start basis or r)` | clears final answer on match |
| `ElicitationResult` unmatched/absent pending | preserve | preserve | preserve | preserve | prior projection | event still emits its normal snapshot; no own Running transition |
| `Notification("idle_prompt")` | clear | set `(r, false)` | clear | preserve | `NeedsYou(false, r)` | no correlated clear; later unconditional event may replace it |
| `Notification("permission_prompt"/"agent_needs_input")` | preserve | set `(r, true)` | clear | preserve | `NeedsYou(true, r)` | turn may still be running; no pending id invented |
| other `Notification` or `None` | preserve | preserve | preserve | preserve | prior projection | only snapshot `last_updated` advances |
| top-level `Stop` | clear | set `(r, looks_like_question(text))` | clear | preserve | `NeedsYou(question, r)` | final text and question heuristic unchanged |
| subagent `Stop` | clear | clear | set `r` | preserve | `Idle(r)` | target/routing and final text unchanged |
| `StopFailure` | preserve | set `(r, false)` | clear | preserve | `NeedsYou(false, r)` | no fabricated text or start clearing |
| `SubagentStart` | set to `r` | clear | clear | clear | `Running(r)` | subagent identity, parent, prompt unchanged |
| `SubagentStop` | remove target | remove target | remove target | remove target | `Gone`, no snapshot | tombstone behavior unchanged |
| `SessionEnd` | remove target | remove target | remove target | remove target | `Gone`, no snapshot | top-level removal unchanged |

For tool events routed to a subagent, the existing M1 rule that a pending
top-level permission may need clearing as well remains mandatory. The matrix's
matching/unmatched rule applies to the record that owns the pending id; do not
invent subagent pending ids when the event schema has no `agent_id`. A
mismatched `PostToolUse` or `PostToolUseFailure` still performs its own M1
Running transition on the targeted record and clears that record's
`turn_ended`, while leaving the unrelated pending id on the top-level record.
Cover the case where the top-level record is pending and an unrelated
`agent_id` tool event targets a subagent: the subagent becomes Running, the
top-level pending id and NeedsYou fact remain unchanged, and routing/content
behavior is preserved.

## Acceptance — executable

1. `ClaudeTrackedSession` contains no separately mutable `AttentionState`.
2. A state test exercises each matrix row, including every SessionStart source
   (`Fork` is a reset source),
   both Notification question classes and the no-op class, top-level and
   subagent Stop, StopFailure, both tombstones, and all matching/unmatched
   pending-id paths for PermissionDenied, ElicitationResult, PostToolUse, and
   PostToolUseFailure.
3. A feature test starts with `PermissionRequest` or `Elicitation` as the
   first event, then sends its matching `PermissionDenied` or
   `ElicitationResult`, and asserts `Running { turn_started: receipt }`.
   A companion test with a retained start asserts that the original basis is
   preserved.
4. Tests assert exact `Timestamp` and question semantics: `NeedsYou` uses the
   transition receipt, Running uses the retained start basis or the tool
   receipt fallback, new Idle transitions use their receipt, and preserved
   Idle projections retain `idle_since` across no-op/Resume/Compact paths.
5. Tests prove `turn_ended` precedence does not permanently pin NeedsYou:
   every specified Running transition clears it; mismatched
   `PermissionDenied`/`ElicitationResult` preserve NeedsYou, while mismatched
   `PostToolUse`/`PostToolUseFailure` clear it without clearing an unrelated
   pending id, including the top-level-pending/subagent-tool case.
6. Existing assertions for `current_action`, `recent_actions`, final text,
   prompts, parent/target routing, created/updated timestamps, and Gone
   events remain green and continue to pass without changed expected values.
7. `snapshot.rs` and `shell/reclassify.rs` comments describe that Claude may
   construct Idle, without changing executable behavior or the public enum.
8. `cargo test -p dashboard` and `cargo clippy -p dashboard --all-targets`
   pass.

## Review Frame

**Status** — advisor-adjudicated amendment · v3 · fresh implementation review
pending

**Context** — Replace Claude's duplicated mutable attention value with authoritative turn facts and one projection while preserving M1 behavior, including existing Idle timestamps.

**Expectations** — Verify the complete event matrix, including retained tool
fallback bases across compact/resume, idle_since preservation on no-op paths,
Fork reset, exact timestamp/question semantics, turn-ended precedence,
matching/no-start fallback, unmatched correlation, unrelated subagent tools,
Stop/StopFailure, tombstones, and unchanged action/content/routing.

**Depth** — Fresh `luna-high`; exercise every matrix row and adversarial stale-fact sequence with synthetic tests, including preserved Idle timestamps, then run the full dashboard test/clippy gates. Reject any change to the public enum, wire, expiry, tombstones, or M1 content/routing behavior.
