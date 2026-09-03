# opencode dashboard — redesign directions (build specs)

Scope: visual/UX rethink of the two-level dashboard (projects → sessions). Four genuinely different directions, each specified so a builder can implement it without further creative calls. Evidence renders of all four already exist in `redesign-options.html` (same dir, pills A–D, `150×42` / `80×36` toggle; screenshots in `shots/`). That file is a JS character-grid simulation — every effect in it is cell-based, so anything shown there is drawable by ratatui.

Hard constraints honoured by all four: ratatui + crossterm, monospace grid, 24-bit ANSI colour, Unicode box/block glyphs only, no mouse, R9.2 degrade order (readability > project presence > proportionality > 100% fill), attention model `running / needs-you(question | plain) / idle` (R6.7), nickname rules (R6.8).

---

## 0. Shared decisions (apply to every direction)

### 0.1 Typography
- Font: **JetBrains Mono** (the user's terminal font; the dashboard doesn't choose fonts, it assumes this one). Bold is the only weight used. No italic (uneven terminal support), no underline.
- Glyphs, all single-cell-width in JetBrains Mono, verified in the HTML render: `? ● ○ ◐◓◑◒ ▇ █ ▍ ▎ ▁▂▃▄▅▆▇█ ↳ · … › ⋯ ◆ ╭╮╰╯│─`. **Do not use `⚠`** (the current `palette.rs` question glyph) — it is emoji-presentation/ambiguous-width in many terminals and breaks the grid. Question glyph is `?`.
- Running glyph animates `◐ ◓ ◑ ◒`, one frame per 250 ms tick (the R2 poll interval). This is the only motion; it is what makes a static frame read as "alive".

### 0.2 Palette — what carries over from `src/palette.rs`, what changes

| Role | Hex | vs. `palette.rs` |
|---|---|---|
| Background | `#1a1b26` | keep (`BACKGROUND`) |
| Chrome background (header/footer rows, gutters) | `#16161e` | **new** — one shade darker than bg; used for the header/footer band and for tile gutters in C |
| Rule / separator lines | `#292e42` | **new** (Tokyo Night `bg_highlight`); replaces coloured borders as the grouping line |
| Border (where a border exists at all) | `#3b4261` | **new**; only D's zone rules and B's column divider use it |
| Text primary | `#c0caf5` | keep (`TEXT_PRIMARY`) |
| Text secondary (action lines, titles) | `#a9b1d6` | **new** — one step above dim, so action text is readable but subordinate to the nickname |
| Text dim | `#565f89` | keep (`TEXT_DIM`) |
| State: running | `#7aa2f7` | keep (`STATUS_RUNNING`) |
| State: needs-you · question | `#f7768e` | keep (`STATUS_NEEDS_YOU_QUESTION`) |
| State: needs-you · plain | `#e0af68` | keep (`STATUS_NEEDS_YOU_PLAIN`) |
| State: idle | `#565f89` | keep (`STATUS_IDLE`) |
| Subagent | `#7dcfff` (cyan) | **new** — subagent lines/tags get their own hue so `↳` content is never confused with the parent's action |
| Live indicator | `#9ece6a` | **new**, header only |
| Project accents (6, rotating) | `#bb9af7 #9ece6a #7dcfff #ff9e64 #7aa2f7 #73daca` | keep the first five from `PROJECT_COLORS`; **replace red `#f7768e` with teal `#73daca`** — red is reserved for "question" and must not be a project identity colour |

**The single most important palette change:** project accent colour is applied to **one small element per project only** (a `▍` rail + the project name, or a name tag). It is **never** applied to a whole border. In the current build six saturated borders compete with the state colours and nothing pops. In every direction below, saturated colour on a large surface means exactly one thing: state.

### 0.3 Header and footer (identical in all four)
- Row 0, bg `#16161e`: ` ◆ opencode ` bold blue · `8 projects · 54 sessions` dim · `? 5` red bold · `● 9` yellow bold · `◐ 25` blue bold (spinning) · `○ 15` dim bold · right-aligned `window 10m   ● live` (● green). Below 100 cols: drop the project count, right side becomes `10m ● live`.
- Last row, bg `#16161e`: ` j/k move   ↵ zoom   [ ] window ±5m   a all   ? help` dim.

### 0.4 Card and line grammar (reused by A, B, D)
- **Status text**: `question · 9m` / `needs-you · 22m` / `running · 3m` / `idle · 12m`. Always glyph + this string. Elapsed for needs-you is time since `time.idle`; for running it is time since turn start; for idle time since `time.updated`.
- **Content text** by state: question → the question sentence; running → action line (R6.5 format); needs-you plain and idle → wire title.
- **Question badge**: the status text for a question session is rendered inverse — fg `#16161e` on bg `#f7768e`, bold, padded one space each side: ` ? question · 9m `. This is the loudest thing a terminal can draw without motion and is reserved for questions.
- **Rail card (3 lines, borderless)** — a `▎` in the state colour down the left edge, then:
  1. nickname bold in state colour (truncate at 16 chars with `…`) · ` ` · tail text dim (wire title, or project name in the project's accent when the card is outside its project)
  2. content text (`#c0caf5` for questions, `#a9b1d6` otherwise)
  3. glyph + status text in state colour (badge form for questions); if subagents exist and ≥7 cells remain: ` ↳N ` cyan bold + last subagent's action, cyan
- **Ledger line (1 line)** — `  ` glyph ` ` nickname padded to 14 (bold, state colour; idle is dim not bold) ` ` content text (width = remaining − 6) ` ` elapsed right-aligned in 4 cells (red bold for questions, dim otherwise). Subagent child line: 6-cell indent, `↳` cyan bold, ` nickname  action` cyan.
- **Truncation** is always tail-`…`, never mid-string.

### 0.5 Mock content
Fields: `n` nickname, `t` wire title, `s` state (`q` question / `need` / `run` / `idle`), `e` elapsed, `a` action or question, `w` waiting-minutes (needs-you only, sort key), `sub` subagents.

**Two fixtures. The REAL one is what to optimise for; STRESS is a "must not break badly" check only.** Real usage (per the user) is ~4 projects, ~2 sessions each, ~8 sessions total.

#### 0.5.1 REAL fixture (primary) — 4 projects, 8 sessions (3/2/2/1)
1. **web-dashboard** (accent 0 purple, pulse `2 3 5 7 6 3 1 2 5 7`) — `amber-falcon` run 3m `editing: requirements.md`, sub `cinder-wisp` `editing: render.rs` · `brave-otter` q 9m "Which file would you like me to delete?" (title "Add multiple code titles with edit support") · `extraordinarily-verbose-nickname-case` run 11m `running: cargo test --package payments-integration` (title: the 140-char "Investigate why the CI pipeline intermittently fails…on Tuesdays")
2. **infra-tools** (green, pulse `1 1 2 4 3 2 6 7 4 2`) — `golden-hawk` q 6m "Confirm destroy of 12 resources in staging?" (title "Apply terraform destroy for staging, are you sure?") · `hawk-otter` "Rewrite deploy script" idle 51m
3. **mobile-app** (blue, pulse `4 5 3 2 6 7 7 5 3 4`) — `sable-heron` run 6m `editing: NotificationManager.swift`, subs ×3 `pebble-owl` `editing: TokenStore.swift` / `misty-vole` `running: xcodebuild test` / `ashen-crane` `reviewing: PushDelegate.swift` · `quartz-badger` "Update app icon assets" need 1m
4. **scratch-cli** (cyan, pulse `0 0 0 0 0 1 3 6 7 7`) — `violet-otter` "Prototype a faster arg parser" run 45s `shell: cargo bench`

Totals: 2 question, 1 needs-you, 4 running, 1 idle. Covers: question vs plain needs-you, 1 and 3 subagents, adversarial title + nickname, an idle-as-context session, a 1-session project.

#### 0.5.2 STRESS fixture (secondary; mirrors `src/fixture.rs`, 8 projects, 54 sessions)

1. **web-dashboard** (accent 0 purple, pulse `2 3 5 7 6 3 1 2 5 7`)
   - `amber-falcon` · "Reviewing opencode dashboard requirements doc for session card layout" · run 3m · `editing: requirements.md` · sub: `cinder-wisp` `editing: render.rs`
   - `brave-otter` · "Add multiple code titles with edit support" · q 9m · "Which file would you like me to delete?"
   - `extraordinarily-verbose-nickname-case` · "Investigate why the CI pipeline intermittently fails on the integration test suite for the payments service when run under load on Tuesdays" (adversarial 140-char title + 37-char nickname) · run 11m · `running: cargo test --package payments-integration`
   - `cobalt-wren` · "Resolve dashboard branch merge conflict" · need 22m
   - `dusty-lynx` · "Update footer legend copy" · idle 12m
2. **infra-tools** (green, pulse `1 1 2 4 3 2 6 7 4 2`)
   - `silver-marlin` · "Plan terraform infra changes" · run 1m · `running: terraform plan`
   - `ember-quail` · "Rotate staging API keys" · need 4m
   - `golden-hawk` · "Apply terraform destroy for staging, are you sure?" · q 6m · "Confirm destroy of 12 resources in staging?"
   - `hawk-otter` · "Rewrite deploy script" · idle 51m
3. **scratch-cli** (cyan, pulse `0 0 0 0 0 1 3 6 7 7`) — `violet-otter` · "Prototype a faster arg parser" · run 45s · `shell: cargo bench`
4. **docs-site** (orange, pulse all 0) — all idle: `quartz-badger` "Fix broken anchor links" 2h · `copper-hawk` "Update install instructions" 40m · `marble-finch` "Regenerate API reference" 3h · `linen-otter` "Proofread changelog" 18m
5. **mobile-app** (blue, pulse `4 5 3 2 6 7 7 5 3 4`)
   - `sable-heron` · "Fix push notification token refresh" · run 6m · `editing: NotificationManager.swift` · sub ×3: `pebble-owl` `editing: TokenStore.swift`, `misty-vole` `running: xcodebuild test`, `ashen-crane` `reviewing: PushDelegate.swift`
   - `quartz-badger` (deliberate nickname collision with docs-site) · "Update app icon assets" · need 1m
   - `willow-stag` · "Bump minimum iOS deployment target" · q 3m · "OK to drop iOS 15 support?"
   - `flint-osprey` · "Wire up crash reporting SDK" · run 2m · `running: pod install`
   - `brass-heron` "Localize onboarding screens" idle 35m · `cider-fox` "Audit accessibility labels" idle 1h
6. **big-monorepo** (teal, pulse `6 7 7 6 7 5 6 7 7 6`) — 11 sessions: `north-tiger` run 1m `running: bazel build //...` · `east-lynx` run 2m `running: pnpm test:e2e` · `south-heron` run 4m `editing: .eslintrc.js` · `west-badger` run 5m `running: tsc --noEmit` · `cinder-wolf` q 7m "Which service owns the billing table?" · `moss-otter` "Fix CI cache key" need 9m, sub `quiet-lark` `running: cache-key-diff.sh` (needs-you WITH a subagent) · `plum-heron` run 3m `editing: webhooks.test.ts` · `amber-stag` "Clean up dead feature flags" need 16m · `teal-mole` run 8m `shell: kubectl get pods -w` · `rust-crane` "Archive old migrations" idle 25m · `frost-wren` "Tidy up README" idle 50m
7. **ci-fleet-runner** (purple again — tests accent reuse; pulse `7 7 6 7 7 7 6 7 7 7`) — 22 sessions generated: nickname `runner-{alpha,beta,gamma,delta,epsilon,zeta,eta,theta}[i mod 8]`; `i mod 7 == 0` → need `(5+i)m` "Investigate flaky runner shard"; else `i mod 11 == 0` → q "Scale runner pool to 40?"; else `i mod 5 == 0` → idle 30m "Retired shard cleanup"; else run 1m `running: fleet-health-check.sh`. Result: 1 q, 4 need, 13 run, 4 idle.
8. **tiny-service** (cyan) — `ivory-crane` · "Bump lockfile" · idle 5m

Totals: 5 question, 9 needs-you, 25 running, 15 idle. `pulse` = tool-events per minute over the last 10 min (0–7), used only by the sparklines in B and D.

Session sort order everywhere: question → needs-you (longest `w` first) → running → idle.

---

## A · Swimlanes — project rows × state columns

**Thesis.** Pick this when the first question is always "what needs me?" but the user still thinks in projects. Rows are projects, columns are states, so urgency is one vertical scan down the left column and busyness is row height.

**Is NOT:** a kanban where cards move between columns on state change (they do move column, but rows never reorder); not a per-project box with its own internal layout; not a grid of bordered cards.

### Layout (numbers for 150×42; 80-col variant in brackets)
- Row 1: column headers. Gutter `PROJECT` dim bold; at the needs-you column x `? NEEDS YOU` red bold + total count dim; at running x `◐ RUNNING` blue bold (spinning) + count; at idle x `○ IDLE` dim bold + count. Row 2: full-width rule `─` in `#3b4261`.
- Columns: gutter **18** cells [14]; idle column **22** [0 — folded, see below]; remaining width split needs-you : running = **44% : 56%** (150 → needs 48, running 62).
- Card slots: minimum card width 24; cards per column = `floor(colWidth / 24)`; card width = `floor(colWidth / perRow)`. At 150 that is 2 needs-you cards of 24 and 2 running cards of 31 per row. Cards are **rail cards** (§0.4), 3 rows tall, no vertical gap between rows within a band.
- Each project is a **band**. Gutter shows: line 0 `▍` + name, both in the project accent, bold; line 1 composition bar — one `▇` per session, sorted, each in its state colour (clip at gutter width−2 with `›`); line 2 `N sessions` dim [narrow: `N · k idle`].
- Band height = max(needs-you cell, running cell, idle cell, 3). Idle cell = one **ledger-style line** per idle session (`○ nickname 12m`, dim), capped at band height − 1 with `+N idle`.
- All-idle project → 2-row band: name + bar in gutter, and `all idle · 4 sessions · last 18m` dim in the needs-you column.
- Bands separated by a 1-row `─` rule in `#292e42`. Bands are in **fixed fixture order** (R5.7): they never reorder; only their heights change.
- **Per-band cap growth**: start every band at 1 card-row per cell. While the total height fits, grow the band with the largest needs-you overflow by one card-row; repeat until nothing more fits. Overflow in a cell prints `+N more waiting` (yellow) / `+N more running` (blue) on the row below the last card. This is what makes the render use the whole screen instead of leaving the bottom third blank (compare the first vs. second A screenshot in `shots/`).
- Projects that don't fit at all: one dim line `⋯ +N projects: name, name` on the row above the footer.

### Weak points
- *Minimal-card truncation*: there is no minimal card. Idle sessions are one ledger line (`○ nick 12m`) — glyph, colour, name, age always present. Needs-you/running are never shrunk below 3 lines; they overflow to a counted line instead.
- *Cliff*: ladder is card-rows → fewer card-rows → `+N more` line → 2-row all-idle band → `⋯ +N projects`. The composition bar survives every step.
- *Bigger = more sessions*: row height grows with count **and** the bar under the name is one block per session. Lengths are comparable at a glance (5 vs 4 blocks); area is not.

### Known cost
Question text on a 24-wide card truncates at ~22 chars ("Which file would you…"). The badge says *that* it's a question; you zoom to read it.

---

## B · Ledger — dense two-column list, terminal-native

**Thesis.** Pick this when you want maximum density and a terminal-native feel: one line per session, a project header that carries the composition bar and an activity sparkline, two newspaper columns. Reads like htop/k9s. The one where every session at every width is a complete sentence.

**Is NOT:** a treemap or boxes of any kind; not cards; not state-first (projects stay grouped). It is deliberately a list.

### Layout
- Two columns [one below 100 cols], gap 3, column width `floor((W − 2 − gap) / 2)` = 72 at 150 [77 at 80]. A `│` in `#292e42` runs full height between columns.
- **Project header line**: `▍` accent · name accent bold · one `▇` per session (sorted, state colours; clip at 24 with `›`) · count bold `#c0caf5`. Right-aligned on the same line: `2 need · 2 run · 1 idle` dim, then a **10-cell sparkline** `▁▂▃▅▇…` from `pulse` (blue if the project has running sessions, dim otherwise). If the right part doesn't fit the column, drop it.
- **Session lines** (§0.4 ledger line), sorted; subagents as indented `↳` child lines directly under their parent.
- **Per-project cap** (lines after the header): needs-you sessions always shown; running sessions added while budget remains; idle shown individually only if ≤2 and budget allows, otherwise collapsed. Overflow is one dim line `⋯ +9 running · +4 idle`.
- **Cap search / ladder**: try cap = 24 down to 4 with masonry placement (next project goes to the shorter column; fixture order preserved). If nothing fits at cap 4, convert the tallest project to **header-only** (its header line still carries bar, count, status counts, sparkline) and retry from cap 24. Repeat. If still nothing fits, hidden projects are listed on the last row: `⋯ +N projects: a, b`.
- Projects separated by 1 blank row with a `─` rule in `#292e42`.

### Weak points
- *Minimal-card truncation*: solved by construction — the minimal form IS the full form (one line: glyph · nick · text · age). Idle lines are dim but complete.
- *Cliff*: full list → shorter list with `⋯ +N` → header-only (still bar + counts + sparkline) → `⋯ +N projects`. Every rung keeps project presence and a busyness signal.
- *Bigger = more sessions*: the header bar (one block per session) and the visible line count. 22 vs 11 vs 5 is countable in the header row before reading anything.

### Known costs
- A 22-session project with a high cap eats a column (visible in the wide render — ci-fleet takes 22 rows). Recommend a hard per-project cap of 12 in production and rely on the `⋯` line.
- No 2D "shape" feeling at all.
- Sparkline needs a client-side per-minute counter of SSE tool events per session/project (cheap; new state).

---

## C · Mosaic — space-filling colour-field tiles (WinDirStat, literally)

**Thesis.** Pick this when you want the WinDirStat gestalt for real: every cell belongs to something, project area ∝ session count, every session is a tile whose **background colour is its state**, readable from across the room. This is the wallboard.

**Is NOT:** bordered boxes with text cards inside (no borders exist); not a text-first view — text is a bonus that appears when a tile is big enough.

### Layout
- Whole body (rows 1..H−2) is filled with `#16161e` (gutter colour).
- **Project rects**: one squarified treemap call (Bruls) over all projects, weight = session count. Squarify in a space where **row height counts as 2 cells** (cells are ~1:2), then snap to integers; this is what makes tiles come out visually square. **Do NOT sort by weight before squarify** — feed projects in first-appearance order and keep that order for the life of the process (R5.7). With ≤8 projects the aspect-ratio penalty is negligible (checked: `shots/C-real-wide.png` and `shots/C-wide.png` both use fixed order) and a project's region stays in the same corner of the screen; only its size changes. Each project rect is inset 1 col right and 1 row bottom (gap). Fill the rect with `#1a1b26` — a slightly lighter plate on the dark gutter.
- Project **tag** at the rect's top-left: ` name ` with bg = project accent, fg `#16161e`, bold; then session count dim bold. No border, no title bar.
- **Session tiles**: second squarify inside the rect below the tag (weight-sorted is fine here — tiles within a project may reorder); weight = `question 4 · needs-you 3 · running 2 · idle 1`, `+0.5` per subagent. Each tile inset 1/1 for gap. Tile fill: question **`#3d2230`** (dark rose) with fg `#f7768e` — *not* solid red: at real scale two question tiles cover ~45% of the screen and solid red was overwhelming (compare the first vs. current `shots/C-real-wide.png`); the loud element is the inverse badge ` ? question · 9m ` (fg `#16161e` on `#f7768e`) on the status line, same as every other direction. Needs-you `#3a2e1b` with fg `#e0af68`; running `#1f2d52` with fg `#7aa2f7`; idle `#1e2030` with fg `#565f89`. Body text `#a9b1d6` (questions: `#c0caf5`).
- **Tile text ladder** by inner size (w−2, h):
  - h ≥ 3 and w ≥ 14: `glyph nick` bold / content / `status · age` (badge form for questions); if taller: subagent lines (`↳ nick action`, cyan), then a blank rhythm row, then the wire title dim.
  - h = 2, w ≥ 10: `glyph nick` / `status · age`
  - h = 1, w ≥ 12: `glyph nick · age`
  - h = 1, w ≥ 6: `glyph age`
  - w < 3: colour fill only.
- **Content scales with area (required at real scale).** At 4×2 a tile is ~55×18 cells; three lines of text leave the body blank (visible in `shots/C-real-wide.png` — that blank is C's one remaining weakness). Rules for tall tiles:
  - Never truncate when there is vertical room: **wrap** the question / action / title across lines (word-wrap at tile width) before falling back to `…`.
  - Running tiles: below the status line, show a **recent-actions list** — the session's last `h − 6` tool calls, oldest first, dim `#565f89`, current one in `#a9b1d6`. Source: the same SSE `tool.input.started`→`tool.called` pairs R6.4 already tracks; keep a ring buffer of the last 8 per session. This is the "alive" signal: the list scrolls as the agent works.
  - Question tiles: show the **full final assistant message** (the text the question heuristic matched on), wrapped, up to the tile height. R6.7 already fetches it once on transition to idle.
  - Needs-you plain / idle tiles: wire title wrapped, then nothing. Blank is acceptable here; these are not where the eye should go.
- Below 14×7 project rect: tag only. All-idle projects are not tiled at all (R3.2/R9: idle is context only).

### Weak points
- *Minimal-card truncation*: a tile too small for words is still a legible colour block with a glyph; state is carried by background, never by text.
- *Cliff*: the ladder is continuous (5 lines → 3 → 2 → 1 → glyph → colour); no summary tile exists.
- *Bigger = more sessions*: literally area. The 22-session project takes ~40% of the screen — this is the signal the current build's math had but the render didn't show, because here the interior is filled with tiles, not padding.

### Known costs
- Sizes still change when a session count changes (positions hold with fixed-order squarify, sizes don't). At 4 projects this is a resize of neighbouring regions, not a shuffle; acceptable, but it is motion the flow-grid directions don't have.
- Under STRESS (22-session project, 150 cols) most tiles show a cut nickname and cut action; at 80 cols mostly colour blocks and ages. State stays readable; text doesn't. Acceptable for a case the user says won't happen; not acceptable if it does.
- Colour-blind users get the glyph and the badge, not the field.
- State-weighted tile area re-introduces "size means status", which R5.2/R6.1 retired for *cards*. Here it's the mechanism that makes a question the biggest thing in its project; flagging so the requirements doc gets updated deliberately rather than drifting.

---

## D · Signal — exception-first

**Thesis.** Pick this when most sessions are ignorable and only exceptions matter: needs-you gets the stage, running is a compact stream, projects are a one-line index each. The screen stays calm; nothing you can't act on takes space.

**Is NOT:** project-grouped for active work (this deliberately breaks R7's "project-grouped" for zones 1–2 — that is the thesis, not an oversight); not a list of everything.

### Layout
Three zones top to bottom, each with a header row: `▍` + `TITLE` bold in zone colour + count dim + a `─` rule in `#292e42` to the right edge.
- **Zone 1 `NEEDS YOU`** (red): rail cards (§0.4) 30 wide [39], `floor((W−2)/30)` per row (5 at 150), sorted question → needs-you longest-waiting; line 1 tail is the **project name in its accent** (cards are outside their project, so the project must be on the card). Rows capped so that zone 2 keeps ≥4 rows and zone 3 keeps all projects; overflow line `+N more waiting on you` yellow.
- **Zone 2 `RUNNING`** (blue): one line per running session, two columns [one]: `◐ nick(14)  project(14, accent)  action…  ↳N(cyan)  age(4, right)`. Fills whatever height remains above zone 3; overflow `⋯ +N more running` dim.
- **Zone 3 `PROJECTS`** (purple): exactly one line per project, always fully shown: `▍name(16, accent bold)  glyph string  count(2, bold)  sparkline(10)  2 need · 2 run · 1 idle`. The glyph string is one character per session (`? ● ◐ ○` in state colour, sorted, `?` bold), clipped at 24 [12] with `›`.

### Weak points
- *Minimal-card truncation*: the minimal form of a session is one glyph in the index, and one full ledger line in the running stream — both complete.
- *Cliff*: zone 1 → `+N more waiting`; zone 2 → `⋯ +N`; zone 3 never degrades (it's the floor of the order). Idle sessions never take more than one glyph.
- *Bigger = more sessions*: the glyph string — 5 vs 4 is literally 5 vs 4 characters, next to the count and the sparkline.

### Known cost
No per-project context for running work: to know what `big-monorepo` is doing you read six scattered lines in zone 2. Cards in zone 1 reorder as waiting times change (by design, but it is motion).

---

## Real-scale findings (4 projects, 8 sessions — the case that decides)

Rendered with the REAL fixture at 150×42 (`shots/*-real-wide.png`; C also at 80×36, `shots/C-real-narrow.png`). Rows used out of 40 available:

| | Rows used | What the screen looks like | Text readable in full? |
|---|---|---|---|
| A Swimlanes | 12 | four 3-row bands, then 28 blank rows | no — cards are 24 wide, questions cut at ~22 chars |
| B Ledger | 11 | two short columns, then 29 blank rows | yes |
| C Mosaic | 40 | every cell filled; four regions, one to three roomy tiles each | yes — questions, 140-char title, action lines, all 3 subagent lines |
| D Signal | 13 | three short zones, then 27 blank rows | mostly (question cards 30 wide) |

At this scale the ladders, caps and overflow lines that made A/B/D look good under STRESS never fire. What's left is: three list-shaped views that use a quarter of the screen, and one view that uses all of it. The user asked for the WinDirStat feeling and "no wasted dead space"; only C delivers that at the scale that matters. The concerns that argued against C (truncation, re-tiling churn, loudness) were all stress-scale concerns — with 2 sessions per project the tiles are ~55×18 cells and everything fits.

The STRESS check on C (`shots/C-wide.png`, fixed-order squarify) shows it doesn't break badly: state stays legible as colour + glyph + badge, the 22-session project visibly dominates, text degrades to nickname · age. It's not pretty there, but the user says that case is not real usage.

## Recommendation (revised for real scale)

**Build C (Mosaic)**, with the three refinements now written into its section: tinted question tiles + inverse badge (not solid red), fixed-order squarify for projects (positions hold, sizes flex), and content-scales-with-area (wrap instead of truncate; recent-actions list on running tiles; full assistant message on question tiles). Reasons, checkable in `shots/C-real-wide.png`:

1. It is the only direction that uses the screen at real scale. A/B/D leave 70% blank with 8 sessions; there is no ladder tweak that fixes that, it's what lists do.
2. Every piece of text the user said they want to "make a judgment call" on is readable in full: both questions, the 140-char title, every action line, all three of `sable-heron`'s subagents, and the idle session's age.
3. "Bigger project = more sessions" is literal area, and at 3/2/2/1 you can see it: web-dashboard is visibly the largest region, scratch-cli a strip.
4. State is a colour field plus a badge — readable from across the room, and the question badge is the same loud element A/B/D use, so nothing is lost versus them.
5. It is the WinDirStat feeling the user asked for, and the one render of the four that a reviewer will call "cool" without prompting.

**What C doesn't handle well:** region sizes shift when a count changes (positions don't, with fixed order); a 1-session project is one big tile that needs the recent-actions list to not look empty; under STRESS the text goes away and only colour/glyph remain; colour-blind users rely on glyph + badge. None of these bite at 4×2.

**B stays as the fallback**, and is cheap to keep as a `l` (list) toggle later since it shares the line grammar in §0.4: it is the right view if real usage grows past ~6 sessions per project, or if the user finds the colour fields loud after a week. **A** and **D** are dropped — A's own worst-case is the typical case here, and D's exception-first structure is overkill for 8 sessions.

### Should treemap-by-count survive? — Yes, at real scale, with two fixes

My earlier "replace it" verdict was reasoned against the STRESS fixture. Against the REAL one, the three objections mostly dissolve:

- *"Area encodes the wrong variable"* — with 8 sessions, nothing is hidden by area; every session is on screen with full text. Urgency is carried by the badge and tile-level weight, not by project area. Fine.
- *"Must re-tile to stay proportional"* — with fixed-order squarify and 4 projects, a count change resizes neighbouring regions but does not move a project to a different corner. That satisfies R5.7's intent ("learn where things are").
- *"Fixed-size text in proportional area is padding"* — this one was real and is the actual root cause of the current build's blank space. The fix is content-scales-with-area (tiles that fill their region with wrapped text + history), not abandoning the treemap.

So: the foundation survives; the execution was wrong in four specific ways — fixed-size cards inside proportional regions, weight-sorted placement that separated the boxes you wanted to compare, six coloured borders competing with state colour, and a minimal-card form that carried no state. Each is addressed above. If real usage ever looks like STRESS, that is the trigger to flip the `l` toggle to B.

---

## Appendix — original stress-scale recommendation (superseded, kept for the reasoning)

**Build B (Ledger).** Verifiable reasons, all visible in `shots/B-wide.png` and `shots/B-narrow.png`:

1. It is the only direction where every session, in every state, at every width, is a complete sentence: glyph · nickname · question/action/title · age. The "cut-off nickname and nothing else" failure cannot occur.
2. Questions are readable in full (`Which service owns the billing table?`). A and C both truncate them; R6.7's own words were that question sessions must be prominent, and a highlight you can't read isn't prominent.
3. Subagents are indented `↳` children — no substitution/append mode, no variable card height.
4. Busyness is honest and comparable: one block per session in the header bar, sorted by state. 22 vs 11 vs 5 is visible before reading anything.
5. It is the only direction that keeps the same shape at 80 columns (single column, identical lines). The advisor called that the decisive render.
6. "Sleek" in TUI-land is what k9s/lazygit/btop look like: rails, bars, sparklines, tight columns, one accent per project instead of six coloured borders. B is that.

**What B doesn't handle well:** a 22-session project eats a column at wide widths (cap at 12 lines/project in production and lean on `⋯ +N`). No 2D shape. Sparklines need a per-minute event counter.

**Mashup worth building next:** B's ledger with D's zone 1 on top — one row of question-only rail cards (max 5 at 150 cols) above the two columns. That gives B the one thing it lacks (questions always top-left regardless of project) for 4 rows.

**C wins if** the dashboard is a wallboard read from across the room, not a working screen — 50+ sessions where per-session text can't fit anyway and colour-area is the only channel left. It gets the "cool" in three seconds; it is not what you read a question off. **A wins if** the project×state matrix is the mental model the user wants to keep.

### Should treemap-by-count survive?

No — replace it. The requirements doc already retired it (R5/R5.1/R5.3); the brief re-introduced it. A fresh look agrees with the retirement, for three reasons that are not execution problems:

- **Area encodes the wrong variable.** Count is context; needs-you is the decision. In the fixture, 22 fleet health checks get 40% of the screen and the 1-session project waiting to destroy staging gets a corner.
- **A treemap has to re-tile to stay proportional.** R5.7 (stable slots) was a direct user pick. A treemap that can't move stops being proportional; one that moves defeats "learn where things are".
- **Fixed-size text in proportional area is padding, not content.** That is the blank space the reviewer saw. Either content scales with area (C — text gets cut) or area stops mattering (a flow grid). On a character grid there is no third option.

Treemap is the right foundation only under the wallboard condition. Then build C and accept re-tiling.
