<!-- Spawn prompt for T07's runner. Filled from templates/runner-brief.md. -->

You run the review loop for one task. Read the `refine-loop` skill and
`tasks/2026-09-02-opencode-dashboard/contracts/T07-crosslink-and-validate.md`.
Run the loop with implementer=`coder`, reviewer=`ask_opus` (run config:
`tasks/2026-09-02-opencode-dashboard/PLAN.md`, "M2 ground truth" section).
Max 2 passes. All of T02-T06 must already be gated (check
`tasks/2026-09-02-opencode-dashboard/gates/` for T02-T06 reports, and
`git log` for their commits) — if any are missing, don't proceed, report
back to the conductor instead.

Reviewer calibration: give it the contract's Context line verbatim and tell
it to read and apply the `writing-specs` skill before reviewing. Confirm the
gate report actually includes each of the 5 files' final validator verdict
(not just a claim that validation ran) — that evidence is part of this
task's acceptance criteria, not optional detail. Otherwise check
faithfulness and consistency, not edge-case pedantry. Do not hand it a
checklist of things to check.

- Write the gate report to `tasks/2026-09-02-opencode-dashboard/gates/T07-report.md`
  (shape: the conductor skill's `templates/report.md`); append deferrals to
  `tasks/2026-09-02-opencode-dashboard/deferred.md`. Include each of the 5
  files' validator verdict as evidence in the report per the contract's
  Acceptance criteria.
- **On pass, commit:** stage `docs/specs/dashboard/{overview,client,layout,visuals,interactions}.md`
  (only if you edited them for cross-link fixes) and `docs/specs/README.md`,
  plus your gate report and `deferred.md` — nothing else, never
  `git add -A`/`-u`. Message: `T07: cross-link pass + spec validation`. You
  are already on branch `conductor/opencode-dashboard`. If the index is
  locked, wait and retry.
- **On escalation** (pass-2 residuals above the line): stop — no 3rd pass, no
  commit. Report the escalation brief back to the conductor. Uncommitted work
  unwinds with a checkout if the verdict re-cuts the contract.
