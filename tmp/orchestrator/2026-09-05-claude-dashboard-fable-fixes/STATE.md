# STATE — claude-dashboard-fable-fixes

Goal: implement fable 5.1 review findings 1/2/4/5 + 4 structural smells
against `crates/dashboard/src/claude/` (finding 3 excluded).
Current milestone: M2 — structural rewrites (signed off).
Git: branch `claude-dashboard-fable-fixes`, M1 committed at `b1ea617`; M2
signed off in `2325f3a`.
Active agents: none; advisor sign-off recorded in `gates/M2-review.md`.
Open escalations: none; M2 execution topology and T04 fallback escalation
resolved (`gates/M2-escalation.md`, `gates/M2-integration-escalation.md`).
Deferred count: 4 active (`current_action` not cleared at turn end, stale
snapshot ownership comment, broad legacy spec-rubric cleanup, and the
three-subtype module-comment wording nit). The helper-test and client.md R1.3
entries are closed by T02 and T01 respectively (see decisions.md and reports).

Scoping: APPROVED unconditionally (gates/scoping.md). Decomposition:
COMPLETE — 4 M1 tasks (T00 relocate shared helpers, T01 turn-termination,
T02 tile content, T03 session lifecycle), all sealed with advisor-authored
Review Frames. Pipeline order: T00 -> T01 -> T02 -> T03 (T01/T02 depend on
T00's exports; T03 independent but sequenced last for a clean base each
commit). Entering Execution.

## Tasks (this milestone)

| id | status | agent | gate result |
|---|---|---|---|
| T00 | gated | runner-T00 (coder) | PASS, committed 305f24c |
| T01 | gated | runner-T01 (coder) | PASS, committed d16330a |
| T02 | gated | runner-T02 (luna) | PASS, committed 3d3bfa2 |
| T03 | gated | runner-T03 (luna) | PASS, committed d1f41b7 |
| T04 | gated | direct-impl-T04-pass-4 (luna) | PASS, follow-up committed 71841e5; fan-in PASS |
| T05 | gated | direct-impl-T05-pass-4 (luna) | PASS, committed b88e56e |

T02 pass-1 reviewer verdict: conformance yes, no blocking defects. 360 tests
passed / 0 failed, clippy clean. The runner verified the genuine truncated JSON
boundary test, exhaustive `clear_pending_tool_use` routing, direct helper
coverage, and selective staging of only the R5.3 `layout.md` hunk. Commit
`3d3bfa2` contains the eight expected T02-owned paths; unrelated mosaic work
remains uncommitted.

## Next action

T02 and T03 bookkeeping is complete: both reports are on disk, the single
active deferral is recorded, and commits `3d3bfa2` and `d1f41b7` contain only
their expected task-owned paths.
Next action: proceed to the separately scoped live-proof phase using only an
isolated Claude settings path; correct the deferred DESIGN.md state-model note
before or alongside that phase.
Advisor is flagging: if T03 surfaces a fourth *independent* spec conflict
(distinct from T00/T01's one continuous found-then-fixed thread on
client.md R1.3), that's the signal the spec set itself needs a pass — don't
patch it as a one-off, raise it with advisor first.

Historical M1 sign-off and M2 decomposition are recorded above. M2 is now
signed off after the task and fan-in reviews. The next separately scoped phase
is live end-to-end hook-registration proof, using ONLY isolated test configs
(`claude --settings <scratch-path>`) — never touch real `.claude/settings.json`,
per the user's explicit standing instruction.

Full run record: `tasks/2026-09-05-claude-dashboard-fable-fixes/` (PLAN.md,
delivery-profile.md, advisor-brief.md, decisions.md, deferred.md,
contracts/T00-T05, gates/T00-T05 and M2 reports).
