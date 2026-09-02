# docs/specs — index

Functional specs for the opencode dashboard TUI. Read `CLAUDE.md` in this
directory first — it defines the format and the skill gate.

## Spec files

- **`dashboard/overview.md`** — what the system is, its two-crate scope, the
  harness-agnostic core boundary (summary), the active/idle time window, real
  usage scale, and V1 non-goals. Start here.
- **`dashboard/client.md`** — the `HarnessAdapter` boundary in full, session
  snapshots, session identity, project identity, staleness, multi-harness
  tagging, the opencode adapter's REST/SSE mechanics, and
  tool-call-to-action-line rendering.
- **`dashboard/layout.md`** — the area-proportional "Mosaic" layout:
  project/session weighting, squarify packing, tile content scaling, minimum
  sizes, and degrade/empty/too-small states.
- **`dashboard/visuals.md`** — look and feel: TUI chrome, card content
  lines, the running/needs-you/idle attention model, and the session/project
  naming scheme.
- **`dashboard/interactions.md`** — keyboard navigation, zoom, and the
  active-window controls.

## File map (which R-numbers live where)

| File | Covers |
|---|---|
| `overview.md` | R1, R1.1, R1.2, R1.3 (summary only), R2, R3-R3.2, R5.8, R10 |
| `client.md` | R1.3 (full), R1.4-R1.8, R4, R6.4-R6.6 |
| `layout.md` | R5-R5.11, R9-R9.2 |
| `visuals.md` | R6, R6.1-R6.3, R6.7, R6.8 |
| `interactions.md` | R7-R7.1, R8-R8.1 |

No `glossary.md`, no `interfaces/` — see `CLAUDE.md`'s "File organization"
section for why.
