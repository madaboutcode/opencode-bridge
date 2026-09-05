You are the advisor for the run `2026-09-05-claude-dashboard-fable-fixes`
(`tasks/2026-09-05-claude-dashboard-fable-fixes/`). Read the `advisor`
skill — your stance and boundaries. This brief adds what this conductor run
asks of you. On resume, read `delivery-profile.md`, `decisions.md`, and
`deferred.md` before judging anything.

You are the same `advisor` agent already running this session — this run
continues directly from the schema-widening and subagent-clearing-bug work
you already reviewed. Do not lose that history.

## What this run is

A separate agent (Fable 5.1, read-only, no code access) reviewed the
current Claude-monitoring implementation (`crates/dashboard/src/claude/`)
right after your round-2 sign-off, specifically for everyday-flow
robustness, usability, and simplicity — not edge-case hunting. Its full
findings are recorded verbatim in
`tasks/2026-09-05-claude-dashboard-fable-review-findings.md`. This run
implements all of them:

- **Finding 1** — `Notification` sub-types (`idle_prompt`,
  `permission_prompt`, `agent_needs_input`) are captured and shipped but
  never mapped to attention state, so an interrupted/denied turn leaves a
  tile stuck `Running` forever. Map them.
- **Finding 2** — the action line renders as literally `"tool: Bash"` and
  the recent-actions ring double-counts every tool call (PreToolUse and
  PostToolUse both push). Render a real action line (command/filename,
  mirroring `opencode/action_line.rs`'s pattern) and fix the double-push.
- **Finding 3 is OUT OF SCOPE for this run.** The reviewer recommended
  narrowing R14's ~20 captured fields down to the 6 production code
  actually reads. The user was shown that this reverses their own earlier
  explicit decision (an `AskUserQuestion` answer choosing "Larger raw
  preview, no consumer yet," made specifically to keep wide capture even
  with no consumer, because the original ask was "capture every darn
  thing"). First told to proceed with narrowing anyway; then, on reflection,
  reversed back to keeping the wide capture — "no need to pull it in if
  we are not using [narrowing], easy to reverse right?" So: field set
  stays as-is. No task in this run should touch which fields R14 captures.
  Do not let any other task (especially the structural smells or finding
  1/2/4) quietly narrow a field as a side effect — if a task's clean
  implementation would naturally drop an unused field, flag it rather than
  doing it, since that's finding 3 by another name.
- **Finding 4** — a permission prompt renders the Question tile with the
  *previous* turn's closing text, because `final_assistant_text` is never
  cleared on a new prompt and `PermissionRequest`/`Elicitation` set no
  content. Fix both.
- **Finding 5** — `SessionStart{source: compact}` resets an already-tracked
  session to `Idle` mid-turn, so a long session's tile vanishes from the
  grid during auto-compaction. Only initialize `Idle` for a genuinely new
  session.
- **Structural smells (4, listed in the findings file)** — turn-state
  triplication (`attention`/`turn_started`/`pending_tool_use_id` kept in
  sync by hand instead of derived), the 15-event wire schema hand-written
  three times (parse/encode/decode) where a serde derive would replace the
  encode/decode pair, a finished Claude turn never gets question-glyph
  detection that OpenCode already has (`question.rs`), and a finished
  subagent shows as needing attention until its tombstone arrives. **Also
  explicitly in scope for this run** (the user overrode the findings file's
  own recommendation to defer these until after live-proof).

**Explicitly out of scope for this run:** the live end-to-end proof against
a real Claude Code session (registering hooks, real transcripts) — that is
a separate, already-planned next phase this run does not touch or block on.
Do not let any task here grow into live-wiring work.

## Roster (user-approved)

- **advisor** — you, Opus, already running.
- **runner** — `coder` agent, one per reviewed task.
- **implementer** — `coder` agent.
- **reviewer** — `coder` agent (a fresh instance per task, never the same
  agent as that task's implementer). The user was shown `refine-loop`'s own
  "bind the strongest judgment available" guidance and explicitly chose
  `coder` over bumping to Opus for the reviewer role. That is a recorded
  user decision, not a gap to flag at every gate — but if a specific task's
  stakes genuinely warrant escalation, say so and let the user reconsider
  for that task alone, rather than silently accepting a weak review on
  something that matters.

## Ground truth for this project

- Spec: `docs/specs/dashboard/claude.md` (R11-R17) — the source of truth
  for the current 15-event allowlist and field table. Any task that changes
  captured fields, attention-state mapping, or wire shape must update this
  spec (`writing-specs` skill governs the edit) and produce a
  `spec-delta.md`-style record of what changed and why, same discipline as
  the two rounds you already reviewed.
- Code lives under `crates/dashboard/src/claude/` (`hook.rs`, `wire.rs`,
  `state.rs`, `mod.rs`, `listener.rs`, `command.rs`, `DESIGN.md`) plus
  `crates/dashboard/src/snapshot.rs` (the provider-neutral contract —
  `current_action`/`recent_actions` invariants finding 2 currently
  violates) and `crates/dashboard/src/opencode/action_line.rs` /
  `question.rs` (existing patterns findings 2 and the question-glyph smell
  should reuse rather than reinvent).
- Build/test: `cargo test -p dashboard`, `cargo clippy -p dashboard
  --all-targets`. Currently 333 tests passing, clippy clean — this run's
  exit bar includes staying green plus new regression tests per finding.
- Skills to name in contracts as they apply: `code-quality`,
  `software-design`, `writing-unit-tests`, and `writing-specs` for any task
  touching `docs/specs/dashboard/claude.md`.

## Note on how you're being spawned

You are a fresh process — the `advisor` agent from earlier in this session
is no longer reachable (the session that ran it appears to have ended or
reset). Per the `advisor` skill's "On session resume" guidance, treat
yourself as a resumed advisor, not a first-time one: this brief plus the
verdict history below is your memory of the run so far, since no
`decisions.md` was kept for the informal pre-conductor rounds.

**Prior verdicts you already reached, before this conductor run existed:**
1. Approved widening the Claude hook contract from 3 metadata-only events
   to 15 events with bounded content fields, across two review rounds,
   after finding and requiring fixes for: no exit event for
   `PermissionRequest` (added `PermissionDenied`/`Elicitation`/
   `ElicitationResult`), an ambiguous closed-set-vs-length-bound validation
   rule (resolved: only `source`/`reason` are closed sets), a whole-payload
   drop threshold that could fire before truncation ever ran (raised to
   2 MiB), and a units mismatch in a scenario. Full record:
   `tasks/2026-09-05-claude-dashboard-activity-capture.spec-delta.md`.
2. Found (and the fix confirmed correct) a real bug: `PermissionRequest`/
   `Elicitation` carry no `agent_id`, so the pending-clear flag always
   lands on the top-level session record; a subagent's permission-gated
   tool call routes to the subagent's own record, so the clear check never
   matched and the top-level tile got stuck `NeedsYou` forever. Fixed in
   `state.rs` with an explicit invariant comment and a bidirectional
   regression test.
3. Raised the scope question that led directly to this run's own
   boundary: registering hooks in any real `.claude/settings.json` (even
   user-level) starts capturing real session content immediately and needs
   to be disclosed, not assumed — the user independently and separately
   ruled out touching any real config, which is why this run's own
   out-of-scope line excludes the live-proof phase entirely.

## Draft the delivery profile

Turn the above facts into `delivery-profile.md` using its template. This is
an internal single-developer tool (the user monitoring their own Claude
Code sessions in this repo), not a multi-user product — release context is
"pre-live-proof refinement pass," not a hardening or security release.
Supported workflows are the ordinary daily flows the findings describe:
normal turn (prompt → tools → stop), permission approval, subagent
spawn/finish, an interrupted/escaped turn, and a session that
auto-compacts mid-conversation. Do not invent scale or concurrency this
project doesn't have.

## Scoping sign-off

Withhold until all seven definition-of-ready items hold, per the `advisor`
skill and this run's specifics: boundaries (the 5 findings + 4 smells,
named above); out-of-scope (live-proof phase, named above); project ground
truth (above); milestones real (propose to the conductor whether this is
one milestone or two — e.g. "attention-state and field-set fixes" then
"structural smells," given the field-set narrowing in finding 3 changes the
wire shape the structural-smell serde-derive task would also touch);
roles bound (above, already user-approved); git policy (a run branch, the
runner commits per task); delivery profile drafted and approved by the
user. Missing profile or approval → no sign-off.

## Review Frames

Author each reviewed task's Review Frame once its contract is stable,
per the `advisor` skill's usual 90-word cap. Given finding 3's field cut
interacts with what findings 1/4 need to keep, watch specifically for a
contract that narrows a field finding 1 or 4 still depends on — that's
exactly the kind of cross-task seam a Review Frame should flag rather than
let a reviewer discover cold.
