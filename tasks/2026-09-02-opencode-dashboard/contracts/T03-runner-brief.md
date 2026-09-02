<!-- Spawn prompt for T03's runner. Filled from templates/runner-brief.md. -->

You run the review loop for one task. Read the `refine-loop` skill and
`tasks/2026-09-02-opencode-dashboard/contracts/T03-client-spec.md`.
Run the loop with implementer=`coder`, reviewer=`ask_opus` (run config:
`tasks/2026-09-02-opencode-dashboard/PLAN.md`, "M2 ground truth" section).
Max 2 passes. `docs/specs/CLAUDE.md` (from T02) must already exist — if it
doesn't, T02 hasn't gated yet; wait rather than proceeding without it.

Reviewer calibration: give it the contract's Context line verbatim and tell
it to read and apply the `writing-specs` skill before reviewing — check
faithfulness to the requirements doc (especially R1.6/R6.5's precise
mechanics) and internal consistency, not edge-case pedantry. Do not hand it
a checklist of things to check.

- Write the gate report to `tasks/2026-09-02-opencode-dashboard/gates/T03-report.md`
  (shape: the conductor skill's `templates/report.md`); append deferrals to
  `tasks/2026-09-02-opencode-dashboard/deferred.md`.
- **On pass, commit:** stage `docs/specs/dashboard/client.md`, plus your gate
  report and `deferred.md` — nothing else, never `git add -A`/`-u`. Message:
  `T03: client.md (harness-adapter spec)`. You are already on branch
  `conductor/opencode-dashboard`. If the index is locked (a parallel task
  committing), wait and retry.
- **On escalation** (pass-2 residuals above the line): stop — no 3rd pass, no
  commit. Report the escalation brief back to the conductor. Uncommitted work
  unwinds with a checkout if the verdict re-cuts the contract.
