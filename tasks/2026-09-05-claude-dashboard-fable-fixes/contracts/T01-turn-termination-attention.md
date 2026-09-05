# T01 — Turn-termination attention correctness

**Contract version** — 2 (v2: added item 3a below, resolving T00's deferred
promotion trigger on `client.md` R1.3 — see `deferred.md`)

**Context** — goal: make the tile's attention state correctly reflect how
a turn actually ended, for the exit paths that currently leave it wrong
(an interrupted/escaped/denied turn with no `Stop`; a finished subagent
still shown as needing attention; a turn ending in a question shown with
the plain glyph) · who uses it: the person watching the dashboard tile ·
scale: one developer, one dashboard process · criticality: this is the
review's top-priority defect — "otherwise live-proof will validate tile
output that's misleading." **Scope correction (advisor review):** finding
1 *shortens* the interrupted-turn window, it does not eliminate it —
`idle_prompt` fires after Claude has been idle roughly 60 seconds, so a
tile stuck `Running` after an Escape self-corrects in about a minute
rather than immediately, and depending on the reclassify window `W` this
may not even be the first thing that corrects the tile. Do not describe
this task as "fixing interrupted turns" without that qualifier — it
narrows the window, it does not close it to zero.

**Delivery profile** — `tasks/2026-09-05-claude-dashboard-fable-fixes/delivery-profile.md` version 2 · task override: none

**Boundaries** — owns: `crates/dashboard/src/claude/state.rs`'s
`ClaudeEvent::Notification` match arm and `ClaudeEvent::Stop` match arm
only (currently lines ~287-293 and ~294-310 — confirm against the file as
found, not these line numbers, since T00 may shift them slightly); the
module's top doc comment (CONTRACT/GUARANTEES block) to describe the
corrected mapping · must not touch: any other match arm in `process()`,
`ClaudeEvent::StopFailure` (unaffected — it carries no assistant text to
classify and no `agent_id`), `crates/dashboard/src/claude/hook.rs` or
`wire.rs` (no field/wire change), `docs/specs/dashboard/claude.md` (its own
Scope section places attention mapping outside it — see the module doc
comment instead).

**Conventions** — `cargo test -p dashboard` and `cargo clippy -p dashboard --all-targets`; baseline after T00 lands is 333 passed, clippy clean, plus whatever T00 net-changed. Import `looks_like_question` from T00's new shared module (T00 must land first — this contract does not restate its own copy).

**Skills to read and apply** — `code-quality`, `writing-unit-tests` (each scenario below needs a regression test reproducing it, not just a passing suite), `writing-specs` (for item 3a's client.md wording fix only).

**Acceptance — done when**:

1. **`Notification` mapping.** The event's `notification_type` field
   (`Option<String>`) drives attention instead of being ignored:
   - `Some("idle_prompt")` → `attention = NeedsYou { question: false, turn_ended: receipt }`, and `turn_started` clears to `None` (mirrors `Stop`'s own "turn is over" bookkeeping, so a later tool event doesn't resurrect a stale start time).
   - `Some("permission_prompt")` or `Some("agent_needs_input")` → `attention = NeedsYou { question: true, turn_ended: receipt }`. Do not touch `turn_started` here — the turn may still be running with a subprocess waiting, unlike `idle_prompt`.
   - Any other value (`None`, or a sub-type not in this list) → unchanged behavior: no attention change, only the produced snapshot's own `last_updated` advances via `receipt`, same as today.
   - Document in the module doc comment that a `Notification`-driven `NeedsYou` has no dedicated clearing event (no `tool_use_id` to correlate against) and relies on the next event that unconditionally sets attention (any `UserPromptSubmit` or tool event) — this is intentional, not a gap to fix here.

2. **Subagent `Stop` goes to `Idle`, not `NeedsYou`.** When `Stop`'s `agent_id` is `Some(_)` (the event targets a subagent), set `attention = Idle { last_update: receipt }` instead of `NeedsYou`. A finished subagent does not need anyone — it is superseded by its own `SubagentStop` tombstone shortly after, and until then it should read as done, not as blocking. The top-level case (`agent_id: None`) is unaffected by this change beyond item 3 below.

3. **Question-glyph reuse for top-level `Stop`.** For the top-level case only (`agent_id: None`), replace the hardcoded `question: false` with `looks_like_question(last_assistant_message)` (from T00's shared module). The subagent case (item 2) does not need this — it now goes to `Idle`, which carries no question flag.

3a. **Spec reconciliation (T00's deferred item).** `docs/specs/dashboard/client.md` R1.3 lists the needs-you/question-detection heuristic as "opencode-specific guesswork with no real wire signal," reasoning that "a harness with an actual 'waiting on you' signal reports it directly, not re-derived from text." Once this item lands, that sentence is false: Claude has no such signal either (its `Stop` event carries only text), so it re-derives the same way OpenCode does, through the same shared function. Fix the wording (under `writing-specs`) to describe the heuristic as shared, text-based, and used by any harness lacking a real wire signal — the forward-looking clause about a harness with a real signal reporting it directly stays true and doesn't need to change, only the "opencode-specific" characterization does. Record the correction in `decisions.md` or `deferred.md` (implementer's call), matching T02's item 12 pattern for the same class of fix.

**Boundaries addition (v2):** this task also owns `docs/specs/dashboard/client.md` R1.3's wording (the one clause named in item 3a only — not the surrounding requirement).

4. Regression tests, one per scenario: (a) a `Notification` with `notification_type: Some("idle_prompt")` transitions a `Running` session to `NeedsYou{question:false}` and clears `turn_started`; (b) `notification_type: Some("permission_prompt")` and separately `Some("agent_needs_input")` both transition to `NeedsYou{question:true}` without touching `turn_started`; (c) a `Notification` with `notification_type: None` or an unrecognized value leaves attention unchanged; (d) a `Stop` with `agent_id: Some(_)` transitions the subagent's tracked session to `Idle`; (e) a top-level `Stop` (`agent_id: None`) whose `last_assistant_message` ends in `?` (or matches a phrase in `looks_like_question`'s list) produces `NeedsYou{question:true}`; (f) a top-level `Stop` with plain text still produces `NeedsYou{question:false}`, proving the existing common case doesn't regress.

5. `cargo test -p dashboard` green (baseline + these new tests), `cargo clippy -p dashboard --all-targets` clean.

**Gate** — report-only (refine-loop)

**Dependencies** — T00

## Review Frame

*(v2 — refreshed for item 3a.)* Three code changes on two arms plus one spec clause; review how the code composes, not each item alone. Does a subagent `Stop` still reach `Idle` when its text ends in a question mark — item 3 must not leak into item 2's path. Item 1's asymmetry is deliberate: `turn_started` clears for `idle_prompt` only, and is easy to flatten by accident. An unrecognized `notification_type` changes nothing but `last_updated`. On 3a, confirm the edit narrows to the "opencode-specific" characterization and leaves the forward-looking clause standing. Don't accept "interrupts fixed" as the claim.
