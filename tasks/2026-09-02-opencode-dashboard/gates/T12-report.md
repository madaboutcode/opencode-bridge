<!-- gates/T12-report.md. Written by the task's runner; it is the conductor's entire
     gate and must stand alone. -->

# T12 — gate report

**Conformance:** yes — reviewer's (`ask_opus`) explicit pass-1 verdict against
the contract's Acceptance section, all ten criteria individually confirmed:
`dashboard` builds, pairs with a live opencode server via the shared
`opencode_client` pairing path (no MCP process required, `overview.md` R1.2),
and renders real session data through T11's `mosaic::draw` (AC1); terminal
restore proven, not asserted, on normal exit, `q`/`Esc`, Ctrl-C, and a forced
panic via a real test that enters raw mode, panics mid-render, and asserts
`is_raw_mode_enabled()` is false afterward (AC2); all six R8 window-control
deltas exact, including the 1-60m clamp and the no-auto-show-all-past-60 rule
(AC3); `j`/`k`/arrow navigation wraps at both ends and follows the current
frame's actual on-screen order, verified against T11's real `Rect`s from a
`TestBackend` render rather than an assumed order (AC4); help overlay opens
closes correctly and lists every real R7.1/R8 binding, asserted against the
rendered buffer text rather than a hardcoded list (AC5); footer renders the
exact literal format in both windowed and show-all modes, with live/idle
counts from this task's own R3 classification (AC6); Enter is unbound, no
trace/zoom view built (Amendment 3 respected) (AC7); a manual smoke
instruction is recorded below (AC8); claim wiring proven both directions for
subagents and tombstones, by test not code read (AC9, detail below); `cargo
build/test/clippy(-D warnings)/fmt --check` clean workspace-wide, confirmed
independently by both the runner and the reviewer, no T09/T10/T11 internals
touched beyond their public interfaces (AC10).

**Calibration:** delivery profile version 1 · contract version 3 · Review
Frame "as of" contract version 3 — confirmed by direct read of the contract
before spawning either agent, matched, no mismatch.

**Passes:**

- Pass 1 — implementer: `coder` (Agent-tool subagent). Built
  `crates/dashboard/src/main.rs` (replaced the T08 placeholder — pairs with
  opencode via `opencode_client::resolve_bin`/`pair`/`Client::new`/`health`,
  the same path `opencode-bridge` uses, starts the adapter, hands its channel
  to the shell), `crates/dashboard/src/lib.rs` (one line: `pub mod shell;`),
  and a new `crates/dashboard/src/shell/` module (10 files, ~1700 lines incl.
  tests: `mod.rs`, `terminal.rs` — R2 lifecycle guard, `window.rs` — R8 state
  machine, `nav.rs` — R7.1 reading-order/wrap, `reclassify.rs` — R3
  active-window reclassification, `live.rs` — T10 claim wiring (AC9),
  `keys.rs` — key-to-action mapping, `footer.rs`/`help.rs` — R7.1 chrome,
  `app.rs` — the event loop and `App` state). No new Cargo dependency —
  raw-mode/alt-screen calls go through `ratatui::crossterm`, already pulled
  in transitively. 117 tests in `dashboard` (up from ~90 after T11; exact
  prior count not re-verified but all 117 pass now), `cargo
  build/test/clippy/fmt` all clean.

  Implementer surfaced seven judgment calls unprompted, reviewed
  independently below rather than accepted at face value:

  1. **Footer ownership conflict.** T11's `render.rs` draws its own footer
     row (`hidden:`/`aggregated:` text, generic hint) that doesn't match
     R7.1's mandated literal format (`window: W (N live / M idle)`). Rather
     than edit T11 (out of this task's boundary), T12 overwrites that row
     post-render via the public `Frame`/`Buffer` API, folding T11's
     `hidden`/`aggregated` signal into trailing text so it isn't lost.
     Reviewer independently confirmed this produces the correct literal
     format without corrupting adjacent cells.
  2. **Selection highlight.** `mosaic::draw`'s signature carries no
     selection parameter, so T12 overlays a `Modifier::REVERSED` toggle on
     the selected tile's cells post-render. Reviewer confirmed the cell
     range computed for the toggle matches the selected tile.
  3. **Arrow-key mapping** (Left/Up→prev, Right/Down→next) — `interactions.md`
     R7.1 has an open `[REVIEW]` on this; both implementer and reviewer
     treat it as a reasonable call, not a defect.
  4. **`]`/`[` from show-all mode** exits show-all and resumes from the last
     numeric `W` — undecided in the spec; tested in `window.rs`, reviewer
     confirmed sane behavior, not a defect.
  5. **Shift+]/Shift+[ detected by character** (`}`/`{`) rather than a
     modifier flag, with a documented US-layout assumption (most terminals
     don't report `SHIFT` alongside an already-shifted symbol). Reviewer
     confirmed this is a reasonable implementation choice given the terminal
     input model, not a bug.
  6. **`window_minutes` under show-all** — T11's `draw()` takes a plain
     `u32` with no separate show-all bool, so T12 feeds it the last numeric
     `W` even while show-all is active (T11 only uses it for cosmetic
     empty-state text under show-all; T12 pre-filters the session list
     itself for the show-all case). Reviewer confirmed this doesn't cause
     T11 to incorrectly filter sessions that should be showing.
  7. **No SIGTERM/`kill` handling.** R2's prose mentions "on kill" but AC2's
     enumerated exit paths are normal exit, `q`/`Esc`, Ctrl-C, and panic —
     SIGTERM is not among them, and no AC exercises it. Implementer flagged
     this explicitly as a real gap rather than silently dropping it, judging
     that adding real signal handling would need a new dependency
     (`signal_hook` or raw `libc`) for a path nothing tests. Reviewer formed
     an independent view (not just accepting the implementer's framing) and
     concluded this is correctly scoped as a deferral, not a release-bar
     finding: AC2 is exhaustively specific about which four paths it
     requires, SIGTERM isn't one of them, and the delivery profile's release
     bar names "terminal restore on panic" specifically, not kill signals
     generally. See `deferred.md`'s new T12 heading.

  Reviewer: `ask_opus` (Agent-tool subagent), independent judgment. Read the
  contract, delivery profile, and all named upstream source files directly
  (`adapter.rs`, `snapshot.rs`, `naming/{mod,claim_map}.rs`,
  `mosaic/{mod,render}.rs`, `opencode/reconcile.rs`) rather than trusting the
  contract's prose. Verified AC9's claim wiring both directions with the
  implementer's tests: a subagent fixture reaches a real claimed nickname
  (not the id fallback) after the wiring runs, and a tombstone fixture's slot
  is observably freed in `NamingClaimMap`'s state, available to a later
  claim, both proven by test not code read. Verified R3/R3.1's active/idle
  reclassification is computed by T12 itself from `SessionSnapshot::
  last_updated` against the current window, never from `AttentionState`'s
  own timestamp basis or an opencode-native field — confirming the exact gap
  T09's gate report deferred is genuinely closed here, not just assumed
  closed. Verified the `NamingClaimMap` `Default`-vs-`new()` footgun (flagged
  in T10's deferred.md entry) is correctly avoided: `LiveState::new()` calls
  `NamingClaimMap::new()`, with a doc comment explaining why, not
  `Default::default()`. Verified via `git diff HEAD` that no T09/T10/T11
  path was touched. Independently re-ran `cargo build/test/clippy(-D
  warnings)/fmt --check` rather than trusting the implementer's report — all
  clean. Reviewed all seven of the implementer's self-flagged judgment calls
  independently (see above) and confirmed each reasonable, including forming
  an independent view on the SIGTERM gap rather than accepting the
  implementer's own disposition of it. Verdict: clean, no correctness
  findings, pass on all ten criteria. Did not spend pass 2 — see below.

  Runner's own check, before and independent of the reviewer: confirmed via
  `cargo build --workspace`, `cargo test --workspace` (117/117 `dashboard`
  tests pass), `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo fmt --check` run directly — all clean; confirmed via `git status
  --short`/`git diff --stat` that only `crates/dashboard/{src/lib.rs,
  src/main.rs}` were modified and only `crates/dashboard/src/shell/` was
  added — nothing under T09's, T10's, or T11's owns-lists touched.

- No pass 2. Per the refine-loop's stopping rule ("pass 1 clean → done"): the
  reviewer's pass-1 verdict was conformance-yes on every criterion with zero
  correctness findings to fix, so a verification pass had nothing to verify.
  Passes used: 1 of the 2-pass budget.

**Residuals:** none above the depth line.

**Challenges:** none — neither agent contested the delivery profile or the
contract's Review Frame. The reviewer did form an independent (not
rubber-stamped) view on the SIGTERM disposition per the caller's brief, and
concluded the implementer's own scoping of it was correct — see judgment
call 7 above.

**Contested:** none.

**Manual smoke instruction (AC8):** on a machine with a real terminal, start
`opencode2 service start` (or ensure an opencode server is already paired),
then run `dashboard`; confirm it shows live sessions rendered via Mosaic;
press `]`, `[`, `w`, `a`, `j`/`k`, `?` and confirm each responds as specified;
quit with `q`; confirm the terminal is left sane afterward (e.g. `tput smso;
echo test` behaves normally, no residual raw-mode/alt-screen state). The
implementer ran the connect path against a real local opencode2 server in
their sandbox (no TTY available there): pairing and health-check succeeded,
then `TerminalGuard::enter()` failed cleanly with `Device not configured (os
error 6)` — one stderr line, exit code 1, no panic, no hang — real evidence
the connect/pairing path itself works even though the interactive TTY portion
couldn't be exercised in that sandbox. The full interactive smoke test above
still needs running on a real terminal before this ships to a user.

**Deferred:** 1 item appended to `deferred.md` under a new "T12" heading —
SIGTERM/`kill -TERM` does not run the Drop-based terminal-restore guard (Rust
destructors don't run on raw signal delivery by default); R2's prose
mentions "on kill" but AC2's four enumerated exit paths (normal exit, q/Esc,
Ctrl-C, panic) don't include it, and Ctrl-C already covers the ordinary
interactive "stop this" case. See `deferred.md` for full detail, assumption,
and promotion trigger.

**Files changed (owns-list, per the contract's Boundaries section):**
- `crates/dashboard/src/main.rs` (replaces the T08 placeholder — pairing,
  adapter startup, hands off to the shell)
- `crates/dashboard/src/lib.rs` (edited — `pub mod shell;` only)
- `crates/dashboard/src/shell/mod.rs` (new — module root)
- `crates/dashboard/src/shell/terminal.rs` (new — R2 terminal lifecycle:
  raw mode, alt screen, Drop/panic guard)
- `crates/dashboard/src/shell/window.rs` (new — R8 window `W` state machine)
- `crates/dashboard/src/shell/nav.rs` (new — R7.1 reading-order navigation,
  wrap)
- `crates/dashboard/src/shell/reclassify.rs` (new — R3/R3.1 active/idle
  reclassification from `last_updated`)
- `crates/dashboard/src/shell/live.rs` (new — T10 claim wiring for every
  live session including subagents, tombstone-to-release wiring, AC9)
- `crates/dashboard/src/shell/keys.rs` (new — key-to-action mapping)
- `crates/dashboard/src/shell/footer.rs` (new — R7.1 footer literal format)
- `crates/dashboard/src/shell/help.rs` (new — R7.1 help overlay)
- `crates/dashboard/src/shell/app.rs` (new — the event loop and `App` state,
  T11's `draw()` call site)

Nothing under `crates/dashboard/src/{adapter.rs,snapshot.rs,
project_identity.rs,opencode/**}` (T09's types, read-only consumed),
`crates/dashboard/src/naming/{claim_map.rs,wordlist.rs}` (T10's internals,
only public output consumed), `crates/dashboard/src/mosaic/**` (T11's
internals, only public `draw()`/`DrawReport` consumed),
`tmp/20260901-prototype-dashboard-layout/` (spike, untouched), or
`docs/specs/**` was touched.
