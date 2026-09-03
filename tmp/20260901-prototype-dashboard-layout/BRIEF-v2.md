# Dashboard layout spike v2 — Mosaic build brief

Shared source of truth for the implementer and the reviewer. Read this file directly; do
not paraphrase it. Where this brief and `redesign-specs.md` §C disagree, this brief wins —
each divergence is deliberate and listed in "Changes from redesign-specs.md §C" at the end.

## Goal

A throwaway ratatui TUI that renders the **Mosaic** direction — area-proportional project
regions, state-coloured session tiles, tile content that grows with tile size — so the user
can judge it in a real terminal. It replaces the current spike in this directory. It is a
design-validation prototype, not production code.

The bar it must clear (user's words): "the smart/genius lies in using the space in showing
all the relevant content easily without making it cluttered." Real content in real space.
No decoration to fill gaps. Where content runs out, the tile is tinted-blank and the
report says how much of the screen that was.

## Location

`/Users/ajeesh/projects/madaboutcode/opencode-mcp/tmp/20260901-prototype-dashboard-layout/`

Keep: `src/squarify.rs` (unmodified). Rewrite everything else (`fixture.rs`, `palette.rs`,
`layout.rs`, `render.rs`, `main.rs`) against this brief. Delete the subagent
substitution/append toggle and the demotion ladder from the previous spike — Mosaic has
neither.

## Already settled (do not re-derive)

- Attention model: `running / needs-you / idle`; needs-you has two visual sub-states,
  `question` and plain (R6.7). No `stalled`.
- Nickname: deterministic adjective-noun from session id (R6.8). Fixture supplies them.
- Project region position: first-appearance order, never re-sorted by weight or status
  (R5.7/R5.9). Sizes change; reading order does not.
- Project accent colour on one small element only — the name tag (R5.11). Never a border.
- Glyphs: `?` question, `●` needs-you plain, `◐◓◑◒` running (animated), `○` idle,
  `↳` subagent, `⋯` elision, `…` truncation, `·` separator. No `⚠`.
- Real usage is ~4 projects × ~2 sessions (R5.8). Optimise for that; stress must not
  break badly.

## Geometry

### Rows

Row 0 header, row H−1 footer, body = rows 1..H−2. Whole body filled with gutter
colour `#16161e` before anything else is drawn. Below `40×12` total: centered
`terminal too small`, nothing else.

### Project regions — one squarify call, 2:1 cell space, fixed order

1. Input: projects in first-appearance order (fixture order; a project added at runtime
   goes last). **Exclude all-idle projects entirely** (R9: idle is context only). Do not
   sort.
2. Weight per project = number of top-level sessions it shows (all states). Subagents
   never count — they are content inside the parent's tile.
3. Call `squarify::squarify()` once, target rect `W × 2·Hb` where `Hb` is body height
   (rows count double so regions come out visually square). Do not modify the algorithm.
4. Snap by rounding **edges, not sizes**: `x0=round(x)`, `x1=round(x+w)`,
   `y0=round(y/2)`, `y1=round((y+h)/2)`. Adjacent regions then share edges — no gaps, no
   overlaps, no drift.
5. Inset each region 1 col on the right and 1 row on the bottom (gutter shows through).
   Fill the region with plate colour `#1a1b26`.
6. Region too small: `w < 6` or `h < 1` → not drawn, counted in the report. `h == 1` →
   tag row only. `h ≥ 2` → tag row + tiles.

Expected and accepted for this spike: at 150×42 with weights 3/2/2/1, the 1-session
project comes out as a tall ~18-col strip. That is squarify's greedy last-item
behaviour, not an ordering bug. **Do not add an aspect floor, a minimum width, or any
second area formula.** Report the worst region aspect ratio (see Report) — whether to
amend the algorithm is the user's call after seeing it.

### Project tag (row 0 of the region)

` name ` with bg = project accent, fg `#16161e`, bold; then one space; then session
count dim bold. If `name + 2 + count` exceeds region width, truncate the name with `…`
(min 3 name chars). Accent = first-appearance index mod 6 over
`#bb9af7 #9ece6a #7dcfff #ff9e64 #7aa2f7 #73daca`. Red is never a project accent.

### Session tiles — second squarify inside the region

1. Tile area = region rect minus its tag row (rows 1..h−1 of the region).
2. Order sessions `question → needs-you (longest wait first) → running → idle`, then
   squarify in that order, same 2:1 space and edge-snap rule as above.
3. Weight is **content demand, not status** (R6.1 stays in force — status is colour,
   glyph, order; never geometry):
   - idle: 1
   - running / needs-you / question: 2
   - +1 per subagent
4. Inset each tile 1 col right, 1 row bottom (plate shows through).
5. Fill the whole tile with its state background. Text is drawn over it with a 1-col
   inset on each side and no vertical inset.

| State | Tile bg | Text (nick/status) | Body text |
|---|---|---|---|
| question | `#3d2230` | `#f7768e` | `#c0caf5` |
| needs-you plain | `#3a2e1b` | `#e0af68` | `#a9b1d6` |
| running | `#1f2d52` | `#7aa2f7` | `#a9b1d6` |
| idle | `#1e2030` | `#565f89` | `#565f89` |

Dim text everywhere = `#565f89`. Subagent lines = `#7dcfff`. Question badge = fg
`#16161e` on bg `#f7768e`, bold, one space padding each side.

## Tile content ladder — a total function of (inner width `wi = w−2`, height `h`)

### Regime table

| `wi` | `h = 1` | `h = 2` | `h = 3–4` | `h ≥ 5` |
|---|---|---|---|---|
| `< 3` | colour only | colour only | colour only | colour only |
| `3–5` | glyph | glyph / age | glyph / age | glyph / age |
| `6–11` | `glyph age` | `glyph age` / nick | `glyph age` / nick / — | same as h=2 |
| `≥ 12` | `glyph nick · age` | `glyph nick` / `status · age` | **compact** | **extended** |

`nick` truncates with `…` to fit. `age` is the elapsed string (`9m`, `45s`, `2h`).

### Status line forms

- `wi ≥ 16`: `question · 9m` (as badge), `needs-you · 22m`, `running · 3m`, `idle · 51m`.
- `12 ≤ wi < 16`: badge ` ? 9m `, `need · 22m`, `run · 3m`, `idle · 51m`.
- Elapsed: needs-you = since `time.idle`; running = since turn start; idle = since
  `time.updated`. Fixture supplies the strings directly.
- If the session has subagents and the tile has no room for subagent lines, append
  ` ↳N` (cyan bold) to the status line when `≥ 4` cells remain.

### Compact (`wi ≥ 12`, `h = 3–4`)

Row 1: `glyph nick` bold in state colour. Row 2: content line, truncated with `…`
(question → first line of the assistant text; running → current action; needs-you plain
and idle → wire title). Row 3: status line. Row 4 (if `h = 4`): first subagent line
`↳ nick action` cyan, truncated — else blank.

### Extended (`wi ≥ 12`, `h ≥ 5`) — priority blocks with one elastic block

Blocks are laid top-to-bottom in the order given. A blank rhythm row precedes a block
only if that block actually renders. Fixed blocks are placed first in priority order
(1 = never dropped); whatever rows remain go to the single **elastic** block. If fixed
blocks alone exceed `h`, drop from the bottom of the priority list until they fit. Rows
left after the elastic block is exhausted stay tinted-blank — no filler, no decoration.

Wrapping: word-wrap at `wi`; a word longer than `wi` is hard-broken. Wrap whenever a
block has more than one row available; truncate with `…` only when it has exactly one.

**Running**
1. `◐ nick` (bold)
2. current action (R6.5 format), wrap ≤2 rows
3. status line
4. subagent lines, one per subagent, `↳ nick action`, truncated
5. blank + wire title, dim, wrap ≤2 rows
6. blank + `files: a.rs, b.rs, …` dim, wrap ≤2 rows (files touched this turn; omit block
   if empty)
7. **elastic** — recent actions: entries *before* the current action (never repeat row 2),
   oldest first, dim; the last 2 entries in `#a9b1d6`. Fill top-down from the oldest that
   fits, i.e. show the most recent `k` where `k` = rows available. Each entry one row,
   truncated.

**Question**
1. `? nick` (bold)
2. badge
3. blank + **elastic** — final assistant text, wrapped. If it exceeds the rows available,
   keep the **tail** (the question is at the end) and replace the first shown row with
   `⋯`.
4. blank + `you: <last user prompt>` dim, wrap ≤3 rows
5. blank + wire title dim, wrap ≤2 rows
6. subagent lines (rare on a question; after title)

Priority for dropping when fixed blocks don't fit: 6, 5, 4 — the elastic block always
keeps ≥1 row if `h ≥ 3`.

**Needs-you plain**
1. `● nick` (bold)
2. wire title, wrap ≤2 rows
3. status line
4. subagent lines
5. blank + **elastic** — final assistant text (what it finished), wrapped, tail-kept with
   `⋯` on overflow
6. blank + `you: <last user prompt>` dim, wrap ≤3 rows

**Idle**
1. `○ nick` (dim bold)
2. wire title dim, wrap ≤2 rows
3. status line
4. blank + **elastic** — final assistant text dim, wrapped, tail-kept

## Header and footer

- Header (row 0, bg `#16161e`): ` ◆ opencode ` bold blue · `4 projects · 8 sessions`
  dim · `? 2` red bold · `● 1` yellow bold · `◐ 4` blue bold (spinning) · `○ 1` dim bold ·
  right-aligned `window 10m  ● live` (`●` `#9ece6a`). Below 100 cols drop the project
  count; right side becomes `10m ● live`. **Counts are on-screen sessions only.**
- Footer (row H−1, bg `#16161e`): left = spike keys (below), dim. Right = hidden all-idle
  projects if any: `hidden: docs-site (4 idle)` dim, truncated to fit.
- Spinner advances one frame per 250 ms tick. The only motion.

## Fixture

Session fields: `nick`, `title`, `state` (`q`/`need`/`run`/`idle`), `age`, `wait_m`
(needs-you sort key), `action` (current, running only), `subs: [(nick, action)]`,
`recent: [action]` (oldest → newest, excluding current), `files: [path]`,
`assistant_text` (multi-line), `user_prompt`.

Accent index = project first-appearance index mod 6.

### REAL (default) — 4 projects, 8 sessions, 3/2/2/1

**web-dashboard**
- `brave-otter` · q · 9m · wait 9 · title `Add multiple code titles with edit support` ·
  user_prompt `Clean up the old title components and remove whatever's unused` ·
  assistant_text:
  ```
  I found three candidates for removal under src/components/titles/:

  1. LegacyTitle.tsx — no imports anywhere
  2. TitleEditor.old.tsx — imported only by the storybook story
  3. title-utils.ts — 2 helpers still used by TitleBar.tsx

  Deleting 3 would break TitleBar. Which file would you like me to delete?
  ```
  recent: `grep: LegacyTitle`, `read: src/components/titles/LegacyTitle.tsx`,
  `grep: TitleEditor`, `read: src/components/titles/TitleEditor.old.tsx`,
  `grep: title-utils`, `read: src/components/TitleBar.tsx`
- `amber-falcon` · run · 3m · title `Reviewing opencode dashboard requirements doc for
  session card layout` · action `editing: requirements.md` · subs `cinder-wisp`
  `editing: render.rs` · files `requirements.md`, `render.rs`, `layout-brainstorm.md` ·
  user_prompt `Fold the brainstorm learnings back into the requirements doc` · recent (10):
  `read: tasks/2026-09-01-opencode-dashboard.requirements.md`, `grep: R5.2`,
  `read: src/render.rs`, `shell: cargo build`, `editing: render.rs`,
  `shell: cargo test -p dashboard`, `read: tasks/2026-09-01-opencode-dashboard.layout-brainstorm.md`,
  `editing: layout-brainstorm.md`, `shell: git diff --stat`, `read: BRIEF.md`
- `extraordinarily-verbose-nickname-case` · run · 11m · title `Investigate why the CI
  pipeline intermittently fails on the integration test suite for the payments service
  when run under load on Tuesdays` · action `running: cargo test --package
  payments-integration` · files `payments/tests/integration.rs`, `ci/pipeline.yml` ·
  user_prompt `The payments integration suite flakes under load on Tuesdays. Find out why.`
  · recent (14): `read: ci/pipeline.yml`, `grep: test-threads`,
  `shell: gh run list --workflow ci.yml --limit 50 --json conclusion,createdAt`,
  `read: payments/tests/integration.rs`,
  `shell: cargo test --package payments-integration -- --test-threads=1 integration::tuesday_load_profile`,
  `grep: sleep_until`, `read: payments/src/clock.rs`, `editing: payments/src/clock.rs`,
  `shell: cargo test --package payments-integration -- --test-threads=8`, `read: ci/cache.yml`,
  `shell: cargo test --package payments-integration -- --test-threads=8 --nocapture 2>&1 | tail -80`,
  `grep: RUST_TEST_THREADS`, `editing: ci/pipeline.yml`, `shell: cargo build --release`

**infra-tools**
- `golden-hawk` · q · 6m · wait 6 · title `Apply terraform destroy for staging, are you
  sure?` · user_prompt `tear down staging, we're rebuilding it from the new module` ·
  assistant_text:
  ```
  terraform plan -destroy reports 12 resources in staging:

    aws_rds_instance.staging        (db.t3.medium, 40 GB)
    aws_ecs_service.api             ×3
    aws_lb.staging
    aws_s3_bucket.staging-assets    (2.1 GB, versioning off)
    aws_iam_role.staging-task       ×5

  The S3 bucket is not backed up. Confirm destroy of 12 resources in staging?
  ```
  recent: `shell: terraform init`, `shell: terraform workspace select staging`,
  `shell: terraform plan -destroy -out=destroy.tfplan`, `read: destroy.tfplan`
- `hawk-otter` · idle · 51m · title `Rewrite deploy script` · user_prompt `rewrite the
  deploy script in python with a dry run flag` · assistant_text `Rewrote deploy.sh as
  deploy/run.py with --dry-run and per-service targets. All 6 services deploy cleanly
  against the staging cluster.`

**mobile-app**
- `sable-heron` · run · 6m · title `Fix push notification token refresh` · action
  `editing: NotificationManager.swift` · subs `pebble-owl` `editing: TokenStore.swift`,
  `misty-vole` `running: xcodebuild test`, `ashen-crane` `reviewing: PushDelegate.swift` ·
  files `NotificationManager.swift`, `TokenStore.swift`, `PushDelegate.swift`,
  `AppDelegate.swift` · user_prompt `Push tokens go stale after the app is backgrounded
  for a day; fix the refresh path` · recent (7): `grep: didRegisterForRemoteNotifications`,
  `read: AppDelegate.swift`, `read: NotificationManager.swift`,
  `editing: AppDelegate.swift`, `shell: xcodebuild -scheme App test -only-testing:PushTests`,
  `read: TokenStore.swift`, `subagent: 3 started`
- `quartz-badger` · need · 1m · wait 1 · title `Update app icon assets` · user_prompt
  `update the app icon from the new 1024 png in Design/` · assistant_text:
  ```
  Regenerated all 18 icon sizes from Design/Icon-1024.png and updated
  Assets.xcassets/AppIcon.appiconset/Contents.json. Removed the two deprecated
  iPad Pro entries. Xcode build succeeds with no warnings.

  Done.
  ```

**scratch-cli**
- `violet-otter` · run · 45s · title `Prototype a faster arg parser` · action
  `shell: cargo bench` · files `src/args.rs`, `benches/parse.rs` · user_prompt `try a
  hand-rolled parser and bench it against clap` · recent (5): `read: src/main.rs`,
  `write: src/args.rs`, `write: benches/parse.rs`, `shell: cargo build --release`,
  `shell: cargo bench -- --warm-up-time 1`

### STRESS (toggle) — 8 projects, 54 sessions

Exactly `redesign-specs.md` §0.5.2 (including the `quartz-badger` nickname collision,
the all-idle `docs-site`, and the generated 22-session `ci-fleet-runner`). Fill the new
fields generically: `recent` = 6 entries cycling `read: x.rs` / `shell: cargo test` /
`editing: y.rs`; `assistant_text` = title + ` — done.`; `user_prompt` = empty; `files`
= empty. `docs-site` must vanish from the treemap and appear in the footer as hidden.

## Runtime

- Alt screen + raw mode, panic-safe restore. Poll 250 ms, `KeyEventKind::Press` only.
- Layout recomputed every frame from the fixture (no caching, no stability logic).
- Keys:
  - `q` / `Esc` quit
  - `w` toggle simulated 80-col width vs real width
  - `f` toggle REAL / STRESS fixture
  - `+` append a synthetic running session (`new-N`, action `shell: sleep 1`, 3 recent
    entries) to the **last** project; `-` remove the last session of the last project
    (never below 1)
  - `p` append a new project `late-arrival` with one running session
  - `.` append one action (cycle `read: a.rs`, `shell: cargo check`, `editing: b.rs`)
    to every running session's `recent` — demonstrates the list scrolling
- Footer left: `q quit  w 80col  f fixture  +/- session  p project  . tick` dim.
- `--dump` CLI flag: no TTY. Render REAL and STRESS at `150×42` and `80×36` via
  ratatui `TestBackend`, write plain-text frames to `renders/<fixture>-<W>x<H>.txt`, print
  the report metrics (below) to stdout, exit.

## Report — evidence of done

`cargo build` clean, no warnings. Then, for each of the four `--dump` renders:

1. Body cells classified: **gutter/plate** (no tile), **tile-blank** (tile bg, space),
   **tile-text** (tile bg, non-space). Print counts and percentages. This is the number
   that answers the user's whitespace critique; report it, don't editorialise it.
2. Each project region as `name: x,y w×h weight aspect` where aspect = `max(w, 2h) /
   min(w, 2h)`. Flag the worst.
3. Any tile that landed in the `< 12` width regimes, and any region/tile dropped as too
   small.
4. For REAL 150×42 specifically: which extended blocks rendered per tile, and how many
   rows each tile left tinted-blank.

Plus, from an interactive run: after `+` `+` `p` on REAL at real width, state in words
which regions moved column and which only resized (R5.7 evidence).

Anything ambiguous in this brief: say what you chose and why. Anything that looks wrong
when following the brief (the 1-session sliver, a region that flips column, text that
still can't fill a tile): report it as a finding. Do not silently fix it.

## Out of scope for this spike

Selection highlight, `j/k` navigation, zoom, live data, question heuristic, window
controls, nickname hashing. All fixture-driven.

## Changes from `redesign-specs.md` §C, and why

| §C said | This brief says | Why |
|---|---|---|
| Session tile weight = status (4/3/2/1) | Weight = content demand (idle 1, active 2, +1/subagent) | §C:182 admits it contradicts R6.1; in `C-real-wide.png` the 1-line question tile got 2× the area of the 5-line running tile |
| Ladder had unspecified (w,h) cells; recent-actions order contradicted subagent/title order; `h − 6` | Total regime table; per-state priority blocks with one elastic block | A coder would otherwise decide these ad hoc |
| Ring buffer 8 | No cap in fixture; production note: buffer ≥32 | 8 entries cannot fill an 18-row tile |
| Assistant text "up to tile height" | Tail-kept, `⋯` first row | The question is at the end |
| Needs-you plain / idle: title then blank | Title, then final assistant text, then user prompt | "What did it finish" is the judgment-call content; blank was the user's complaint |
| No user-prompt or files-touched blocks | Added (production: same message fetch as R6.7; files from `edit`/`write`/`patch` inputs this turn) | Real content that exists and fills space |
| "Aspect penalty negligible" | Expected sliver named, reported as a number, not fixed | The screenshot contradicts the claim; the fix is a user decision on R5.9 |
| "Positions hold" | Mutation keys + written R5.7 evidence | Never tested under count change |
| Below 14×7: tag only | `h == 1` tag+count; `h ≥ 2` tiles | R5.5 wants counts present; tag carries the count already |
| Evidence = build clean | Text dumps + coverage metric + aspect table | The thesis is "space is used"; measure it |

## Borrowed from concept review (2026-09-02)

Four ChatGPT concept images (full-width project bands, flow-grid cards — not our
direction, and not provably monospace-drawable) were reviewed for ideas only. Three
things carry over into the Mosaic layout without touching its foundation. Where this
section conflicts with a rule above, this section wins.

### B1. Per-project state counts on the tag row

The tag row currently holds ` name N` and is blank to the right. Fill it: after the
count, right-aligned within the region width, the project's state glyph counts in state
colours, zeros omitted: `? 1  ● 1  ◐ 2  ○ 1`. Below `wi = 24` drop the right-aligned
group entirely (the tag + count stays). This is also what an `h == 1` region shows, which
satisfies R5.5's "name + status counts" directly.

### B2. Idle sessions as one chip row, outside squarify

Replaces the idle rules in "Session tiles" steps 2–3. Idle sessions are **not** tiled.
They render as a single row of chips on the **last row** of the region, left to right in
most-recent-first order: `○ nick · 51m`, chips separated by two spaces, each chip in idle
colours on the plate (no tile bg). When the next chip does not fit, stop and print
`+N idle` dim in place of the remainder (always fits: reserve 8 cells for it before
placing chips). Squarify then runs over **active sessions only** (running / needs-you /
question) in the region rect minus the tag row minus the idle row. Weights unchanged
(active 2, +1 per subagent).

Thresholds: region `h ≥ 3` → tag + tiles + idle row. `h == 2` → tag + tiles, idle row
dropped and idle count folded into the tag (`○ 1` still shows via B1). `h == 1` → tag
only. If a project has active sessions but the tile area after reserving the idle row is
`< 1` row, drop the idle row first.

Why: in `C-real-wide.png` the single idle session in `infra-tools` is a 24×36 tinted
sliver next to the question tile — the largest blank block on that screen after the
question tile itself. Idle is context (R9); one chip row carries name + age, gives the
area back to the session that has content, and is the same `h = 1` ladder form already
defined. Cost: the idle extended-regime block (assistant text) never renders. Accepted —
that text is reachable by zoom later, and the user's complaint was blank area, not
missing idle detail.

### B3. Tool-call count on the running status line

Running status line gains a third segment when `wi ≥ 24`: `running · 11m · 14 calls`.
The number is tool-call pairs this turn — the same SSE `tool.input.started`→`tool.called`
tracking R6.4 already does, so it is a real field, not a decoration. It answers "is this
working or stuck" faster than the spinner does. Fixture: add `calls` to running sessions —
`amber-falcon` 11, `extraordinarily-verbose-nickname-case` 15, `sable-heron` 8,
`violet-otter` 6; STRESS running sessions all 7.

### Rejected, and why

- **Subagents as a per-project subsection** (image 4's `subagents ──── 3 live`): it
  drops the parent link — `cargo-test` / `cargo-watch` in the image belong to nobody
  visible — and in an area layout it would need region area, contradicting "subagents
  never count toward weight". The nested `↳` lines stay.
- **Sparklines on running cards**: same information as the recent-actions list (activity
  over time), less precise, needs a new per-minute counter, and every bar in the images
  is invented. If the actions list ever has to go for space, a sparkline is the compact
  substitute — not now.
- **Key/value metric lines** (`current: 36  target: 40  queued: 18`): the wire has no such
  fields. B3 is the one honest number available; the rest would be fabricated.
- **State-coloured borders on needs-you cards**: Mosaic has no borders; the tile
  background already carries state and the badge carries urgency. Adding a border would
  re-introduce the chrome cost that killed cards at 80 cols.

### Report additions

For each REAL render, list the idle row contents per project and whether `+N idle` fired.
Add B1's right-aligned group to the tag row check in report item 3.
