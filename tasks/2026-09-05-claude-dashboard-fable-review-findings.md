# Fable 5.1 pragmatic design review — findings to investigate and act on

## Status: PENDING — findings not yet triaged into a build plan

This file records a full design review, unedited, run against the current
Claude-monitoring implementation (`crates/dashboard/src/claude/`) right
after the round-2 advisor-reviewed schema widening and the subagent
permission-clearing bug fix landed. The review agent (Fable 5.1, model
override, no code access to change anything — read-only) was asked to be
pragmatic, not pedantic: prioritize everyday-flow robustness, usability,
and simplicity over exotic edge cases. This is not a build plan yet —
findings need triage (which to fix, in what order, whether any need the
`advisor` teammate's sign-off before changing spec) before a Task list with
file/agent/verify assignments can be written.

## Purpose

User asked, verbatim: "can you ask a fable 5.1 agent to review the current
claude integration we have and the events and such - i wanna know if the
design is alright - can we improve it - can we simlpify it - can we better
the functionality, the usability etc - ask it to not make changes - and be
pragamatic about the improvements - not pedantic edge cases - but every day
typical flows much be smooth and robust."

## Reviewer's verdict, verbatim

> The shape — hook helper → Unix socket → adapter state → shared snapshot →
> tile — is right, and I would not reopen it. The listener/command
> hardening is heavier than a local socket strictly needs, but it's
> contained and tested; leave it. The problems are in what flows through
> the pipe and how the state machine uses it. Three of the five findings
> below are "the data is already captured, it just isn't used" or "it's
> used wrong". Ship and refine, but fix findings 1 and 2 before live-proof,
> otherwise live-proof will validate tile output that's misleading.

## Findings, in priority order (verbatim)

### 1. After Escape/interrupt, the tile says "running" until the window expires — and the signal that would fix it is captured and ignored

`Stop` is the only event that ends a turn (`state.rs:246-262`). Claude
Code's hooks reference says `Stop` does not fire on a user interrupt (this
is from the reviewer's memory of the docs — live-proof should confirm). So
the most common daily move — Escape, "no, do it this way" — leaves the
tile `Running` with the last tool action until the user types a new prompt
or the reclassify window `W` turns it idle. Same for denying a permission
interactively (no `PermissionDenied` — that fires for policy/hook denials
— and probably no `Stop`).

`Notification` with `idle_prompt` (Claude has been waiting at the prompt
~60s), `permission_prompt`, and `agent_needs_input` is exactly the "I'm
waiting on you" signal, and it is allowlisted, parsed, shipped, decoded —
and then `state.rs:239-245` only bumps `last_updated`. R13 puts it in the
"does it need me" bucket and then declines to map it.

**Direction:** map three sub-types and ignore the rest: `idle_prompt` →
`NeedsYou{question:false}` (turn ended, whichever path got there),
`permission_prompt`/`agent_needs_input` → `NeedsYou{question:true}`. This
makes the attention model robust to *which* exit path fired, which is the
whole worry behind the `pending_tool_use_id` machinery.

### 2. The action line is "tool: Bash", and every tool appears twice in recent actions

`state.rs:195` renders `current_action = "tool: {tool_name}"` and `:196`
pushes the bare tool name — on `PreToolUse` *and* `PostToolUse`. So the
extended running tile's elastic list reads `Read / Read / Edit / Edit /
Bash`, and the 5-entry ring holds 2.5 tool calls. It also violates the
snapshot contract twice: `snapshot.rs:149-152` says `current_action` is
"never a raw tool name", and `:169-171` says `recent_actions` never
includes the current action (the test at `state.rs:832` asserts the
opposite as intended behaviour).

Meanwhile `tool_input` (up to 4 KiB of JSON) is shipped on four event
kinds and never read. OpenCode already solves this in
`opencode/action_line.rs`: `shell` → the command, `edit` → `editing:
<basename>`, else `running: <name>`.

**Direction:** render the action line in `hook.rs`, where the `tool_input`
object still exists (`Bash` → command, `Edit`/`Write`/`Read` → basename of
`file_path`, `Grep` → pattern, `Agent` → description, else `running:
<name>`), ship it as one ~200-byte `action` field, and drop `tool_input`
from the wire. `PreToolUse` moves the old `current_action` into the ring
and sets the new one; `PostToolUse` touches neither.

### 3. R14 ships ~20 fields; the tile consumes 6

Production `process()` reads: `prompt`, `tool_name`, `agent_id`,
`tool_use_id`, `last_assistant_message`, `agent_prompt`. Every other
allowlisted field — `tool_input` (×4), `tool_response`, `error`,
`error_type`, `denial_reason`, `notification_type`, `notification_message`,
`elicitation_request`, `user_response`, `server_name`, `stop_reason`,
`model`, `source`, `reason` — is parsed, bounded, serialized, decoded,
validated, and dropped. Every reference to them in `state.rs` is inside
`mod tests`.

This is the "over-fitted to capture everything" answer: yes. The cost
isn't only the ~1300 production lines in `hook.rs` plus the 2 MiB / 24 KiB
bound gymnastics; R14 itself says all of this is sensitive content held in
memory for the session's lifetime — and `tool_response` (file contents,
command output) is the single most sensitive field and has zero
consumers.

**Direction:** the rule should be "a field enters R14 when a tile block
renders it". Cut to: rendered action line, `prompt`, `last_assistant_message`,
`agent_prompt`, `agent_id`/`agent_type`, `tool_use_id`, `source`,
`notification_type`. Add `tool_response`/`error` back when a detail pane
exists to show them. Narrow the spec now, before live-proof records
evidence against the wide table.

**Caution before acting on this one:** this reverses part of what R13/R14
round 1 and round 2 deliberately widened, with the `advisor` teammate's
sign-off, specifically because the user's original request was "capture
every darn thing... all activity in the session." Narrowing the spec back
down needs to go back to `advisor` and arguably back to the user before
being treated as settled — it is a product-scope reversal, not a bug fix.

### 4. A permission prompt renders as a Question tile showing the previous turn's answer

`PermissionRequest` sets `NeedsYou{question:true}` (`state.rs:199-209`)
but sets no content. The Question tile (layout.md R5.3) shows: badge, then
`final_assistant_text` as the elastic block, then `you: <prompt>`.
`final_assistant_text` is whatever the last `Stop` left —
`UserPromptSubmit` (`state.rs:162-171`) never clears it. So mid-turn-two,
the tile shows the question badge over turn one's closing message, and
nowhere says "wants to run `rm -rf build`" even though `tool_name`+
`tool_input` arrived on that very event.

**Direction:** `UserPromptSubmit` clears `final_assistant_text` (new turn,
old answer is stale); `PermissionRequest`/`Elicitation` set it to the
rendered ask (`"allow: rm -rf build"` / the elicitation request). Cheap,
and it turns the Question tile from misleading into useful.

### 5. `SessionStart{source: compact}` hides a mid-turn session

`state.rs:154-161` sets `Idle` on every `SessionStart`. Auto-compaction
fires `SessionStart` with `source: compact` mid-conversation on the same
session id. R5.1 excludes idle sessions from packing, so a long session's
tile vanishes into the footer during compaction and reappears on the next
tool event, with a grid recompute each way. Long sessions compact
routinely. `source` is one of the few captured fields that would matter
here, and it's unused.

**Direction:** `SessionStart` initialises only an untracked session; for a
tracked one, `compact`/`resume` leave attention alone.

## Structural smells (verbatim)

| Smell | Where | Direction |
|---|---|---|
| Three overlapping notions of "where in the turn are we": stored `attention`, separate `turn_started` (`state.rs:115`, comment admits the duplication), and `pending_tool_use_id`. Every transition has to keep them consistent by hand. | `state.rs:107-127` | Store facts (`turn_started`, `turn_ended`, `pending`) and derive `AttentionState` in `snapshot_event`. One source of truth; the known subagent-pending bug becomes a routing question, not a state-sync one. |
| Three hand-written copies of the 15-variant schema: `parse_*` (needed — truncation/serialization), `envelope_to_value` (`hook.rs:987-1234`), `decode_event` (`wire.rs:183-344`). Wire-side pair is ~400 lines that a `#[serde(tag="kind", rename_all="snake_case")]` derive replaces; unknown keys are still never read. | `hook.rs`, `wire.rs` | Derive on `ClaudeEvent` for the wire format; keep hand-parsing only on the Claude-payload side. |
| `Stop` always yields `question:false`; OpenCode runs `looks_like_question` on the same text (`reconcile.rs:112-115`). A Claude turn ending "Want me to update the tests too?" gets the plain glyph. | `state.rs:255` | Reuse `question.rs`; it's pure text. |
| `Stop` with `agent_id` marks a finished subagent `NeedsYou` until `SubagentStop` tombstones it. A finished subagent doesn't need anyone. | `state.rs:246-262` | Route subagent `Stop` to `Idle` (view.rs already drops idle subagents from the parent's list). |

## On the 15-event allowlist itself (verbatim)

> The event set is fine — each has a distinct job except `Notification`,
> which should have the biggest one (finding 1), and `PostToolUse`, whose
> only state-changing role is the permission-clear (finding 2 removes its
> duplicate push). The everyday flows — prompt → tools → stop, permission
> → approve → post-tool, subagent start → tools → stop — do work in the
> code as written; the `tool_use_id` clear is sound for the top-level
> case. The fragility is in the paths where no clearing event arrives
> (interrupt, interactive deny), and finding 1 is the generic answer to
> that rather than more per-event pairing.

## What has already happened since this review

Independent of this review, the same day: a real bug in the
`pending_tool_use_id` clearing mechanism (a subagent's permission-gated
tool call could leave the top-level session stuck `NeedsYou` forever) was
found by the `advisor` teammate, confirmed directly in code, and fixed in
`crates/dashboard/src/claude/state.rs` with a regression test and an
explicit invariant comment (see the "BUG FIX" comment in the
`PreToolUse|PostToolUse|PostToolUseFailure` match arm). `cargo test -p
dashboard` was at 333 passing / 0 failed and `cargo clippy -p dashboard
--all-targets` was clean at that point. This is unrelated to findings 1-5
above but touches the same file and the same event-handling logic — read
the current `state.rs` fresh rather than assuming it matches any earlier
description of it.

## Tasks

### Task 1: Triage findings 1, 2, 4, 5 into a build plan
- **What:** These four are small, self-contained, and don't touch the
  spec's scope (they're bug fixes against the existing R13/R14 field set,
  not field additions or removals). Write a proper `.plan.md` (File Tree /
  Data Flow / Boundaries / Testing Strategy / Tasks, matching this repo's
  convention — see `2026-09-05-dashboard-harness-tag.plan.md` for the
  shape) covering all four, or four small ones if they turn out to
  conflict on the same lines enough that one worker should own all of
  `state.rs` at once.
- **Files:** `crates/dashboard/src/claude/state.rs` primarily; finding 2
  also needs a place to render the action line (either `hook.rs` at parse
  time per the reviewer's suggestion, or `state.rs` if `tool_input` is
  kept on the wire — that choice needs to be made explicitly, not
  defaulted).
- **Depends on:** none — these don't require a spec change, only a
  `state.rs`/possibly `hook.rs` behavior fix. Confirm with `advisor`
  whether any of the four cross into "behavior the spec documents
  differently than this changes it" before implementation, since R14's
  co-located scenarios may need updating to match (e.g. finding 4 changes
  what `final_assistant_text` means, which R14's scenario text may
  reference).
- **Agent:** coder (sonnet) — this is straightforward implementation
  against a clear spec-level description once triaged.
- **Verify:** existing test suite stays green; add regression tests
  specifically reproducing each of the four scenarios (Escape mid-turn,
  two consecutive tool calls, a permission request mid-turn-two, a
  `SessionStart{source: compact}` on an already-tracked session).

### Task 2: Decide finding 3 (field-set narrowing) with `advisor` and the user
- **What:** Finding 3 recommends cutting R14's field list down to only
  what a tile renders today. This directly reverses part of the R13/R14
  widening the user explicitly asked for ("capture every darn thing") and
  that `advisor` already reviewed and signed off on across two rounds.
  Before treating this as a task to implement: bring finding 3 to
  `advisor` for its read (does it agree fields with zero consumers should
  be dropped now vs. kept for a future detail-view consumer, which the
  user already explicitly chose to keep per an earlier `AskUserQuestion`
  decision — "Larger raw preview, no consumer yet"), then bring both
  readings to the user for the actual call. This is a product-scope
  decision, not a bug fix — do not implement it as a side effect of Task
  1.
- **Files:** none yet — decision only.
- **Depends on:** none, can run in parallel with Task 1.
- **Agent:** you (Kai) — this needs the same judgment call as every other
  product decision in this thread, not a delegate.
- **Verify:** N/A until the decision is made.

### Task 3: Consider the structural smells for a later cleanup pass
- **What:** The four structural smells (turn-state triplication, hand-
  duplicated schema code, missing question-detection reuse, subagent
  `Stop` marking `NeedsYou`) are real but the reviewer explicitly did not
  rank them as blocking. Do not fold these into Task 1's urgency. Track
  them for after the live-proof phase (the one this whole thread has been
  building toward) lands, unless one of them turns out to interact with
  something the live-proof phase discovers.
- **Files:** TBD.
- **Depends on:** live-proof phase completing first, per priority.
- **Agent:** TBD.
- **Verify:** N/A — not scheduled yet.
