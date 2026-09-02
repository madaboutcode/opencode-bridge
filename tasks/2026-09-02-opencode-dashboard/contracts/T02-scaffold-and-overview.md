<!-- contracts/T02-scaffold-and-overview.md. Read by both implementer and reviewer. -->

# T02 — spec-tree scaffold + overview.md

**Context** — goal: bootstrap `docs/specs/` for this repo (it doesn't exist yet) and write the top-level `overview.md` that every other M2 spec file depends on for shared conventions · who uses it: future M3 implementers (and the coordinator) as the source of truth for building the dashboard crate · scale: one small repo's spec tree, ~5-6 files total, no multi-team consumers · criticality: moderate — a wrong or missing convention here propagates into every sibling spec, but a human (the user) reviews before anything ships, this isn't unattended production

**Boundaries** — owns: `docs/specs/CLAUDE.md`, `docs/specs/README.md`, `docs/specs/dashboard/overview.md` (new files/dirs) · must not touch: `docs/specs/dashboard/{client,layout,visuals,interactions}.md` (owned by T03-T06, don't pre-create them — leave room for their authors, but you may add stub entries to `README.md`'s index for all five so T07 doesn't have to invent the index structure later), `tasks/2026-09-01-opencode-dashboard.requirements.md` (read-only source), anything in `src/` or `crates/` (implementation, not spec — that's M3)

**Conventions** — read `~/.claude/skills/writing-specs/SKILL.md` in full before writing anything. `docs/specs/CLAUDE.md` should state (per the skill's "Before anything" section): the format this project uses (R#/R#.# numbering, one co-located Given/When/Then scenario per requirement, `[REVIEW: ...]` markers for carried-forward open questions instead of silently resolving them), the consumer lens (spec describes observable behavior, not implementation), and file organization (the 5-file map below — no `glossary.md`/`interfaces/`, this run isn't using the `greybeard` process those exist for, see `tasks/2026-09-02-opencode-dashboard/decisions.md`'s "M2 decomposition" entry for why). `docs/specs/README.md` is the index: one line per spec file naming what it covers, plus the 5-file map so a reader knows where to look. This repo has no prior `docs/specs/` — you are establishing precedent for T03-T06, so keep the format doc itself short and concrete, not aspirational.

**Source material** — `tasks/2026-09-01-opencode-dashboard.requirements.md`, sections: R1, R1.1, R1.2 (Architecture & Scope, partial), R1.3 (summarize only — one paragraph stating the HarnessAdapter boundary exists and pointing to `client.md` for the full contract, which T03 is writing in parallel; do not duplicate R1.4-R1.8's detail here), R2 (TUI engineering conventions — brief, this is closer to a non-functional constraint than consumer-observable behavior, a short paragraph is enough), R3, R3.1, R3.2 (Data & Active Window), R5.8 (real usage scale — this governs why the whole design looks the way it does, worth stating prominently), R10 (Non-Goals). Translate to consumer-observable spec language per the skill's Create-mode process (what does someone see/experience, not "the code calls X").

**Skills to read and apply** — `writing-specs`

**Acceptance — done when** — `docs/specs/CLAUDE.md` and `docs/specs/README.md` exist and state the conventions above; `docs/specs/dashboard/overview.md` exists, covers every R-number listed in Source material, has a co-located scenario per requirement, states the file map (which spec owns which R-numbers) so a reader can navigate the tree, and does not duplicate R1.4-R1.8's detail (summary + pointer only). No implementation code touched.

**Gate** — report-only (refine-loop)

**Dependencies** — none (this is M2's pipeline-gating task — T03-T06 depend on this one)
