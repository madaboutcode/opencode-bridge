<!-- contracts/T04-layout-spec.md. Read by both implementer and reviewer. -->

# T04 — layout.md (Mosaic layout + degrade states)

**Context** — goal: write the full spec for the area-proportional "Mosaic" layout algorithm and how it degrades at the edges (empty, too-small, overflow) · who uses it: M3's implementer, building the layout/render module against this as source of truth · scale: one layout algorithm, already verified against a real ratatui build · criticality: moderate — this section is CONFIRMED and build-verified, so faithfulness to the requirements doc matters more than fresh judgment; a human reviews before anything ships

**Boundaries** — owns: `docs/specs/dashboard/layout.md` (new file) · must not touch: any other file under `docs/specs/`, the requirements doc (read-only), `tmp/20260901-prototype-dashboard-layout/` (read-only reference, don't modify the existing spike), `src/`, `crates/`

**Conventions** — read `docs/specs/CLAUDE.md` (written by T02) before writing. Follow its format: R#/R#.# numbering carried over unchanged, one co-located Given/When/Then scenario per requirement, `[REVIEW: ...]` markers for carried-forward OPEN items.

**Source material** — `tasks/2026-09-01-opencode-dashboard.requirements.md`, sections: R5-R5.11 in full (squarify-based project-region + session-tile packing, weight rules, idle chip row, tile content ladder, viewport/resize behavior, minimum sizes, visible-set cap and ordering, the retracted positional-stability decision — R5.7 — state plainly that position is NOT guaranteed stable and why, don't soften it, R5.8's real-scale numbers, accent-color rule), R9-R9.2 (empty active-window panel, terminal-too-small panel, degrade hierarchy — grouped here as layout's edge-case output). Also read `tmp/20260901-prototype-dashboard-layout/BRIEF-v2.md` for the full tile-content regime table referenced by R5.3 (don't just cite it — pull the actual regime table into this spec so it's self-contained, per writing-specs' "lose the codebase and rebuild" bar). Mark R5.5 (min sizes needing rechecking) and the `a`-mode overflow policy as `[REVIEW: OPEN, see requirements doc]`.

**Skills to read and apply** — `writing-specs`

**Acceptance — done when** — `layout.md` covers R5-R5.11 and R9-R9.2 with original numbering preserved, includes the pulled-in tile-content regime table (not just a citation), each requirement has a co-located scenario, all OPEN items marked `[REVIEW: ...]`, and the file is self-contained enough that an M3 implementer could build the layout module without re-reading the requirements doc or the prototype's brief.

**Gate** — report-only (refine-loop)

**Dependencies** — T02 (needs `docs/specs/CLAUDE.md`'s conventions to exist first)
