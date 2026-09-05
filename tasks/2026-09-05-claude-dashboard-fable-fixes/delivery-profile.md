<!-- Instantiated at scoping. Drafted by the advisor from facts the conductor supplied.
     The user approves material release posture. Referenced by path, never pasted
     into spawn prompts. -->

# Delivery Profile — 2026-09-05-claude-dashboard-fable-fixes

**Status** — user-approved · version: 2 · approved: 2026-09-05 by the user (both posture statements below confirmed as written)

**Source facts** — `tasks/2026-09-05-claude-dashboard-fable-fixes/advisor-brief.md` (run scope, roster, ground truth, prior verdicts); `tasks/2026-09-05-claude-dashboard-fable-review-findings.md` (the Fable 5.1 review, verbatim); `docs/specs/dashboard/claude.md` R11–R17 (event allowlist, field table, attention mapping); `crates/dashboard/src/snapshot.rs` (`current_action` / `recent_actions` invariants); two recorded user decisions — (a) keep the wide R14 field capture even with no consumer ("Larger raw preview, no consumer yet"), reaffirmed this run; (b) reviewer role is a `coder` agent, not Opus. No workflow, scale, or user population is invented beyond these.

**Release context** — stage: pre-live-proof refinement pass (not initial build, not hardening, not security review) · users and scale: one developer — the repo owner — watching their own Claude Code sessions on one machine; a local Unix socket, one dashboard process, no concurrency model beyond the sessions one person runs · purpose: make the tile state truthful on the ordinary daily paths *before* a live end-to-end run records evidence, so live-proof validates real behaviour rather than blessing known-wrong tiles. The reviewer's own words: fix findings 1 and 2 first, "otherwise live-proof will validate tile output that's misleading."

**Supported workflows** — the five everyday paths the findings describe, plus what the run's structural work must not break:

1. **Normal turn** — prompt → tool calls → `Stop`. Tile shows a readable action line while running, one entry per tool call in recent actions, and settles correctly at the end (including a question glyph when the turn ends by asking something).
2. **Permission approval** — a permission-gated tool call at top level *and* inside a subagent. Tile says what is being asked for, and clears when the call proceeds or is denied.
3. **Subagent spawn → work → finish** — the subagent appears under its parent, and a finished subagent stops claiming attention without waiting for its tombstone.
4. **Interrupted or escaped turn** — Escape mid-turn, or an interactive deny. No `Stop` arrives; the tile must still stop claiming to be running.
5. **Session auto-compaction mid-conversation** — `SessionStart{source: compact}` on an already-tracked session; the tile stays where it is instead of dropping to `Idle` and vanishing from the grid.

**Release bar** — a change is required in this run when it is needed for any of:

- **Task acceptance** — each finding (1, 2, 4, 5) and each structural smell has a named acceptance behaviour and a regression test that reproduces the specific scenario, not just a passing suite. Per the ground truth: `cargo test -p dashboard` green (333 baseline plus new tests) and `cargo clippy -p dashboard --all-targets` clean.
- **Correct state under supported operations** — after any of the five workflows above, the tile's attention state matches what is actually true of the session. A tile stuck `Running` after Escape, or stuck `NeedsYou` after a subagent finished, is a defect against this bar, not a rough edge.
- **Contract integrity** — `snapshot.rs`'s stated invariants hold: `current_action` is never a raw tool name; `recent_actions` never contains the current action. Finding 2 currently violates both, and a test asserts the violating behaviour as intended — that test is wrong and must change with the code.
- **Documented truth** — any task that changes attention mapping, captured fields, or wire shape updates `docs/specs/dashboard/claude.md` under the `writing-specs` skill and produces a `spec-delta.md`-style record, matching the discipline of the two prior reviewed rounds. Code and spec disagreeing is a defect in this run, not a follow-up.
- **Foundations expensive to retrofit** — the turn-state derivation (structural smell 1) and the serde-derived wire schema (smell 2) are in scope precisely because retrofitting them after live-proof means re-validating live evidence. They are here to make later work cheap, not because anything is broken today.

Privacy note, not a bar item: R14 holds sensitive session content (prompts, assistant text, tool input) in memory for the session's lifetime. This is a single-user local tool and the user has twice chosen wide capture knowingly. It is therefore an accepted, recorded posture — not a finding — under the assumption that the socket stays local, the data is memory-only, and nothing is persisted or transmitted. If any task in this run would write captured content to disk or across a network, that assumption breaks and the change needs the user, not this profile.

**Deferral posture** — the following are known and deliberately not addressed here:

- **Live end-to-end proof** — registering hooks in a real `.claude/settings.json` and running against real transcripts. Explicitly a separate next phase. No task in this run may grow into live wiring. Assumption: the behaviours fixed here are provable by unit and integration tests against synthetic events; anything only provable live belongs to that phase. **Promotion trigger:** a task that cannot state its acceptance without a real session.
- **Finding 3 — narrowing R14's field set** — rejected for this run by explicit user decision, reversing an earlier instruction to proceed. The captured field set stays exactly as-is. This is not a deferral awaiting a trigger; it is a settled product decision. **Watch item for every task in this run:** a clean implementation that "naturally" drops an unused field is finding 3 by another name and must be flagged, not done. See the open question below — finding 2's own recommended direction trips this wire.
- **Exotic edge cases** — the review was scoped to everyday flows by the user's own instruction ("not pedantic edge cases"). Rare event orderings outside the five workflows are out of scope. Assumption: the everyday paths dominate; the reviewer already judged the listener/command hardening sufficient and said to leave it. **Promotion trigger:** live-proof surfaces an ordering the state machine mishandles.
- **Detail pane for unread fields** — `tool_response`, `error`, `denial_reason` and friends have no renderer. Building one is not this run. **Promotion trigger:** the user asks to see tool output or errors in a tile.
- **Scale, concurrency, multi-user, auth** — no such requirement exists for this tool. Not deferred so much as absent; do not let a reviewer manufacture one.

## Finding Disposition

- **Correct now** — evidenced defects against the release bar, plus unapproved implementation scope beyond the supported release.
- **Preserve foundation** — keep the minimum seam needed to avoid disproportionate retrofit cost; record the constraint in `decisions.md`, but do not build the future behavior.
- **Defer with trigger** — append a credible, bounded concern to `deferred.md` with scenario, consequence, assumption, and promotion trigger.
- **Reject** — do not turn scenario-free hypotheticals or alternative design preferences into known issues.

Frequency alone never decides disposition. A reviewer may challenge this profile with evidence that a real workflow or non-deferrable consequence was misdrawn. Preserve contested classifications in the gate report; do not silently downgrade them.

**Amendment** — the advisor may propose changes, but material changes to supported workflows, risk posture, or non-deferrables require user approval and a `decisions.md` entry naming affected tasks and deferrals.

## Resolved at scoping

**Finding 2 vs. finding 3 (settled, v2).** Finding 2's written direction would have dropped `tool_input` from the wire, which is finding 3 by another name and contradicts the user's decision to keep wide capture. Resolved: render the action line in `state.rs` from the `tool_input` already on the wire. No field added, none dropped, no wire or spec-field change. The user delegated the technical call with the guidance "show useful info on the screen, basic metadata is fine, keep it consistent with opencode, think from user pov"; the conductor reports OpenCode renders its own action line in `session_state.rs`, its adapter-state module — the same architectural position as Claude's `state.rs` — so this is the consistent choice, not merely the cheapest. Recorded in `decisions.md`.

Consequence to hold in review: `state.rs` now parses `tool_input` JSON (bounded at 4 KiB) on the event path. That is accepted here as a small, contained cost. If a reviewer finds it materially expensive on a hot path, that is a finding against this decision, not a silent redesign.

## Posture confirmed by the user

These two statements were the advisor's reading of an unstated posture rather than something the user had said; everything else in this profile restates decisions the user already made. Both were put to the user and confirmed as written on 2026-09-05. A reviewer calibrates against them:

1. **Privacy posture** — captured session content (prompts, assistant text, tool input) sits in memory for the session's lifetime, and that is an accepted risk rather than something this run addresses, on the assumption it stays local, memory-only, and is never written to disk or sent anywhere.
2. **No scale requirement** — one developer, one machine, one dashboard process; a reviewer should not raise concurrency, multi-user, or auth concerns as findings, because no such requirement exists.
