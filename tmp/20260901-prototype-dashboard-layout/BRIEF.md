# Dashboard layout spike — build brief

This is the shared source of truth for both the implementer (coder) and the reviewer
(opus) in the refine loop. Do not paraphrase it into a second version — read this file
directly.

## Goal

A throwaway Rust ratatui TUI that renders a two-level dashboard layout (project boxes
containing session cards) so a human can judge the design by looking at a real terminal
render. This is a design-validation prototype, not production code.

## Location

`/Users/ajeesh/projects/madaboutcode/opencode-mcp/tmp/20260901-prototype-dashboard-layout/`

Starting materials already in `src/`:
- `squarify.rs` — tested squarified-treemap algorithm, copied verbatim from
  `tmp/dashboard-spike/src/squarify.rs`. Use as-is, unmodified, at the project level.
- `palette.rs` — Tokyo Night colors, copied from `tmp/dashboard-chrome-prototype/src/palette.rs`,
  adapted to the new fixture's `Status` enum.

Reference only (don't copy): `tmp/dashboard-chrome-prototype/src/render/common.rs` and
`option_b.rs` for existing line-formatting conventions (nickname · title, status line,
truncation with ellipsis).

## Design decisions (final — implement, don't re-derive)

### Two-level structure
Project boxes (bordered, `Block::bordered().border_type(Rounded)`, title = project name,
border color = project's rotation color) tile the full terminal viewport with no gaps.
Inside each project box, session cards are packed.

### Project box sizing (the core thing being tested)
Box area ∝ session count `n` (subagent sessions do NOT count toward `n` — they render as
content inside their parent's card, never their own box/card). Computed via ONE call to
`squarify::squarify()` across all projects, `TreemapItem { weight: n as f64, ... }`,
target area = full terminal Rect (minus header/footer rows as needed). This is the
area-from-count mechanism — chosen specifically because deriving area straight from the
weight number via squarify avoids a proportionality-inversion bug that summing actual
variable card heights was found to have during design. Do not write a second area formula.

Minimum project box: `14×7` cells, else render a project-summary tile (name + status
counts, no cards) instead of packing cards into too-small a space.

### Session card model — two discrete sizes, not continuous scaling
- **Full card**: bordered, 3 content lines (5 rows outer: border+3+border).
  1. Title line: nickname (bold, status-colored) + " · " + wire title (dim, truncated).
  2. Status line: `"running · <elapsed>"` / `"needs-you · question"` / `"needs-you · <elapsed>"`.
  3. Current-action line — see "Subagent content" for what replaces it.
  Used for `running` and `needs-you` sessions — both get the FULL form; prominence
  between them is carried by color only, not size.
- **Minimal card**: bordered, 1 content line (3 rows outer: border+1+border). Content:
  dim nickname + " · " + short status (e.g. "idle 40m"). Used for `idle` sessions.

Card width: start from `CARD_SLOT_WIDTH = 26`, `CARD_GAP = 2` (from the old common.rs),
adjust once real renders are visible.

### Subagent content — build BOTH modes, toggle live with `s`
Session may have zero or more active subagents. Two ways to show on the full card's
line 3, switchable at runtime without restart:
- **Substitution (default)**: line 3 shows subagent activity instead of parent's own
  action, e.g. `"↳ 2 subagents: editing render.rs"`. Card stays exactly 3 lines.
- **Append**: keep the parent's own action line AND add a line per subagent (or one
  combined line) — card grows taller than 3 content lines when subagents present.
  Implement plainly even though it breaks the fixed-height assumption — the point is to
  compare both against a real render, not to hide append's flaws.

Subagents never get their own card/box — only ever content inside the parent's card.

### Space pressure / demotion (replaces old "cap at 3 + overflow chip")
Within a project's allocated Rect, try all sessions as full cards, shelf-packed
(row-wrap flow, reuse the flow-wrap logic style from the old `layout.rs`). If not all
fit, demote to minimal one at a time, lowest-priority-demotes-first order:
`needs-you first → longest-waiting → running → most recent idle`.

**[UPDATED 2026-09-01, after seeing the first render]** The old rule stopped here and
fell straight to an all-or-nothing project-summary tile (zero cards) once all-minimal
still didn't fit. That's too coarse — real fixture testing showed several projects
falling back to a bare header with a large empty box beneath, even ones with
comfortable-looking room, which lost far more information than necessary. Replace the
last step with a proper progressive-disclosure ladder:

1. All sessions as full cards (running/needs-you) + minimal cards (idle), shelf-packed.
2. If that doesn't fit: demote per the priority tiers above until everyone is minimal.
3. If STILL doesn't fit with everyone minimal: collapse idle sessions into a single
   small chip (`+N idle`), freeing that space, and keep running/needs-you sessions as
   individual minimal cards as long as they fit.
4. Only if even (active sessions as minimal cards + one idle chip) doesn't fit, fall
   back to the full project-summary tile (name + counts, zero cards) — this should now
   be rare, reserved for genuinely tiny boxes, not a common outcome.

Also: before implementing step 3/4, first investigate and fix why the current build
renders several projects (including ones with plenty of visible room, e.g. a 1-session
project and a 6-session project) as bare header-only summary tiles with large empty
space beneath — that's a bug, not this design's intended behavior, and needs a root
cause, not just the new fallback logic layered on top of a broken trigger condition.

Below `40×12` total viewport: centered "terminal too small" message, nothing else.

### Position stability — deferred, note in a comment
Recompute layout fresh each frame from squarify; no debounce/stability logic in this
spike (that's a follow-up once the static shape is validated).

## Mock fixture — one ragged fixture, all cases at once

7-9 projects, must include all of:
1. A project with exactly 5 sessions and one with exactly 4, positioned so their boxes
   render near each other — checks whether 5 visibly looks bigger than 4.
2. A 1-session project (small box).
3. An all-idle project immediately adjacent to a busy (running/needs-you mix) project.
4. A project with 10+ sessions to trigger demotion-under-pressure; ideally one large
   enough to hit the summary-tile fallback too (note in the report if fixture size
   couldn't reach that path).
5. A session with 1 active subagent, and a session with 3 concurrent subagents (worst
   case for both subagent-content modes).
6. A session with an adversarially long title (60+ chars) and long nickname.
7. A mix of `needs-you` with a question vs without.

## Runtime behavior
- Alt screen + raw mode, panic-safe terminal restore.
- `q`/`Esc` quits cleanly.
- `s` toggles subagent substitution/append, re-renders immediately.
- `w` toggles simulated 80-column width vs real terminal width.
- Footer line showing current toggle states.

## Evidence of done
`cargo build` clean, no warnings introduced. Report: any ambiguous spec point and what
was chosen and why; any fixture case not confident looks right; exact keybindings
restated. If 5-vs-4 boxes look wrong despite following the spec, report it as a finding
— don't quietly change the algorithm to paper over it.
