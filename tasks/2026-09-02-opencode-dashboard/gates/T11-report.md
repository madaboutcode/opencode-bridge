<!-- gates/T11-report.md. Written by the task's runner; it is the conductor's entire
     gate and must stand alone. -->

# T11 — gate report

**Conformance:** yes — reviewer's (`ask_opus`) explicit pass-1 verdict against
the contract's Acceptance section, all eight criteria individually confirmed:
`squarify.rs` is a byte-faithful port of the spike's algorithm (only an unused
`label` field dropped) and `layout.rs`'s two-pass region/tile packing
preserves the spike's structure (AC1); weight rules, idle chip row, the full
content-regime ladder (all four attention-state block orders verified line by
line against the spec), minimum sizes, the 3-tile cap + overflow, and accent
placement (verified with buffer-color tests reading the `TestBackend` buffer
directly, not just code inspection) all driven by real T09/T10 types, not
fixtures (AC2); R9/R9.1/R9.2's full degrade hierarchy implemented, tested,
and demonstrated in render evidence (AC3); elapsed-time strings proven
computed at call time from the correct per-state timestamp basis
(Running→`turn_started`, NeedsYou→`turn_ended`, Idle→`last_update`) via a
test that renders the same snapshot at two different `now` values and gets
different ages (AC4); render evidence captured at the R5.8 design center plus
zero-active, below-40x12, and single-low-weight-project cases, all four
inspected directly by the runner before handoff (AC5); no opencode-specific
knowledge — the `mosaic` module imports only `crate::snapshot`/`crate::naming`
(AC6); `ratatui` added only to `dashboard`'s `Cargo.toml`, `opencode-client`'s
untouched (AC7); `cargo build/test/clippy(-D warnings)/fmt --check` clean
workspace-wide, spike directory and owns-list boundaries respected, confirmed
independently by both the runner and the reviewer via `git status`/`git diff
--stat` (AC8).

**Calibration:** delivery profile version 1 · contract version 1 · Review
Frame "as of" contract version 1 — confirmed by direct read of the contract
before spawning either agent, matched, no mismatch.

**Passes:**

- Pass 1 — implementer: `coder` (Agent-tool subagent). Built
  `crates/dashboard/src/mosaic/{mod,view,squarify,layout,ladder,palette,
  render,fixtures}.rs`, `crates/dashboard/examples/mosaic_dump.rs` (render
  evidence harness), `crates/dashboard/renders/*.txt` (captured evidence),
  plus edits to `crates/dashboard/Cargo.toml` (`ratatui = "0.28"` only) and
  `crates/dashboard/src/lib.rs` (`pub mod mosaic;` + re-exports). 27 new
  tests in `mosaic::` (76 total in `dashboard`, up from 49 after T10).
  `cargo build/test/clippy/fmt` all clean on first attempt.

  Beyond straight porting, the implementer found and fixed two real gaps in
  the spike (not just data-wiring, actual missing coverage against the
  contract's own acceptance list): (1) R5.5's 14x7 region-minimum threshold
  was never checked in the spike (only a 1-row sliver check existed) — a
  region sized e.g. 14x6 would have still received full tiles; fixed by
  gating `RegionKind::Full` vs `Summary` on the literal raw 14x7 allocation.
  (2) R5.6's 3-tile-per-project cap + overflow chip was entirely unbuilt in
  the spike (every active session was fed into squarify uncapped); fixed
  with a priority-sort-then-truncate at 3, overflow routed into the same
  bottom-row slot already reserved for idle chips (a documented judgment
  call — the spec doesn't give overflow its own geometry, so the
  already-specified idle-row slot was reused rather than inventing new
  layout). Also added, since the spike had neither: R9's zero-active
  empty-state panel (exact copy string asserted present in a rendered
  buffer, not just checked in the report struct) and R9.2's third degrade
  step, an aggregate `+N projects (M sessions)` chip for regions that fall
  below a 7x2 floor even as summaries — bounded to one re-pack pass, not
  recursive, since this is a stress-scale-only path per the delivery
  profile.

  Reviewer: `ask_opus` (Agent-tool subagent), independent judgment. Verified
  data-wiring correctness field by field (`current_action`→action,
  `wire_title`→title, `final_assistant_text`→assistant text,
  `last_user_prompt`→user prompt, `files_touched`→files, `recent_actions`→
  recent — all landing correctly, `Option` fields degrading to empty string
  cleanly); verified subagents fold into their parent's `subs` list via
  `parent_id` rather than becoming a separate top-level project entry;
  verified project first-appearance ordering and the all-idle exclusion +
  footer `hidden:` reporting; walked all four attention-state extended block
  orders (Running/Question/NeedsYou/Idle) against `layout.md` R5.3 line by
  line and confirmed each matches, including block-drop priority under
  space pressure; swept every `FALLBACK-OK` citation in the diff (9 sites)
  and confirmed each points to a real, specific source (mostly R9.2's "leave
  unused space, don't crash" rule for zero-size draw guards, plus the two
  cross-task items below) — no uncited fallback found. One observation, not
  a finding: the header's `" ◆ opencode "` label is a cosmetic string
  carried from the spike, not opencode wire-shaped knowledge, so it doesn't
  violate AC6; would need parameterizing only if a second harness is ever
  added, out of v1's scope per the delivery profile. Verdict: clean, no
  correctness findings, pass on all 8 criteria. Did not spend pass 2 — see
  below.

  Runner's own check, before and independent of the reviewer: confirmed via
  `cargo build/test/clippy/fmt` run directly (not just trusting the
  implementer's report) that all four are clean; read all four render-evidence
  files directly and confirmed each shows what the implementer claimed
  (design-center's 4-active/1-idle-only project mix with a subagent `↳` line
  and an idle chip row; zero-active's exact R9 copy string; below-40x12's
  bare "terminal too small" panel; single-low-weight-project's 3-tile cap +
  "+3 sessions" overflow chip alongside a full-size sliver-check region);
  confirmed via `git status --short`/`git diff --stat` that only
  `crates/dashboard/{Cargo.toml,src/lib.rs}` were modified and only
  `crates/dashboard/{src/mosaic/,examples/,renders/}` were added — nothing
  under T09's or T10's owns-lists, `tmp/20260901-prototype-dashboard-layout/`,
  or the main event loop was touched.

- No pass 2. Per the refine-loop's stopping rule ("pass 1 clean → done"): the
  reviewer's pass-1 verdict was conformance-yes on every criterion with zero
  correctness findings to fix, so a verification pass had nothing to verify.
  Passes used: 1 of the 2-pass budget.

**Residuals:** none above the depth line.

**Challenges:** none — no finding from either agent contested the delivery
profile or the contract's Review Frame.

**Contested:** none.

**Cross-task flag from T09's deferral, resolved:** T09's gate report and
`deferred.md` entry asked this task to determine whether R6.5's "fall back to
the full relative path if it fits" width-aware fallback needs a new field
added to T09's `SessionSnapshot` (currently carrying only the already-rendered
`"editing: " + basename` string). The implementer built the render layer
using only the basename string T09 actually provides — `current_action` is
passed straight through unmodified, no attempt to reverse-engineer a path
from the rendered string. The reviewer independently signed off on this as
correct given T09's documented snapshot shape (`snapshot.rs`'s own doc
comment on `current_action` says "never a raw tool name or argument object" —
consistent with a pre-rendered, non-decomposable string), not a T11 gap and
not something needing a T09 re-cut. Per the brief's instruction ("if you
determine the render layer can reasonably skip that fallback... that's a
normal deferred-with-trigger entry, not an escalation"), this is being closed
as a deferral, not escalated. See `deferred.md`'s new T11 heading.

**Deferred:** 2 items appended to `deferred.md` under a new "T11" heading —
(1) R6.5's width-aware full-relative-path fallback stays unreachable in v1,
by design, given T09's snapshot only carries the pre-rendered basename
string (see cross-task flag above); (2) a new cross-task gap the implementer
surfaced while wiring, not present in T09's original deferral list: neither
T10's contract nor `visuals.md` R6.8 specifies who is responsible for
calling `claim_batch`/`claim_session` for a *subagent* session, so if T12's
eventual wiring only ever claims top-level sessions, a subagent's
`nickname_of()` lookup always returns `None`; the render layer falls back to
the subagent's own harness-native id (still harness-agnostic, an opaque
string T09 already provides) rather than render nothing or panic. Reviewer
confirmed the `FALLBACK-OK` citation meets the bar. Actual disposition
(should T12 claim subagent nicknames) is a T12-scoping-time call.

**Files changed (owns-list, per the contract's Boundaries section):**
- `crates/dashboard/src/mosaic/mod.rs` (new — module root, `draw()` entry
  point for T12)
- `crates/dashboard/src/mosaic/view.rs` (new — builds `ProjectView`/
  `SessionView`/`SubagentView` from real `SessionSnapshot` + `NamingClaimMap`)
- `crates/dashboard/src/mosaic/squarify.rs` (new — ported treemap algorithm,
  byte-faithful)
- `crates/dashboard/src/mosaic/layout.rs` (new — two-pass region/tile
  packing, idle chip row, 3-tile cap, R9.2 degrade hierarchy)
- `crates/dashboard/src/mosaic/ladder.rs` (new — R5.3 content-regime ladder)
- `crates/dashboard/src/mosaic/palette.rs` (new — Tokyo Night colors/glyphs)
- `crates/dashboard/src/mosaic/render.rs` (new — draws to a `ratatui::Frame`,
  R9/R9.1 degrade panels)
- `crates/dashboard/src/mosaic/fixtures.rs` (new — test/evidence fixture
  builders over real `SessionSnapshot`/`NamingClaimMap` types)
- `crates/dashboard/examples/mosaic_dump.rs` (new — `TestBackend`-based
  non-interactive render evidence harness)
- `crates/dashboard/renders/*.txt` (new — captured render evidence: design
  center, zero-active, below-40x12, single-low-weight-project)
- `crates/dashboard/Cargo.toml` (edited — added `ratatui = "0.28"` only)
- `crates/dashboard/src/lib.rs` (edited — `pub mod mosaic;` and re-exports
  only)
- `Cargo.lock` (edited — `ratatui` and its transitive deps resolved)

Nothing under `crates/dashboard/src/{adapter.rs,snapshot.rs,
project_identity.rs,opencode/**}` (T09's types, read-only consumed),
`crates/dashboard/src/naming/{claim_map.rs,wordlist.rs}` (T10's internals,
only public output consumed), `crates/dashboard/src/main.rs` (T12's event
loop), `tmp/20260901-prototype-dashboard-layout/` (spike, untouched — copied
from, never edited), or `docs/specs/**` was touched.
