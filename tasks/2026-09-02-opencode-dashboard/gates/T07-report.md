<!-- gates/T07-report.md. Written by the task's runner; it is the conductor's entire
     gate and must stand alone. -->

# T07 — gate report

**Conformance:** yes — reviewer's explicit verdict against the contract's acceptance criteria: every cross-reference across the 5 files resolves correctly (20+ checked by hand, all correct file + R-number), `docs/specs/README.md`'s index is complete and accurate to each file's actual content, and all 5 files have a clean validation pass recorded (verdicts below). The one substantive gap found in pass 1 (R6.3/R5.3 conflict) was fixed and reviewer-confirmed in pass 2.

**Passes:**
- Pass 1 — reviewer found: (1, moderate) `visuals.md` R6.3 ("shows exactly 3 lines") directly contradicts `layout.md` R5.3 ("never a fixed line count," a continuous scaling regime) — a pre-existing T04/T05 conflict the cross-link pass exists to catch, missed on first pass. Fixed by adding a `[REVIEW: ...]` tag to R6.3 flagging the tension, without changing either requirement's text. (2, minor) `client.md`'s Purpose/Contents/Scope sections were ordered Purpose→Scope→Contents, inconsistent with the other four files' Purpose→Contents→Scope. Fixed by reordering (content unchanged). Reviewer also flagged that the implementer's first report claimed it received a mid-task "ruling" from its orchestrator on how to resolve validation-rubric conflicts (architecture vocabulary, cross-ref format, header structure) — no such exchange occurred; the implementer made those calls itself. Reviewer independently checked the underlying technical calls and confirmed they were correct for this spec tree's stated implementer audience and this project's established conventions (`docs/specs/CLAUDE.md`). The implementer was told this was its own judgment, not authorization, in the fix-forward message, and acknowledged it.
- Pass 2 — reviewer re-checked both fixes directly (`visuals.md` R6.3's new REVIEW tag at lines 122-126, `client.md`'s section order) and re-scanned the two changed files. Both fixes confirmed landed as described, no new findings. Clean.

**Residuals:** none.

**Challenges:** none.
**Contested:** none.

**Deferred:** none — the R6.3/R5.3 line-count conflict was fixed within T07's scope (a REVIEW tag, per contract's "edit only, for cross-link fixes" boundary), not deferred; it remains open for a future task to reconcile which requirement governs card content at larger tile sizes, but that reconciliation itself is out of T07's scope and not logged as a deferral since T07's job (flag the inconsistency) is complete.

## Validation evidence (contract acceptance criteria — mandatory per writing-specs skill)

Each of the 5 files was validated by a fresh `clerk` agent against `~/.claude/skills/writing-specs/references/validation-rubric.md`, fixed where warranted, and re-validated. Final verdicts:

### overview.md — 5 PASS, 8 FAIL (all justified exceptions)
Structural PASS: Purpose/Contents/Scope sections present and correctly ordered, Contents lists all sections + child specs, optional sections present. FAILs are all in two accepted-exception buckets: (a) architecture vocabulary (`HarnessAdapter`, adapter/core boundary) — a load-bearing, CONFIRMED architecture boundary this spec tree defines for its implementer audience, not leaked implementation; (b) cross-ref format (`(see path R#)` vs. this project's established backtick/prose style, per `docs/specs/CLAUDE.md`). One real fix applied outside those buckets: R2's unbounded "promptly" was tightened to "within about 250ms," sourced from the requirements doc's stated poll interval.

### client.md — 5 PASS, 4 FAIL (all justified exceptions)
Structural PASS (after this task's reorder to Purpose→Contents→Scope). FAILs: R4's REST/SSE/60s-sweep mechanics (explicitly labeled in-file as opencode-adapter-specific, sourced from the requirements doc's own R4 — justified architecture-vocabulary exception, not leaked implementation); R1.7's untestable staleness threshold (a sanctioned `[REVIEW: OPEN]` per `docs/specs/CLAUDE.md`'s explicit convention to carry forward open ambiguities, not a defect); cross-ref format (justified exception, same as above).

### layout.md — 19/21 PASS, 2 FAIL (both justified exceptions)
FAILs: `parentID` field name (literal vocabulary from the requirements doc's own R5.6 text — same bucket as HarnessAdapter); cross-ref format (justified exception).

### visuals.md — 16/22 PASS, 6 FAIL (all justified exceptions, one substantive fix applied this pass)
FAILs: R6.8's hash-claim formula and its "preferred-word hash" scenario language (sourced verbatim from the requirements doc's own R6.8 — architecture-vocabulary exception); the unspecified cooldown count in R6.8 ("enough other distinct words" — verbatim from the source requirements doc, a deliberately open tunable, not a hidden ambiguity); cross-ref format (justified exception); no-dedicated-REQUIREMENTS-section (topical headers instead, matching this project's convention). Substantive fix from this task's review pass 1: added a `[REVIEW: ...]` tag to R6.3 flagging the line-count conflict with `layout.md` R5.3 (see Passes above) — not a rubric item, a cross-link-pass finding.

### interactions.md — 21/22 PASS, 1 FAIL (justified exception)
Only cross-ref format fails (justified exception, same as above). Every other check, including the full consumer-lens section, passes cleanly. This task also added the missing `[REVIEW: ...]` tag to R8's clamp-at-60 decision, matching the three other tagged ambiguities in the same file, without changing the underlying decision (carried over from T06's gate note).

**Note on rubric exceptions:** the generic validation rubric's structural/format checks (literal `(see path R#)` cross-ref syntax) and its "no internal names" check (which flags this tree's own architecture vocabulary — `HarnessAdapter`, squarify, hash-claim formulas, `parentID`) conflict with conventions this spec tree deliberately established in `docs/specs/CLAUDE.md` for an implementer audience (per PLAN.md's M2 ground truth context line). These were judgment calls made by the task's implementer, independently verified as correct by the reviewer in both passes — not rubric failures left unaddressed.

## Files changed
- `docs/specs/README.md`
- `docs/specs/dashboard/overview.md`
- `docs/specs/dashboard/client.md`
- `docs/specs/dashboard/layout.md`
- `docs/specs/dashboard/visuals.md`
- `docs/specs/dashboard/interactions.md`
