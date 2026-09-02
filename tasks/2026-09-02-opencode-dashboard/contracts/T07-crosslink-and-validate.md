<!-- contracts/T07-crosslink-and-validate.md. Read by both implementer and reviewer. -->

# T07 — cross-link pass, README index, mandatory spec validation

**Context** — goal: make the 5-file spec tree internally consistent (cross-references actually resolve) and pass the `writing-specs` skill's mandatory independent validation · who uses it: same as the rest of M2 — M3's implementer, treating the spec tree as source of truth · scale: 5 files, ~1 repo · criticality: moderate — this is the last check before the spec tree is considered done; a broken cross-reference or a validation failure caught here is much cheaper than caught during M3

**Boundaries** — owns: `docs/specs/dashboard/{overview,client,layout,visuals,interactions}.md` (edit only, for cross-link fixes and validation-flagged issues — not a rewrite), `docs/specs/README.md` (finalize the index) · must not touch: the requirements doc (read-only), `src/`, `crates/`

**Conventions** — this task assumes T02-T06 have all gated and their five files exist. Read `docs/specs/CLAUDE.md` for the conventions those files were written under.

**Note from T06's gate (coordinator, 2026-09-02):** `interactions.md`'s R8 section resolves a genuine source ambiguity (whether `]` past 60m auto-transitions to "show all" or clamps at 60m — it picked clamp-at-60) without a `[REVIEW: ...]` tag, while three adjacent ambiguities in the same file got tagged. Minor consistency nit, not a defect (the reviewer already confirmed the interpretation itself is defensible) — add the tag while you're doing the cross-link pass if convenient, not worth a dedicated fix cycle on its own.

**Task, part 1 — cross-link pass.** Read all five spec files. Verify every cross-reference between them actually resolves (e.g. `visuals.md`'s pointer to `client.md`'s R6.5 section, `interactions.md`'s pointer to `layout.md`'s R9 section) — fix any that don't (wrong file, wrong R-number, or a reference to something that got renumbered). Check for unintentional duplication (the same content written out in two files instead of one owning it and the other cross-referencing) and fix by keeping the more appropriate owner and replacing the duplicate with a reference.

**Task, part 2 — README index.** Finalize `docs/specs/README.md`'s index: one line per file naming what it covers (T02 left a stub structure — complete it, don't restructure it without reason).

**Task, part 3 — mandatory validation (writing-specs skill requirement).** For each of the 5 spec files, spawn a `clerk` agent with this prompt: "Read the validation rubric at ~/.claude/skills/writing-specs/references/validation-rubric.md, then validate this spec file: <path>. Run every check in the rubric. Report PASS/FAIL for each item with a one-line explanation on failures. End with a summary verdict." Fix every failure yourself, then re-run validation for that file. Per the skill: **the spec is not done until every file's validator reports all items passing.**

**Skills to read and apply** — `writing-specs`

**Acceptance — done when** — every cross-reference across the 5 files resolves correctly, `README.md`'s index is complete and accurate, and all 5 files have a clean validation pass recorded (paste each file's final validator verdict into the gate report as evidence).

**Gate** — report-only (refine-loop)

**Dependencies** — T02, T03, T04, T05, T06 (all must be gated — this is M2's final pipeline stage)
