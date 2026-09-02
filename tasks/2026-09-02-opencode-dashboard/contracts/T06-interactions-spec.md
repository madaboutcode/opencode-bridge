<!-- contracts/T06-interactions-spec.md. Read by both implementer and reviewer. -->

# T06 — interactions.md (navigation, window controls)

**Context** — goal: write the full spec for keyboard navigation, zoom, and the active-window controls · who uses it: M3's implementer, building the input-handling module against this as source of truth · scale: a handful of keybindings, single-user local TUI · criticality: low-to-moderate — small, well-specified surface area, but wrong key semantics are directly user-facing and annoying to get wrong

**Boundaries** — owns: `docs/specs/dashboard/interactions.md` (new file) · must not touch: any other file under `docs/specs/`, the requirements doc (read-only), `src/`, `crates/`

**Conventions** — read `docs/specs/CLAUDE.md` (written by T02) before writing. Follow its format: R#/R#.# numbering carried over unchanged, one co-located Given/When/Then scenario per requirement, `[REVIEW: ...]` markers for carried-forward OPEN items.

**Source material** — `tasks/2026-09-01-opencode-dashboard.requirements.md`, sections: R7-R7.1 (at-a-glance view, `j/k`/arrow navigation, zoom, `q`/`Esc`/`?`, footer content), R8-R8.1 (window controls: `[`/`]`, `w`, `a`, `shift+[/]`, clamping, immediate-snap reflow, rebindable-later note). Cross-reference `layout.md`'s R9 section for what the footer/empty-state text actually says (R9's exact panel copy lives there, not here) rather than duplicating it.

**Skills to read and apply** — `writing-specs`

**Acceptance — done when** — `interactions.md` covers R7-R8.1 with original numbering preserved, each requirement has a co-located scenario (e.g. given the window is 10m, when `]` is pressed, then window becomes 15m and layout reflows immediately), and the file is self-contained enough that an M3 implementer could build the input-handling module from it without re-reading the requirements doc.

**Gate** — report-only (refine-loop)

**Dependencies** — T02 (needs `docs/specs/CLAUDE.md`'s conventions to exist first)
