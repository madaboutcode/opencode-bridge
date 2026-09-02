# docs/specs/CLAUDE.md

Before creating, editing, or extracting any spec under this directory, you
MUST invoke the `writing-specs` skill. It has the full process; this file only
states this project's conventions.

## Format

- **Numbering.** `R1`, `R2`, ... for top-level requirements. `R1.1`, `R1.2`
  for sub-requirements. Numbers are stable identifiers — other specs, QA
  plans, and code comments reference them directly (e.g. "see overview.md
  R3.1"). Never renumber a requirement; if it's superseded, say so in place
  and keep the number.
- **One scenario per requirement, co-located.** Every requirement is followed
  immediately by its own `Given/When/Then` — the primary happy-path example,
  right where the requirement is written, not in a separate QA file.

  ```
  R3 — Active sessions expire after 15 minutes of inactivity.
    Scenario: Given a session with last_activity 16 minutes ago,
    when the expiry check runs, then session status is EXPIRED.
  ```
- **`[REVIEW: ...]` markers for open questions.** If a requirement carries
  forward an unresolved question from the source material, write
  `[REVIEW: <question>]` next to it. Do not silently resolve it by picking an
  answer while writing the spec — that's a design decision, not a transcription
  one.

## Consumer lens

Specs describe what a consumer of the system observes — what they see, send,
or receive — never how the code is structured internally. "The page shows an
error banner" is a spec. "The component calls `fetchSession` in a `useEffect`"
is not. If a sentence names a function, a struct, or a file, it belongs in
the implementation, not here.

## File organization

Five spec files under `docs/specs/dashboard/`, no more:

| File | Covers |
|---|---|
| `overview.md` | R1, R1.1, R1.2, R1.3 (summary only), R2, R3-R3.2, R5.8, R10 |
| `client.md` | R1.3 (full), R1.4-R1.8, R4, R6.4-R6.6 |
| `layout.md` | R5-R5.11, R9-R9.2 |
| `visuals.md` | R6, R6.1-R6.3, R6.7, R6.8 |
| `interactions.md` | R7-R7.1, R8-R8.1 |

No `docs/specs/glossary.md` and no `docs/specs/interfaces/` — the
`writing-specs` skill treats both as fixed locations, but they exist to serve
the `greybeard`/QA-agent process, which this project isn't using; see
`tasks/2026-09-02-opencode-dashboard/decisions.md`'s "M2 decomposition" entry.

Read `README.md` in this directory for the spec index.
