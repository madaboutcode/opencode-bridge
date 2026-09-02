<!-- Spawn prompt for T01's runner. Filled from templates/runner-brief.md. -->

You run the review loop for one task. Read the `refine-loop` skill and
`tasks/2026-09-02-opencode-dashboard/contracts/T01-project-identity-spike.md`.
Run the loop with implementer=`coder`, reviewer=`ask_opus` (run config:
`tasks/2026-09-02-opencode-dashboard/PLAN.md`). Max 2 passes.

Reviewer calibration: give it the contract's Context line verbatim (goal / who
uses it / scale / criticality — criticality is low, scale is a handful of
fixtures, this is throwaway spike code) and tell it to read and apply the
`code-quality` skill before reviewing. Do not hand it a checklist of things to
check (no "focus on X, Y, Z" — that defeats the point of an independent
reviewer, per the refine-loop skill's own guardrail). The Context line's low
scale/criticality is what should naturally keep it off edge-case pedantry, not
an instruction telling it to ignore edge cases.

- Write the gate report to `tasks/2026-09-02-opencode-dashboard/gates/T01-report.md`
  (shape: the conductor skill's `templates/report.md`); append deferrals to
  `tasks/2026-09-02-opencode-dashboard/deferred.md`.
- **On pass, commit:** stage the contract's owns list (`tmp/2026-09-02-project-identity-spike/**`)
  plus your gate report and `deferred.md` — nothing else, never `git add -A`/`-u`.
  Message: `T01: project-identity resolution spike`. You are already on branch
  `conductor/opencode-dashboard`. Repo-wide side effects you didn't intend get
  checked out, not committed. If the index is locked (a parallel task
  committing), wait and retry.
- **On escalation** (pass-2 residuals above the line): stop — no 3rd pass, no
  commit. Report the escalation brief back to the conductor. Uncommitted work
  unwinds with a checkout if the verdict re-cuts the contract.
