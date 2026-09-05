<!-- Append-only. Each entry: scenario, consequence, deferral assumption,
     promotion trigger, source task. -->

## Nobody clears `current_action` at turn end

**Scenario** — A session's turn ends normally (`Stop` fires) after a tool
call set `current_action` to something like `"running: cargo test"`. T02
fixes what the action line says while a tool is running; it does not clear
`current_action` when the turn ends.

**Consequence** — A settled/needs-you tile can still display the last
tool's action line even though nothing is running anymore, until the next
`PreToolUse` overwrites it on a future turn.

**Deferral assumption** — Pre-existing behavior, not introduced by this
run (T02 only changes *what* the line says, not *when* it's cleared).
Bounded: `layout.md` R5.3's `Question`/`Needs-you (plain)`/`Idle` block
lists don't render `current_action` at all — only the `Running` block
does — so this is not the same class of leak as finding 4's
`final_assistant_text` (T02, item 9's fix). Worth fixing but not blocking
this run's release bar.

**Promotion trigger** — Live-proof surfaces this as visibly confusing in
practice, or a future task already touches the `Stop`/`StopFailure` arms
for another reason (cheap to fold in then).

**Source** — advisor review, decomposition stage, flagged during T01/T02
contract review (2026-09-05).

## `client.md` R1.3 will become false once Claude calls `looks_like_question`

**Scenario** — T00 relocated `looks_like_question` from
`opencode/question.rs` into the new shared `crates/dashboard/src/text.rs`.
The deleted module doc said the heuristic is "opencode-specific guesswork
with no real wire signal," matching `docs/specs/dashboard/client.md` R1.3,
which lists question-detection as harness-specific logic that must stay
inside an adapter. T01 (landing after T00) is expected to call
`looks_like_question` from Claude's side.

**Consequence** — Once T01 lands, R1.3's claim is contradicted by the
code: the heuristic is now shared, not opencode-specific. The in-code
comment that used to carry this claim (and would have prompted someone to
notice) no longer exists after the relocation.

**Deferral assumption** — T00 itself changes no observable behavior, so
it is not the task that makes the spec sentence false; T01 is, the moment
it adds a Claude-side consumer.

**Promotion trigger** — T01's own contract must name this reconciliation
explicitly (update `client.md` R1.3 and/or restore equivalent provenance
in `text.rs`'s module doc) rather than leaving it implicit.

**Source** — T00 reviewer, pass 1 (2026-09-05).

## T02 closes direct helper-test deferral

**Disposition** — Closed. T02's Claude action-line module now depends directly
on `collapse_newlines` and `basename`, and `crates/dashboard/src/text.rs`
contains direct unit tests for both helpers (including multiline and bare-path
cases). The coverage no longer depends only on OpenCode's action-line tests.

**Source** — T02 runner triage (2026-09-05).

## `collapse_newlines`/`basename` have no direct unit tests

**Scenario** — T00 moved `collapse_newlines` and `basename` from
`opencode/action_line.rs` into the shared `crates/dashboard/src/text.rs`.
Their only test coverage is indirect, through `render_action_line`'s
tests in `opencode/action_line.rs`.

**Consequence** — Once T02 makes Claude depend on these helpers directly,
a regression in either function is detectable only by OpenCode's test
suite, not by anything exercising the Claude path.

**Deferral assumption** — T00's contract explicitly forbids new tests
here (pure move, no new behavior; a new test would imply new behavior
existing to test).

**Promotion trigger** — T02 should add direct unit tests for
`collapse_newlines`/`basename` once Claude's rendering path depends on
them, so a regression doesn't require running OpenCode's tests to catch.

**Source** — T00 reviewer, pass 1 (2026-09-05).
