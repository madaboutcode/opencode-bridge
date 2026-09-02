<!-- contracts/T05-visuals-spec.md. Read by both implementer and reviewer. -->

# T05 — visuals.md (card content, attention model, chrome, nickname scheme)

**Context** — goal: write the full spec for what a session card shows, the 3-state attention model, the open chrome axis, and the full R6.8 nickname/naming scheme · who uses it: M3's implementer, building the card-rendering module and the naming resolver against this as source of truth · scale: one card format, 10 nickname categories/60 words already frozen · criticality: moderate-to-high on R6.8 specifically — the naming scheme has real correctness properties (two hard uniqueness guarantees) that must be transcribed precisely, not paraphrased loosely; a human reviews before anything ships

**Boundaries** — owns: `docs/specs/dashboard/visuals.md` (new file) · must not touch: any other file under `docs/specs/`, the requirements doc (read-only), `src/`, `crates/`

**Conventions** — read `docs/specs/CLAUDE.md` (written by T02) before writing. Follow its format: R#/R#.# numbering carried over unchanged, one co-located Given/When/Then scenario per requirement, `[REVIEW: ...]` markers for carried-forward OPEN items.

**Source material** — `tasks/2026-09-01-opencode-dashboard.requirements.md`, sections: R6 (TUI look, Tokyo Night/JetBrains Mono), R6.1 (status via color/glyph/order, not geometry), R6.2 (chrome axis — mark as `[REVIEW: OPEN]`, the three options A/B/C are still under test, do not pick one), R6.3 (3-line card content), R6.7 (3-state attention model: running/needs-you/idle, including the question-badge heuristic — mark the exact phrase list as `[REVIEW: OPEN]`), R6.8 in full — this is the largest single piece: the two-layer hash+probe claim scheme (project→category, session→word), the cooldown-based recycling rule, the two hard uniqueness guarantees and their assumptions, the explicit reversal of the 2026-09-01 adjective+noun decision (state the reversal and why, don't silently drop the old rule), the coupling to R1.7 (cross-reference `client.md`'s R1.7 section), and the capacity-edge-case OPEN item. Copy the full 10-category/60-word Appendix from the requirements doc into this spec verbatim (it's frozen content, not a citation) — note it's held here until M3 promotes each list to its own `wordlists/<category>.txt` file per R6.8's "one category per file" rule.

For R6.3's line-3 "current action" content: state what appears on the card (a short rendered line like "editing: foo.rs") and cross-reference `client.md`'s R6.5 section for how that line gets produced — don't duplicate the tool→action-line mapping table here, that's an adapter mechanism, not a card-visual concern.

**Skills to read and apply** — `writing-specs`

**Acceptance — done when** — `visuals.md` covers R6-R6.3 and R6.7-R6.8 with original numbering preserved, R6.8's full scheme (both claim layers, recycling, both guarantees, the reversal, the R1.7 coupling cross-reference) transcribed precisely enough that a reader unfamiliar with the requirements doc could implement it correctly, the full wordlist Appendix is copied in, each requirement has a co-located scenario, all OPEN items marked `[REVIEW: ...]`, and R6.3's action-line content correctly cross-references `client.md` rather than duplicating it.

**Gate** — report-only (refine-loop)

**Dependencies** — T02 (needs `docs/specs/CLAUDE.md`'s conventions to exist first)
