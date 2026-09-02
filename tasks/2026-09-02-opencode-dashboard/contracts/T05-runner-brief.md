<!-- Spawn prompt for T05's runner. Filled from templates/runner-brief.md. -->

You run the review loop for one task. Read the `refine-loop` skill and
`tasks/2026-09-02-opencode-dashboard/contracts/T05-visuals-spec.md`.
Run the loop with implementer=`coder`, reviewer=`ask_opus` (run config:
`tasks/2026-09-02-opencode-dashboard/PLAN.md`, "M2 ground truth" section).
Max 2 passes. `docs/specs/CLAUDE.md` (from T02) must already exist — if it
doesn't, T02 hasn't gated yet; wait rather than proceeding without it.

Reviewer calibration: give it the contract's Context line verbatim and tell
it to read and apply the `writing-specs` skill before reviewing. This task's
Context line flags R6.8 specifically as moderate-to-high criticality (real
correctness properties — two hard uniqueness guarantees — must be
transcribed precisely, not paraphrased loosely) — make sure the reviewer
actually checks R6.8's mechanics against the requirements doc line by line,
not just skims for prose quality. Otherwise, check faithfulness and internal
consistency, not edge-case pedantry. Do not hand it a checklist of things to
check.

- Write the gate report to `tasks/2026-09-02-opencode-dashboard/gates/T05-report.md`
  (shape: the conductor skill's `templates/report.md`); append deferrals to
  `tasks/2026-09-02-opencode-dashboard/deferred.md`.
- **On pass, commit:** stage `docs/specs/dashboard/visuals.md`, plus your
  gate report and `deferred.md` — nothing else, never `git add -A`/`-u`.
  Message: `T05: visuals.md (card content, attention model, nickname scheme)`.
  You are already on branch `conductor/opencode-dashboard`. If the index is
  locked (a parallel task committing), wait and retry.
- **On escalation** (pass-2 residuals above the line): stop — no 3rd pass, no
  commit. Report the escalation brief back to the conductor. Uncommitted work
  unwinds with a checkout if the verdict re-cuts the contract.
