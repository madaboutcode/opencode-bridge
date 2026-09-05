# PLAN — claude-dashboard-fable-fixes

## Boundaries

This run implements the Fable 5.1 design review's findings against
`crates/dashboard/src/claude/` (see
`tasks/2026-09-05-claude-dashboard-fable-review-findings.md` for the
review verbatim):

- Finding 1 — map `Notification` sub-types (`idle_prompt`,
  `permission_prompt`, `agent_needs_input`) to attention state instead of
  a no-op.
- Finding 2 — render a real action line (not `"tool: Bash"`) and stop the
  recent-actions ring double-counting PreToolUse+PostToolUse.
- Finding 4 — `PermissionRequest`/`Elicitation` set the Question tile's
  content; `UserPromptSubmit` clears stale `final_assistant_text`.
- Finding 5 — `SessionStart{source: compact}` on an already-tracked session
  does not reset attention to `Idle`.
- Structural smell — reuse `question.rs`'s question-glyph detection for
  `Stop` instead of a hardcoded `question: false`.
- Structural smell — a finished subagent's `Stop` goes to `Idle`, not
  `NeedsYou`.
- Structural smell — derive `AttentionState` from stored facts
  (`turn_started`/`turn_ended`/`pending`) instead of hand-syncing three
  overlapping fields.
- Structural smell — replace `hook.rs`'s `envelope_to_value` and
  `wire.rs`'s `decode_event` (hand-written, ~400 lines together) with a
  serde derive on the wire format.

Finding 2's action line renders in `state.rs` from the `tool_input` value
already on the wire — no new field, no field dropped, no wire/spec change
(user decision, see `decisions.md`).

**Boundary extension (advisor-approved, does not need the user — small,
reversible, inside this run's own structural-smells remit):** `looks_like_
question` and the neutral parts of `render_action_line`
(`collapse_newlines`, `basename`) currently live in `opencode`'s private
modules, invisible outside it. Rather than bump their visibility (backwards
coupling — `claude` depending on `opencode`'s internals for provider-
neutral code), they relocate to a new shared top-level module (matching
this crate's existing `naming` module precedent: a shared concern sitting
beside `opencode`/`claude`, not inside either). `opencode/action_line.rs`'s
tool-name dispatch (`"shell"`/`"edit"` match) is NOT neutral and stays put,
now calling the shared helpers instead of defining them locally. See
`decisions.md` and T00's contract.

## Out of scope

- Finding 3 (narrowing R14's captured field set) — explicitly rejected by
  the user for this run; the field set does not change.
- The live end-to-end proof (real `.claude/settings.json`, real
  transcripts) — a separate, already-planned next phase.

## Ground truth

- `docs/specs/dashboard/claude.md` R13-R14 (event allowlist, field table,
  attention mapping) — update under `writing-specs` when this run changes
  attention mapping or wire encoding; field set itself does not change.
- Build/test: `cargo test -p dashboard`, `cargo clippy -p dashboard
  --all-targets`. Baseline: 333 tests passing, clippy clean.
- Code: `crates/dashboard/src/claude/{hook,wire,state,mod,listener,command}.rs`,
  `crates/dashboard/src/snapshot.rs` (contract invariants), reuse patterns
  in `crates/dashboard/src/opencode/{action_line,question}.rs` and
  `session_state.rs` (OpenCode renders its action line in `session_state.rs`,
  its adapter-state module — the same architectural spot as Claude's
  `state.rs`, which is why finding 2 renders there too).

## Delivery profile

`tasks/2026-09-05-claude-dashboard-fable-fixes/delivery-profile.md`,
version 2, user-approved (scoping gate: `gates/scoping.md`).

## Milestones

**M1 — Behaviour fixes.** Findings 1, 2, 4, 5, plus the two cheap/local
structural smells (question-glyph reuse, subagent `Stop` → `Idle`). T01/
T02/T03 touch `state.rs`'s existing representation without changing it;
T00 is a prerequisite relocation touching a new shared module and
`opencode/action_line.rs`/`question.rs`/`reconcile.rs` (see the boundary
extension above) so T01/T02 have something to import.

**M2 — Structural rewrites.** Turn-state derivation (replace hand-synced
`attention`/`turn_started`/`pending_tool_use_id` with derived state) and
the serde-derived wire schema. Sequenced after M1 per advisor: both
rewrite code M1 also touches, and should be proven against M1's corrected
behaviour (and its new regression tests) rather than racing it.

Rationale for the split and order: advisor, scoping — see `decisions.md`.

## Roles (run config)

- **advisor** — `advisor` agent (Opus), persistent for the run.
- **runner** — `coder` agent, one per reviewed task.
- **implementer** — `coder` agent.
- **reviewer** — `coder` agent, a fresh instance per task, never the same
  agent as that task's implementer. User-approved; advisor flagged that
  M2's turn-state derivation task specifically warrants reconsidering this
  (highest-stakes single task in the run). The user selected `luna-high` for
  the M2 turn-state reviewer; M2's serde-wire reviewer remains `luna`.

## Git policy

Run branch: `claude-dashboard-fable-fixes`, cut from
`conductor/claude-dashboard-support` after committing the pre-existing
round 1/2 schema-widening + subagent-bug-fix work as its own checkpoint
commit (`69addca`). Runner commits its task at loop-pass (owns list + gate
report + deferred.md, per `templates/runner-brief.md`'s fixed lines).
Conductor commits bare tasks at their gate and milestone artifacts at
sign-off.

## Decomposition

M1 tasks, in dependency order (pipeline — all touch `crates/dashboard/src/claude/state.rs`, T00 also touches a new shared module plus `opencode/`'s two call sites, so serialized per the "no file overlap" fan-out rule even though logically independent):

| Task | Owns | Depends on |
|---|---|---|
| T00 | New shared module (name TBD by implementer, e.g. `crates/dashboard/src/text.rs`); `opencode/question.rs`, `opencode/action_line.rs`, `opencode/reconcile.rs` (import updates only) | none |
| T01 | `crates/dashboard/src/claude/state.rs` (`Notification`, `Stop` match arms) | T00 |
| T02 | `crates/dashboard/src/claude/state.rs` (`PreToolUse`/`PostToolUse`/`PostToolUseFailure`, `PermissionRequest`/`Elicitation`, `UserPromptSubmit` match arms); new Claude-side tool-name dispatch function | T00 |
| T03 | `crates/dashboard/src/claude/state.rs` (`SessionStart` match arm) | none |

T01 and T02 both depend on T00 but not on each other; T03 depends on
nothing. Run order: T00, then T01, then T02, then T03 — strictly
sequential because all four touch `state.rs` (T00 doesn't touch `state.rs`
itself but must land before T01/T02 need its exports) and per-task commits
need a clean base each time.

## M2 Decomposition

M2 design and execution plan: `tasks/2026-09-05-claude-dashboard-fable-m2.plan.md`
and `M2-design.md`. The tasks are independent at the file boundary and fan
out, then meet at the M2 milestone review:

| Task | Owns | Depends on | Reviewer |
|---|---|---|---|
| T04 | `claude/state.rs`; state-model comments in `snapshot.rs` and `shell/reclassify.rs` | none | `luna-high` |
| T05 | `dashboard/Cargo.toml`, `Cargo.lock`, `claude/hook.rs`, `claude/wire.rs` | none | fresh `luna` |

T04 removes the separately stored `AttentionState` and derives it from turn
facts without changing M1 behavior. T05 derives the existing envelope/event
wire schema with serde without changing JSON shape or validation behavior. Run
T04 and T05 as a fan-out where the branch/index permits; all task reports and
commits must be recorded before the M2 fan-in review.
