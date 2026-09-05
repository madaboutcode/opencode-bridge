# Claude subagent visibility — BLOCKED, not ready to implement

## Status: UNBLOCKED 2026-09-05 — see
`2026-09-05-claude-dashboard-activity-capture.spec-delta.md` and the revised
`docs/specs/dashboard/claude.md` R13/R14. The unblock condition below was
met by the published Claude Code hooks reference
(https://code.claude.com/docs/en/hooks, fetched 2026-09-05), a primary
source that documents `SubagentStart`'s `agent_id`/`agent_type`/
`agent_prompt` and `SubagentStop`'s `agent_id`/`agent_type`/
`last_assistant_message`/`stop_reason` by name — stronger evidence than the
field-names-only local capture Task 1 below proposed, because it is the
vendor's own dated contract rather than one observed session's payload.
Task 2 (the adapter wiring: a subagent event tracked as its own session,
keyed `"{session_id}:{agent_id}"`, with `parent_id` set to the top
session) is now in scope for the implementation brief that follows this
spec revision. This file is kept for its investigation trail; do not
restart Task 1.

## Status (original, superseded above): PENDING — blocked on evidence, not a decision

Not implementable yet at all (not even a decision away from it): needs an
authenticated Claude session that actually invokes the Task tool, captured
through hooks, before anyone can write a real allocation/schema plan. See
Task 1.

## Purpose

User question: "what about subagents from Claude — how does that work with
the Claude adapter?" Answer, confirmed directly in code: it doesn't. A
Claude subagent (Task-tool sub-conversation) is fully invisible today, and
— unlike the other three plans in this batch — the gap is not just unbuilt
code, it's an unverified wire schema. This file documents the investigation
and the exact unblock condition; it is deliberately NOT a build plan.

## Findings (confirmed, not delegated-only)

- `parse_hook_input` (`crates/dashboard/src/claude/hook.rs:336-352`) matches
  `hook_event_name` against exactly `"SessionStart"`, `"StopFailure"`,
  `"SessionEnd"` (R13's allowlist). `SubagentStart`/`SubagentStop` hit the
  `_ => dropped(DropReason::UnknownEvent)` arm and are discarded before any
  other field is read. `ClaudeEvent` (`hook.rs:203-206`) has exactly three
  variants — a subagent event cannot be represented even if it parsed.
- The general nesting mechanism this would need already exists and works:
  `SessionSnapshot.parent_id` (`snapshot.rs`) is live-wired for OpenCode
  (`opencode/reconcile.rs:125-126`, sourced from the server's own
  `SessionInfo.parent_id`) and consumed by `mosaic/view.rs::build_projects`
  (`view.rs:213-260`) to group children into a parent's `SessionView.subs:
  Vec<SubagentView>` — the same list `ladder.rs::subagent_line` already
  renders. For Claude, `claude/state.rs:166` hardcodes `parent_id: None`
  unconditionally for every snapshot it builds. The display mechanism is
  not the gap — the Claude adapter simply never feeds it anything.
- `tasks/spikes/2026-09-03-claude-dashboard-support/EVIDENCE.md` (S5):
  "the hooks reference indicates `SubagentStart`/`SubagentStop` include
  `agent_id` and `agent_type` fields, but these could not be verified" and
  "cannot verify if parent identity is representable in hooks" — no
  authenticated session ever exercised the Task tool during the T01c
  spike, so no one has seen an actual subagent hook payload.
  `redacted-schemas.md` has no subagent section at all.
- `tasks/2026-09-03-claude-dashboard-support/deferred.md`, "T01 - Exit-path
  reliability and subagent identity (S4/S5)" (lines 68-81), already tracks
  this exact gap: "subagent parent identity representation remains
  unverified for the adapter," with promotion trigger "T05 records...
  verifies subagent parent identity (or confirms it is not
  representable)." That trigger has not fired.

## Why this is blocked, not just unscheduled

`docs/specs/dashboard/claude.md` R14 (metadata-only privacy boundary) only
allows fields into the delivered envelope that are both observed and
allowlisted — R13's own `[REVIEW: ...]` marker says exactly this: events
outside the observed three are "unverified, not rejected by design"
pending T05. Adding `SubagentStart`/`SubagentStop` to the allowlist and
guessing at an `agent_id`/`parent_session_id` field shape now would mean
shipping a parser against a schema nobody has actually seen — the same
mistake R14 exists to prevent for every other field. This has to wait for
an authenticated Claude session that actually invokes the Task tool with
the hooks configured (the `claude-dash` wrapper set up earlier in this
session is exactly the tool for that capture, once someone runs it with a
Task-tool-using prompt).

## Unblock condition

1. Run an authenticated Claude Code session through `claude-dash` (or
   equivalent hooks config) with a prompt that spawns at least one
   subagent (a Task-tool call).
2. Capture the actual `SubagentStart`/`SubagentStop` hook payloads (field
   names present, whether a parent/session-id field exists and what it's
   called) the same way `redacted-schemas.md` captured the three verified
   events — field presence only, never values, per R14's own discipline.
3. Record the result in `deferred.md`'s T01 entry either way: a schema to
   build against, or written confirmation that no parent-identity field
   exists (in which case subagent nesting for Claude may not be
   representable at all, and this becomes a different, smaller
   "show Claude subagents as flat unrelated sessions" question instead).
4. Only then does a real plan (data model, allowlist extension, adapter
   wiring — mirroring `2026-09-05-dashboard-harness-tag.plan.md`'s shape)
   get written.

## Tasks

### Task 1: Capture an authenticated subagent hook trace
- **What:** Configure Claude Code hooks (via `claude-dash` or a
  project-level settings file) to also forward `SubagentStart`/
  `SubagentStop` to a local observer script (field-names-only logging,
  same discipline as `tasks/spikes/2026-09-03-claude-dashboard-support/
  hook-observer.sh`), run a prompt that triggers the Task tool, and record
  what fields actually appear.
- **Files:** a new spike script under
  `tasks/spikes/2026-09-03-claude-dashboard-support/` (or a fresh
  `2026-09-05-claude-subagent-spike/` dir if the existing one is
  considered sealed) — none of `crates/dashboard/src` yet.
- **Depends on:** none — this is the prerequisite for everything else in
  this file.
- **Agent:** you (Kai) or a coder agent with explicit field-names-only
  instructions (never log values, matching R14).
- **Verify:** a redacted schema doc showing observed field names for both
  events, or explicit confirmation neither ever fires without
  authentication (matching the honesty of the original `EVIDENCE.md`).

### Task 2: (blocked) Write the real implementation plan
- **What:** Once Task 1 produces a verified schema, write
  `<date>-claude-subagent-adapter.plan.md` following this batch's
  `dashboard-harness-tag.plan.md` shape (allowlist extension in `hook.rs`,
  new `ClaudeEvent` variants, `claude/state.rs` wiring `parent_id`).
- **Files:** TBD, depends on Task 1's findings
- **Depends on:** Task 1
- **Agent:** you (Kai) — this needs the same evidence-first discipline as
  the rest of the T01-T05 Claude work in this repo
- **Verify:** N/A until Task 1 lands
