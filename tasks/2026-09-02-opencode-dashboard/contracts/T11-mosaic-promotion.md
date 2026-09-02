# T11 — Mosaic layout and card rendering (spike promotion)

**Contract version** — 1

**Context** — goal: promote the already-verified Mosaic layout/render code
from the throwaway spike (`tmp/20260901-prototype-dashboard-layout/`) into
`crates/dashboard`, rewired to consume T09's real session snapshots and
T10's real assigned names instead of the spike's synthetic fixtures · who
uses it: T12's main loop calls this task's render function every frame · scale:
`overview.md` R5.8 design center (~8 sessions/4 projects) · criticality:
release-critical (delivery profile workflows 3-6) but algorithm-verified —
the risk here is faithful porting and correct wiring to real data, not
inventing new layout behavior. This is a promotion task, not a redesign.

**Delivery profile** — `../delivery-profile.md` version 1 · task override:
none. Per Amendment 1, card content scales continuously with tile size — do
not reintroduce a fixed line count. Per Amendment 2, chrome (borderless) and
minimum tile sizes are already answered; this task does a confirmation pass
against the ported code, not a re-derivation.

**Boundaries**

- **Owns:** `crates/dashboard/src/` — new layout/render modules, ported from
  `tmp/20260901-prototype-dashboard-layout/src/{squarify,layout,ladder,
  render,palette}.rs`. `crates/dashboard/Cargo.toml` — add `ratatui` (and
  any rendering-only deps the spike used) — this is the first M3 task
  allowed a TUI dependency, and it belongs only here, per R1.1.
- **Must not touch:** T09's adapter/model/resolver code, T10's naming
  internals (read its public output type only), the main event loop /
  keyboard input / terminal lifecycle (T12), `tmp/20260901-prototype-
  dashboard-layout/` itself (copy from it, don't edit the spike in place —
  it stays the reference/evidence artifact).

**What "promotion" means here** — the squarify algorithm, the two-pass
region/tile packing, the tile-weight rules, and the content-scaling ladder
are already verified against a real ratatui build (render evidence in that
spike's `renders/` directory, cited by `layout.md`'s own Purpose section).
Port the working logic; don't redesign it. The actual work is: (a) replace
the spike's fixture data source with T09's `Session` type and T10's naming
output, (b) confirm every spec rule below still holds once real field values
(elapsed-time computed from T09's timestamps, real action-line text, real
nicknames) flow through instead of canned fixture data, (c) fill in whatever
the spike doesn't already cover — grep it first; `too small`, `empty`,
`collapse`, and `summary tile` all already appear in its `src/`, so most of
R9 is likely already built, but verify against R9/R9.1/R9.2's exact wording
below rather than assuming full coverage.

**Layout rules to preserve exactly (`layout.md`, full detail there)**

- R5/R5.1: two-pass squarify (project regions, then per-region tiles);
  project weight = top-level session count (subagents never add weight,
  idle-only projects excluded from packing → footer `hidden: <name> (N
  idle)`); projects packed in first-appearance order, never re-sorted;
  regions snap to shared edges, no gaps/overlap.
- R5.2: tile weight = content demand (idle=1, running/needs-you/question=2,
  +1/subagent) — never status-for-area. Idle sessions pulled out of tile
  packing into a chip row (`○ nick · age`) per the exact height thresholds
  (`h≥3` shows it, `h==2` folds into tag row, `h==1` tag row only); `+N idle`
  overflow when chips run out of room.
- R5.3: tile content regime table, exactly as specified — the `wi`×`h`
  breakpoint table, status-line short/long forms, compact form (4 rows,
  `h=3-4`), extended form (elastic block, priority-ordered fixed blocks
  dropped from the bottom when they don't fit) — one variant per attention
  state (running/question/needs-you-plain/idle), each with its own block
  order.
- R5.4: full recompute on resize and on any classification/membership/
  weight/order change — no caching, no positional-stability logic (R5.7 is
  explicit that none should exist; don't add sticky slots even if it "would
  help" — that's a rejected finding, not a gap).
- R5.5: minimum sizes — 12x5 tile, 14x7 region (collapses to project-summary
  tile below that), 40x12 viewport (terminal-too-small panel below that).
- R5.6: 3-tile-per-project cap, priority order (question, then
  longest-waiting needs-you, then running, then most-recent idle), overflow
  chip for the rest; subagents render as first-class tiles with a `↳` prefix
  inside their parent's content, not a third nesting level.
- R5.11: project accent color on the region tag's project name only — never
  a tile background or any border.
- R6/R6.1/R6.2/R6.7 (visuals, ported alongside layout since the spike built
  them together): Tokyo Night palette, borderless chrome (confirm, don't
  re-derive — Amendment 2), attention-state color/glyph/sort order (never
  tile area) for the three states, question sub-badge distinct styling.
- R1.8: the harness-tag slot is a documented seam only — no visible glyph in
  V1 (only one harness kind exists), but don't hardcode assumptions that
  would make adding the slot later a retrofit (e.g. don't hardcode a tile's
  content to occupy a width that would leave no room for a future slot).

**Degrade states to verify/complete (`layout.md` R9 series)**

- R9: zero active sessions in window → centered panel, exact copy pattern
  `No sessions updated in last Xm — N older sessions hidden — press ] or a`.
- R9.1: below 40x12 viewport → centered `terminal too small` panel, nothing
  else drawn.
- R9.2: degrade hierarchy in priority order — full tiles → project-summary
  tiles → aggregated `+N projects` chip → terminal-too-small panel.
  Readability first, then project presence, then proportionality, then
  filling 100% of the screen last (leaving tinted blank space is correct
  behavior, not a bug to fix).

**Conventions** — `cargo build/test/clippy/fmt` per `CONTRIBUTING.md`. Use
`ratatui::TestBackend` + a `--dump` flag for non-interactive render evidence
(the verified pattern from this run's world-facts — `enable_raw_mode()`
fails cleanly under it, no panic; plain-text dumps carry no color, so verify
color separately if a rule depends on it).

**Skills to read and apply** — `code-quality`.

**Acceptance — done when:**

1. Squarify + two-pass packing ported with no behavioral change from the
   spike's verified output shape.
2. Tile/region weight rules, idle chip row, content regime table (all four
   attention-state block orders), minimum sizes, 3-tile cap + overflow, and
   accent-color placement all match the spec exactly, driven by real T09
   session data and T10 nicknames — not fixture data.
3. R9/R9.1/R9.2 degrade hierarchy fully implemented and demonstrated (not
   assumed carried over from the spike without checking).
4. Elapsed-time strings are computed at render time from T09's stored
   timestamps, per state's correct basis (turn-start/turn-end/last-update) —
   never a value baked into the snapshot itself.
5. Render evidence captured via `TestBackend --dump` at the R5.8 design
   center (4 projects/8 sessions) plus at least: one zero-active case, one
   below-40x12 case, one single-low-weight-project case (sliver check).
6. No opencode-specific knowledge anywhere in this task's code — it reads
   only T09's snapshot type and T10's naming output.
7. `ratatui` (and render-only deps) added to `dashboard`'s `Cargo.toml`
   only, not `opencode-client`'s.
8. `cargo build/test/clippy/fmt` clean workspace-wide; nothing outside this
   contract's owns-list touched; the spike directory itself is untouched.

**Gate** — report-only (refine-loop).

**Dependencies** — T09 (session snapshot type), T10 (naming output).

## Review Frame

*Authored by the advisor. Governs disposition and review budget — never what
the reviewer may look at or discover. It cannot suppress credible severe
evidence.*

**As of** — contract version 1

**Context** — Promotion of a verified spike — the algorithm is proven, the risk
is faithful porting and correct wiring to T09/T10's real data.

**Expectations** — Spike behavior preserved through the port (AC 1). Content
regime table correct for all four attention states with real data (AC 2, 4).
Degrade hierarchy complete and demonstrated (AC 3, 5). No opencode knowledge
in render code (AC 6).

**Depth** — Don't re-audit the squarify algorithm (spike-verified). Focus on
data-wiring correctness, regime completeness, and degrade states. Cosmetic
tile-content choices within spec range are not findings. 2 passes.
