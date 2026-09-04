# Dashboard — Layout ("Mosaic")

## Purpose

The area-proportional "Mosaic" layout: how project regions and session tiles
are packed and sized on screen, how tile content scales with the space it's
given, and what the dashboard shows when there's nothing to show or not
enough room to show it. Source: `tasks/2026-09-01-opencode-dashboard.requirements.md`
R5-R5.11, R9-R9.2, and (for the tile-content regime table pulled in under
R5.3) `tmp/20260901-prototype-dashboard-layout/BRIEF-v2.md`. This section of
the requirements doc is CONFIRMED — verified against a real ratatui build,
not a mockup (render evidence in that spike's `renders/` directory).

## Contents

- [Layout algorithm](#layout-algorithm) — R5-R5.11
- [Degrade and edge states](#degrade-and-edge-states) — R9-R9.2

No child spec files — this is a leaf file in the tree; see `overview.md`
for the full five-file map.

## Scope

Covered: the two-pass squarify packing (project regions, then session tiles
within a region — R5-R5.2), tile content scaling as a function of tile size
(R5.3), resize/recompute behavior (R5.4), minimum sizes (R5.5), the
per-project tile cap and overflow (R5.6), position (in)stability (R5.7),
real-usage-scale consequences (R5.8), accent-color placement (R5.11), and
the degrade/empty/too-small states (R9-R9.2).

Not covered: this file covers geometry and content-scaling rules only.
Exact colors, glyphs, and the attention-state model
(`running`/`needs-you`/`idle`) are `visuals.md`'s domain (R6, R6.7) — this
file cross-references them by name where a rule depends on state, but
doesn't redefine them. Session-snapshot data (what fields exist, how
"current action" text is produced) is `client.md`'s domain (R1.4,
R6.4-R6.6).

## Layout algorithm

- **R5** — The screen's body area is packed by an area-proportional
  algorithm — squarify, the Bruls et al. squarified-treemap algorithm, used
  unmodified — applied twice. First pass: it divides the body area into one
  rectangular region per project (R5.1). Second pass: it's applied again,
  separately inside each project's region, to divide that region's own area
  among the project's session tiles (R5.2). Tile content then
  scales with however much space that tile ends up with (R5.3) — there is no
  fixed card size. This replaced an earlier fixed-size-card ("flow-grid")
  design outright: at real usage scale (R5.8) that design left most of the
  screen blank, and it is not being revisited.

  Scenario: Given 4 projects with 3/2/2/1 sessions open, when the dashboard
  renders, then the body area is divided into 4 project regions sized
  roughly in proportion to session count, and each region is further divided
  into tiles for that project's own sessions — not into 4 identical
  fixed-size boxes with blank space around smaller cards.

- **R5.1** — Each project region's weight (the squarify algorithm's input
  for the first pass, R5) is the sum of R5.2's per-session weight over only
  the project's in-window sessions — a session outside the active window `W`
  (`overview.md` R3) contributes nothing to its project's weight, regardless
  of how many such sessions exist or what state they were last in. Subagent
  sessions never add a region of their own — a subagent is content inside
  its parent session's tile (R5.6), never a separate weight entry. A project
  whose in-window sessions sum to zero weight (including a project with only
  idle sessions) is excluded from the region packing entirely; it appears
  only in the footer as `hidden: <name> (N idle)`. Projects are always
  packed in first-appearance order and never re-sorted by weight or
  status — this ordering is kept because it lets the user visually compare
  specific projects side by side, not to hold screen position (R5.7 retracts
  that reasoning). Adjacent project regions are snapped to share exact
  edges — no visible gap and no overlap between neighboring regions.

  **Superseded in part:** the original wording of this
  requirement weighted a project region by "the number of top-level sessions
  it currently shows, counting all states (running, needs-you, idle)" — a
  flat count of a project's entire visible session history, in or out of the
  active window. At real usage scale (22 projects, ~230 sessions accumulated
  over time, but usually only 0-6 actually active at once), that let a
  project's region width scale with stale history rather than what's
  currently relevant: a project with 74 old sessions got 4-5x the area of
  one with 16, even though both had about 1 live session. This number (R5.1)
  is kept and the flat-count-of-all-states clause is void; the
  sum-of-in-window-weight rule above (sourced from R5.2's per-session weight
  table) is the current rule. The "project with no qualifying sessions is
  excluded" and "first-appearance order" clauses are unchanged — only how
  the weight number itself is computed changed. An "exempt a stuck
  `needs-you`/failed session from the window regardless of age" alternative
  was considered and explicitly rejected — the window cutoff is strict, no
  exceptions, confirmed twice by the project owner.

  Scenario: Given project A has 3 sessions, project B has 1 session, and
  project C has sessions that are all idle, when the dashboard renders, then
  A's region is roughly 3x the area of B's region, C does not appear as a
  region at all, and the footer shows `hidden: C (N idle)`.

  Scenario: Given project A has 1 running session inside `W` and 73 sessions
  outside `W`, and project B has 1 running session inside `W` and 15
  sessions outside `W`, when the dashboard renders, then A's and B's regions
  are the same size — both project weights equal R5.2's `running` weight,
  since none of either project's out-of-window sessions contribute anything
  — not a 74:16 ratio, which the old flat-count rule would have produced.

- **R5.2** — Each session tile's weight (the squarify algorithm's input for
  the per-region pass, R5) is the same urgency-based per-session weight
  R5.1 sums for the whole region: `needs-you` (either sub-state — question
  badge or plain, `visuals.md` R6.7) = 3, `running` = 2, plus 1 per subagent
  the session has. (Status still governs color, glyph, and sort order per
  `visuals.md` R6.1/R6.7 — never tile area.) Idle (out-of-window) sessions
  are pulled out of the tile packing entirely and carry no weight of their
  own — only the region's active (running/needs-you/question) sessions are
  packed into tiles. Idle sessions no longer get individual on-screen
  identification either: instead of a chip row of names, the region's
  bottom row (when shown) carries a single bare `+N idle` count, reusing the
  same small-chip mechanic R5.6 already uses for its own `+N sessions`
  overflow chip, rather than any per-session geometry. Region-height
  thresholds for the bottom row: `h ≥ 3` rows shows tag row + tiles + bottom
  row; `h == 2` drops the bottom row (its count still surfaces via the tag
  row's own state-count badges, R5.11); `h == 1` shows the tag row only. If
  a project has active sessions but reserving a row for the idle count would
  leave no room for tiles, the bottom row is dropped first. As with project
  regions, adjacent tiles are snapped to share exact edges — no gaps, no
  overlaps.

  **Superseded in part:** the original wording of this
  requirement gave idle its own weight of `1` (part of a table that also fed
  R5.1's now-superseded flat weight) and rendered out-of-window sessions as
  individual named chips — `○ nick · age`, e.g. `○ hawk-otter · 51m`, chips
  separated by two spaces in most-recent-first order — on the region's
  bottom row, falling back to `+N idle` only as an overflow once chips ran
  out of horizontal room. Per-session idle identification on screen is
  retired in favor of a bare count: the project owner tracks his own
  sessions and widens the active window (`interactions.md` R8) when he wants
  to check on older ones, and does not want individual idle names competing
  for space at real usage scale (`overview.md` R5.8). This number (R5.2) is
  kept; the idle-weight-of-1 and per-session-idle-chip clauses are void. The
  active-session weight table's shape, the region-height thresholds, and the
  tile-packing/snapping rules are otherwise unchanged — only idle's own
  weight (now zero, not one) and its bottom-row presentation (a count, not
  named chips) changed.

  Scenario: Given a project region 3 rows tall with one running session and
  one idle session, when the dashboard renders, then row 1 is the project
  tag, row 2 is the running session's tile, and row 3 reads `+1 idle` — the
  idle session is neither a second tile competing for tile area nor an
  individually named chip.

  Scenario: Given a project region with one `running` session and one
  `needs-you` session of otherwise identical content demand (no subagents),
  when the dashboard renders, then the `needs-you` tile gets a larger share
  of the region's tile-packing weight than the `running` tile (3 vs 2) —
  urgency, not just presence, now drives tile size, unlike the old flat
  active-session weight of 2 shared by both.

- **R5.3** — Tile content is a total function of the tile's own inner width
  (`wi` = outer tile width − 2, for the 1-column inset on each side) and
  height (`h`) — never a fixed line count. A tile scales from color-only, up
  through a single glyph or glyph+age, up through a 3-4 line "compact" form,
  up to an "extended" form built from priority content blocks plus one
  elastic block that absorbs whatever rows are left over. This fixes a
  concrete failure seen in testing: fixed-line cards inside a larger
  proportional box left blank space below the text instead of using it.

  Scenario: Given the same running session rendered once in a 12x3 tile and
  once in a 30x10 tile, when the dashboard renders both, then the small tile
  shows only glyph/nickname/status-line content while the large tile
  additionally shows current action, subagent lines, wire title, files
  touched, and a scrollable recent-actions list — the same session's content
  genuinely grows to fill the larger tile rather than staying fixed and
  leaving blank rows.

  ### Tile content regime table

  Pulled in in full from the build brief (`BRIEF-v2.md`) so this file is
  self-contained. `wi` = inner width in columns; `h` = tile height in rows.
  A `/` inside a cell separates what appears on successive rows.

  | `wi` | `h = 1` | `h = 2` | `h = 3–4` | `h ≥ 5` |
  |---|---|---|---|---|
  | `< 3` | colour only | colour only | colour only | colour only |
  | `3–5` | glyph | glyph / age | glyph / age | glyph / age |
  | `6–11` | `glyph age` | `glyph age` / nick | `glyph age` / nick / — | same as `h=2` |
  | `≥ 12` | `glyph nick · age` | `glyph nick` / `status · age` | **compact** (below) | **extended** (below) |

  `glyph` is the per-state status glyph (see `visuals.md` R6.7 for what each
  state's glyph is). `nick` is the session's assigned name (R6.8, see
  `visuals.md`), truncated with `…` to fit. `age` is an elapsed-time string
  (`9m`, `45s`, `2h`).

  ### Status line forms

  The status line (used standalone at small sizes and as one row of the
  compact/extended forms) has two widths:
  - `wi ≥ 16`: full text — `question · 9m` (shown as a badge), `needs-you · 22m`,
    `running · 3m`, `idle · 51m`.
  - `12 ≤ wi < 16`: shortened — badge ` ? 9m `, `need · 22m`, `run · 3m`,
    `idle · 51m`.

  Elapsed time is measured differently per state: needs-you counts since the
  turn ended; running counts since the current turn started; idle counts
  since the session's last update. If the session has subagents
  and there isn't room for dedicated subagent lines,
  the status line instead gets a suffix ` ↳N` (subagent count) whenever at
  least 4 cells remain on that line.

  ### Compact form (`wi ≥ 12`, `h = 3–4`)

  Row 1: `glyph nick`, bold, in the state's color. Row 2: one content line,
  truncated with `…` — for `question` it's the first line of the session's
  final assistant text; for `running` it's the current action (R6.5, see
  `client.md`); for `needs-you` (plain) and `idle` it's the wire title. Row
  3: the status line. Row 4, only if `h = 4`: the first subagent line
  (`↳ nick action`, truncated) if the session has one, else blank.

  ### Extended form (`wi ≥ 12`, `h ≥ 5`)

  Content blocks are laid out top to bottom in priority order — priority 1
  is never dropped. A blank rhythm row precedes a block only if that block
  actually renders. Whatever rows remain after the fixed blocks go to a
  single **elastic** block, which absorbs the leftover space. If the fixed
  blocks alone don't fit in `h`, blocks are dropped from the bottom of the
  priority list (lowest priority first) until what's left fits. Any rows
  left after the elastic block runs out of content stay tinted blank — no
  filler content, no decoration.

  Text wrapping: word-wrap to `wi`, hard-breaking a single word longer than
  `wi`. A block wraps when it has more than one row available; it truncates
  with `…` instead when it has exactly one row.

  Per-state block order (priority 1 first):

  **Running**
  1. `glyph nick`, bold
  2. current action (R6.5, see `client.md`), wraps up to 2 rows
  3. status line
  4. one line per subagent: `↳ nick action`, truncated
  5. (blank row +) wire title, dim, wraps up to 2 rows
  6. (blank row +) `files: a.rs, b.rs, …`, dim, wraps up to 2 rows (block
     omitted entirely if no files were touched this turn)
  7. **elastic** — recent-actions list: entries *before* the current action
     (never repeats block 2), oldest first, dim, with the most recent 2
     entries shown brighter. Shows the most recent `k` entries that fit,
     one entry per row, each truncated to fit.

  **Question**
  1. `glyph nick`, bold
  2. the question badge
  3. (blank row +) **elastic** — the session's final assistant text,
     wrapped. If it doesn't fit in the rows available, the **tail** is kept
     (the question is at the end of the text) and the first shown row is
     replaced with `⋯`.
  4. (blank row +) `you: <last user prompt>`, dim, wraps up to 3 rows
  5. (blank row +) wire title, dim, wraps up to 2 rows
  6. subagent lines (rare on a question tile; placed after the title)

  When the fixed blocks don't all fit, they're dropped in this order: 6,
  then 5, then 4. The elastic block (3) always keeps at least 1 row as long
  as `h ≥ 3`.

  **Needs-you (plain)**
  1. `glyph nick`, bold
  2. wire title, wraps up to 2 rows
  3. status line
  4. subagent lines
  5. (blank row +) **elastic** — final assistant text (what the session
     finished), wrapped, tail-kept with `⋯` on overflow
  6. (blank row +) `you: <last user prompt>`, dim, wraps up to 3 rows

  **Idle**
  1. `glyph nick`, dim bold
  2. wire title, dim, wraps up to 2 rows
  3. status line
  4. (blank row +) **elastic** — final assistant text, dim, wrapped,
     tail-kept

- **R5.4** — The viewport is the full terminal frame area. The grid (both
  squarify passes) is recomputed on every terminal resize event, and
  whenever a session crosses into or out of the active window `W`
  (`overview.md` R3), or switches between `running` and `needs-you` (either
  sub-state) — the two triggers that can change a project's or session's
  weight (R5.1/R5.2). Nothing else forces a recompute: a new current-action
  line, a subagent being added or removed, a `needs-you` session's wait
  clock ticking up, or the question badge toggling on/off all redraw their
  tile's own content on the next frame without moving or resizing any Rect.
  A larger terminal fits more project regions per row and more tiles per
  region, at proportionally larger sizes — it never just stretches existing
  boxes into empty space.

  **Superseded in part:** the original wording described the
  grid recomputing "on every redraw where a session's classification,
  project/tile membership, weight, or ordering has actually changed...
  consistent with there being no caching... anywhere in this layout." At
  real usage scale most redraws (every ~250ms tick) had no such change at
  all, yet nothing distinguished a truly-unchanged tick from one worth
  recomputing, so the recompute cost was paid uniformly every tick in
  practice. The trigger is now exactly the two explicit conditions above; a
  weight change alone (e.g. a subagent count changing) is deliberately no
  longer sufficient by itself to force a recompute — only the
  window-crossing and running/needs-you triggers are. Reusing the previous
  frame's geometry when neither trigger fired is now correct, cached
  behavior, not the absence-of-caching this requirement originally
  asserted; the "consistent with no caching... anywhere in this layout"
  clause is void. R5.7's own no-positional-stability guarantee is
  unaffected — this requirement only gates *when* a fresh squarify pass
  runs, never *whether* its result is allowed to differ from the last one.

  Scenario: Given the dashboard running at 100 columns wide, when the
  terminal is resized to 180 columns, then the grid is recomputed
  immediately and project regions/tiles grow and rearrange to use the new
  width — the layout is never left stretched or re-drawn with dead space
  around unchanged-size boxes.

  Scenario: Given a `running` session whose current-action text changes
  three times in a row with nothing else changing anywhere, when the
  dashboard redraws each time, then the tile's text updates every time but
  its Rect — and every other tile's Rect — stays exactly the same, since
  neither a window crossing nor a running/needs-you switch occurred.

  Scenario: Given that same session's turn then ends (`running` ->
  `needs-you`), when the dashboard redraws next, then the grid recomputes —
  this transition is one of the two named triggers, even though nothing
  about the terminal size changed.

- **R5.5** — Minimum sizes: a session tile needs at least 12x5 cells outer
  (10x3 usable after its 1-cell inset). A project region needs at least
  14x7 cells, or it collapses to a single project-summary tile (name +
  status counts) instead of showing individual session tiles. Below a
  40x12 total viewport, the dashboard shows a centered "terminal too small"
  panel instead of any layout (R9.1). No layout ever draws an unreadable
  sliver of a tile or region.

  [REVIEW: OPEN, see requirements doc] — These exact numbers came from an
  earlier 4-line card design and need rechecking now that the session tile
  content is 3 lines, not 4 (per `visuals.md` R6.3); they may be able to
  shrink. Not yet re-verified against the current build.

  Scenario: Given a project region that would size to 10x5 cells under the
  current packing, when the dashboard renders, then that region collapses
  to a project-summary tile (name + status counts) rather than drawing
  under-sized session tiles inside it.

- **R5.6** — At most 3 session tiles are shown per project at once. When
  more than 3 active sessions exist in a project, the visible 3 are chosen
  in this order: sessions needing a reply first (question badge, R6.7),
  then longest-waiting needs-you next, then running, then most recently
  updated idle. Everything beyond the visible 3 collapses into one small,
  unweighted overflow chip (`+N idle` / `+N sessions`) rather than being
  tiled. Subagent sessions (sessions whose `parentID` points at another
  session) are shown as first-class tiles with a `↳` prefix inside their
  parent's content — not as a separate third nesting level.

  [REVIEW: OPEN, see requirements doc] — The overflow policy when the
  active window is set to "show all" (`a`, see `interactions.md` R8) and 50+
  sessions are in view is unspecified: tiles may still fall below the
  minimum size (R5.5) at that scale. The per-project 3-tile cap and
  overflow chip described above are confirmed; what a *global* `+N`
  strategy looks like across all projects at once under `a` is not decided.

  Scenario: Given a project with 5 active sessions — 1 question, 2 running,
  2 needs-you at 20m and 5m waiting — when the dashboard renders that
  project, then the visible tiles are the question session, the 20m
  needs-you session, and one running session (in that priority order), and
  the remaining 2 sessions collapse into a `+2 sessions` chip.

- **R5.7** — Project region screen position is **not guaranteed stable**,
  and this is a deliberate reversal of an earlier decision, not an
  oversight. A mutation test on the real build — adding sessions, then
  adding a project — moved 3 of the 4 project regions to a different screen
  column, not just a different size, on the very next redraw. The same
  applies to on-screen reading order: it is not guaranteed to match
  first-appearance/list order either, for the same underlying reason —
  squarify preserves input *order* for how it packs regions, but that
  packing order is not the same thing as final on-screen *position*, and
  nothing in this layout tries to make it one. An earlier decision ("stable
  slots over priority reflow", 2026-09-01) is retracted: the area-proportional
  packing this layout is built on is worth more, at real usage scale
  (R5.8), than positional memory would be. Do not implement any
  compensating logic (sticky slots, position memoization, etc.) to work
  around this — it was considered and explicitly rejected.

  Scenario: Given 4 project regions currently laid out left to right in
  columns A/B/C/D, when a new session is added to project A and a new
  project E appears, then the next redraw may place what was project B in
  a different column than before (e.g. now on the right instead of the
  left) — this is expected behavior of the underlying packing algorithm,
  not a bug to fix.

- **R5.8** — Real usage is small: typically 2 sessions per project, up to 4
  projects at a time (around 8 sessions total) — not the 10-50+ session
  stress cases some earlier layout drafts were tuned against (full
  statement of this requirement is in `overview.md`; restated here because
  it directly drives the layout decisions above). Concretely, at this
  scale: the R5.6 3-tile-per-project cap and its overflow chip essentially
  never fire (a project rarely has more than 2-3 active sessions at once),
  and area-proportional packing (R5/R5.1) is what actually uses the screen —
  a layout built only for large-scale safety renders mostly blank at this
  size. Stress scale (50+ sessions) must still not break badly, but it is a
  secondary check, not what this layout is optimized for.

  Scenario: Given 4 projects with 2 sessions each open (8 sessions total),
  when the dashboard renders, then the full body area is used by
  proportionally-sized project regions and tiles, with no per-project
  overflow chips needed — this is the layout's primary design point, not
  an edge case.

- **R5.9** — *(Folded into R5/R5.1 above; kept here only for provenance.)*
  Project region area being proportional to session count, via one
  squarified-treemap call, was retired on 2026-09-01 and reinstated after a
  real build proved the alternative wrong: measured against a flow-grid/list
  layout, 3 of 4 candidate layouts used at most 13 of 40 available screen
  rows at real scale (4 projects, 8 sessions); only the area-proportional
  layout used all 40. R5/R5.1 above are the current, authoritative
  statement of this rule.

  Scenario: see R5's and R5.1's scenarios above — this entry has no
  behavior of its own to test; it exists only to record why area-proportional
  packing was reinstated after being briefly retired.

- **R5.10** — *(Folded into R5/R5.3 above; kept here only for provenance.)*
  Tile content scaling with the space actually given, as a growth ladder
  rather than a fixed line count, was confirmed against a real render where
  fixed-3-line cards inside a larger proportional box left blank space
  below the text. R5/R5.3 above are the current, authoritative statement of
  this rule.

  Scenario: see R5's and R5.3's scenarios above — this entry has no
  behavior of its own to test; it exists only to record the real-render
  evidence that justified the content-scaling rule in R5.3.

- **R5.11** — A project's accent color is applied to exactly one small
  on-screen element: the project name in its region's tag row. It is never
  applied to a whole tile's background, a tile border, or a project region
  border. (Exact color values and how a project's accent is picked are
  `visuals.md`'s concern — this rule is only about where the accent may and
  may not appear.) This revises an earlier default of a full border per
  project: six saturated project border colors were found, in testing, to
  compete with the state colors (running/needs-you/idle) that are supposed
  to carry the user's attention, and were the reason an earlier build
  looked like "nothing pops." In the current layout there are no borders at
  all — a tile's background carries its state, and a badge carries urgency.

  Scenario: Given a project assigned a purple accent color, when the
  dashboard renders that project's region, then only the project name text
  in the tag row is purple — the tiles inside it keep their normal
  state-colored backgrounds, and no border anywhere is purple.

## Degrade and edge states

- **R9** — When zero sessions are active within the current window
  (`overview.md` R3), the dashboard shows a centered empty-state panel
  instead of tiling idle sessions into tiny boxes: `No sessions updated in
  last Xm — N older sessions hidden — press ] or a` (`X` = the current
  window in minutes, `N` = count of idle sessions that exist but aren't
  shown). Idle sessions continue to appear only as context inside a project
  that has at least one active session (`overview.md` R3.2) — this panel is
  what's shown when no project qualifies for that at all.

  Scenario: Given the active window is 10 minutes and every existing
  session was last updated 25 minutes ago, when the dashboard renders, then
  the body shows the centered panel `No sessions updated in last 10m — 6
  older sessions hidden — press ] or a` instead of any project regions or
  tiles.

- **R9.1** — Below a 40x12 total viewport, the dashboard shows a centered
  `terminal too small` panel and draws nothing else — no project regions,
  no tiles, no header/footer content beyond what's needed for the panel
  itself. This is the same minimum-viewport threshold stated in R5.5.

  Scenario: Given the terminal is resized to 35x10, when the dashboard
  redraws, then the entire body shows a centered `terminal too small`
  message and no attempt is made to pack any project regions into the
  remaining space.

- **R9.2** — When there isn't enough room to show everything at full detail,
  the dashboard degrades through a fixed hierarchy, in this order: full
  session tiles (normal case) → project-summary tiles (name + status counts
  only, R5.5) → an aggregated `+N projects` chip → the terminal-too-small
  panel (R9.1) as the final fallback. The priority behind this ordering,
  most important first: readability, then project presence (every project
  stays visible in some form as long as possible), then proportionality
  (area-proportional sizing), then filling 100% of the screen. Filling
  every pixel is explicitly the *lowest* priority — the layout will leave
  space unused (e.g. tinted blank rows in a tile, R5.3) rather than violate
  a higher-priority rule to fill it.

  Scenario: Given a viewport too narrow to fit every project as a full
  region but wide enough to be above the R9.1 threshold, when the dashboard
  renders, then smaller projects degrade to project-summary tiles (or
  further, to an aggregated `+N projects` chip) before any project is
  dropped from view entirely or any text becomes unreadably small.
