# Spec Delta — claude-dashboard-activity-capture

## MODIFIED

- **R13** (dashboard/claude.md): allowlist widened from 3 events
  (`SessionStart`, `StopFailure`, `SessionEnd`) to 12 (adds
  `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PostToolUseFailure`,
  `PermissionRequest`, `Notification`, `Stop`, `SubagentStart`,
  `SubagentStop`).
  reason: the T01c `[REVIEW]` blocking these events was "unverified without
  an authenticated session." Fetched the published Claude Code hooks
  reference (code.claude.com/docs/en/hooks, 2026-09-05) and cross-checked
  against a live session transcript in this repo — every field is
  documented, closing the blocker. User decision: capture full session
  activity (tool calls, results, subagent identity), not lifecycle-only.

- **R14** (dashboard/claude.md): renamed "Metadata-only privacy boundary" to
  "Bounded activity fields." Per-event field table replaces the single
  metadata-only list; `tool_input`, `tool_response`, `prompt`,
  `agent_prompt`, `last_assistant_message`, `notification_message`, `error`
  now cross the boundary (bounded/truncated), where before they were always
  discarded. `transcript_path`/`agent_transcript_path` remain discarded
  under every event — no change there.
  reason: same decision as R13; these are the fields that carry the
  "activity" the previous metadata-only design could not show.

- **R15** (dashboard/claude.md): hook input cap 64 KiB → 256 KiB; envelope
  cap 8 KiB → 24 KiB; adds a new per-field bound (4096 UTF-8 bytes,
  truncate-not-drop) for fields marked "(bounded)" in R14.
  reason: R14's new content fields (tool output, assistant text) can
  legitimately exceed the old identity-only bounds before truncation is
  applied.

- **R17** (dashboard/claude.md): `[REVIEW]` reframed from "schema unverified,
  needs T05" to "schema resolved by this revision's evidence; only
  operational proof (real hook registration, live ordering/exit-path
  behavior) remains open."
  reason: the schema half of T05's scope is now evidence-backed; only the
  live-wiring half is still outstanding, and this change's implementation
  phase performs that proof.

## MODIFIED (round 2 — advisor review)

- **R13**: added an explicit selection criterion (three questions: alive/
  doing, needs-you-and-cleared, finished-and-how) replacing the unfalsifiable
  "activity" framing from round 1. Added `PermissionDenied`, `Elicitation`,
  `ElicitationResult` (12 events -> 15). Exclusions now each cite the
  criterion or a stated rate rule instead of being asserted. `TeammateIdle`
  explicitly deferred via `[REVIEW: OPEN]`, not silently excluded.
  reason: advisor review found R13 asserted its list without a criterion
  that could justify inclusion or exclusion, and found `PermissionRequest`
  had no exit event (a real bug: a tile could get stuck needing attention
  forever, wrongly, on the denial path). Blocking item per advisor.

- **R14**: added a field-by-field "(bounded)" vs "(label)" distinction
  (round 1 used "or" between closed-set and length-bound validation, which
  is unimplementable — closed-set validation would reject legitimate MCP
  tool names). Named exactly which two fields are closed sets (`source`,
  `reason`); everything else is length-only. Added an explicit privacy
  statement for allowlisted content (memory-only, never logged, never
  persisted, never in a crash report), replacing the "metadata-only"
  guarantee round 1 deleted without a replacement.
  reason: advisor review — the closed-set-or-length "or" is not
  implementable as written, and the privacy guarantee for kept content was
  the strongest part of the old spec and had gone silent in the new one.
  Both blocking items per advisor.

- **R15**: raised the whole-payload drop threshold 256 KiB -> 2 MiB and
  restated the bound structure as two independent kinds (whole-event drop
  vs. field truncation) so they cannot contradict each other. Fixed a
  units mismatch (R15's own scenario said "128-character bound" where the
  requirement text says "128 UTF-8 bytes").
  reason: advisor review found the 256 KiB whole-payload drop could still
  fire on a legitimately large single tool result (e.g. `Read` of a big
  file) before R14's own truncation ever got a chance to run — the two
  requirements disagreed on what happens to oversized content. Blocking
  item per advisor. The 4096-byte per-field cap itself was reviewed
  against the alternative of shrinking it to what current_action's
  one-line render needs (~512 bytes) and deliberately kept large: no
  detail-view consumer exists yet, but the decision is to carry a larger
  raw preview now rather than wait for one (user decision, not a default).

- **R16**: added an explicit rate/flood requirement. Previously R13/R15/R16
  bounded individual event size and connection count but never stated what
  a session emitting many small events per second (e.g. a bulk edit run)
  looks like on the dashboard side.
  reason: advisor review — found no requirement anywhere bounded event
  *rate*, only per-event size and per-connection behavior. Not one of the
  three items advisor said it would block on, but requested before sign-off.

## Not in scope (left for a later decision, not silently resolved)

- The full Claude Code hook event catalog (`TaskCreated`, `ConfigChange`,
  `WorktreeCreate`, `PreCompact`, model-switch events, etc.) was reviewed
  against the same evidence source but excluded — no activity signal this
  dashboard renders. If a future need appears, it is a new R13 addition,
  not an extension of this delta.
