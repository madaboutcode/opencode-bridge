# T12 — Interactive shell: main loop, window controls, keyboard nav

**Contract version** — 3 (conductor fix, from T11's gate: AC 9 now names
subagent sessions explicitly — T11 surfaced that nothing assigned
responsibility for claiming their names, and this task's live-session loop
is where that responsibility belongs)

**Context** — goal: wire T09 (data), T10 (naming), and T11 (render) into one
running, keyboard-driven TUI that satisfies `overview.md` R2's terminal-
lifecycle guarantee — this is the task that makes `dashboard` a real program,
replacing T08's placeholder entirely · who uses it: the end user, running
this daily against a live opencode server · scale: `overview.md` R5.8 ·
criticality: high — R2's terminal-restore-on-crash guarantee is explicitly
named in the delivery profile's release bar ("a TUI that leaves the terminal
in raw mode on crash is a user-visible break").

**Delivery profile** — `../delivery-profile.md` version 1 · task override:
per Amendment 3, session zoom is cut from v1 — Enter is unbound or shows a
"not yet" affordance; do not build a trace-view screen.

**Boundaries**

- **Owns:** `crates/dashboard/src/main.rs` (replaces the T08 placeholder),
  plus a new module for the event loop / app state (window value, selection,
  help-overlay-open flag) — exact file layout is the implementer's choice.
- **Must not touch:** T09/T10/T11 internals beyond calling their public
  interfaces — this task is glue and interaction state, not a fourth place
  that reimplements adapter, naming, or render logic.

**What this task wires together**

- **R2 — terminal lifecycle.** Full-screen takeover; a Drop/panic guard
  restores the terminal (cursor, screen mode, raw mode) on normal exit, on
  kill, and on panic; resize is handled; input responds within ~250ms
  without busy-waiting the CPU (a poll/tick event loop, not a spin loop).
- **R3/R3.1 — active window.** `W` defaults to 10 minutes, adjustable at
  runtime (below). "Active" is computed from time-since-last-snapshot (T09's
  timestamps), never from any opencode-native "updated" field.
- **R8 — window control keys**, exact deltas: `]` = `W += 5m`, `[` = `W -=
  5m`, `w` = reset to 10m, `a` = show-all (every session active regardless
  of age), `Shift+]` = `W += 1m`, `Shift+[` = `W -= 1m`. Clamp 1-60m; `]`/
  `Shift+]` never auto-transition into show-all past 60 — that only happens
  via `a`. Every key triggers an immediate recompute (calls T11's layout),
  no animation, no debounce.
- **R8.1 — fixed defaults.** Exactly the R7.1/R8 bindings below; no
  rebinding UI, no config file read for this purpose in V1.
- **R7.1 — navigate/back-quit/help** (zoom excluded per Amendment 3):
  - Navigate: `j`/`k`/arrow keys move the selection through the *current
    frame's* on-screen reading order (left to right, top to bottom, exactly
    as T11 placed tiles this frame — that order can change frame to frame,
    `layout.md` R5.7, and this task doesn't fight that). Wraps at both ends.
  - Enter: unbound, or a visible "not yet" affordance — no trace view.
  - Back/quit: `q`/`Esc` — since there's no zoom view in v1, both always quit
    from the main screen (restoring the terminal per R2); from the help
    overlay, they close it back to the main screen.
  - Help: `?` opens an overlay listing the R7.1 + R8 bindings; `?`/`q`/`Esc`
    closes it back to whatever was showing underneath (footer hidden while
    the overlay is open, per `interactions.md`'s own note).
  - Footer: always visible except under the help overlay — literal form
    `window: W (N live / M idle)` (or `window: all (N live / 0 idle)` under
    `a`), `N`/`M` from this task's own R3 active/idle classification, plus a
    short key-hint reminder (exact hint wording is this task's call, per the
    spec's own `[REVIEW: OPEN]` on that text).
- **R9/R9.1 triggers.** This task supplies the real viewport size and real
  session/idle counts into T11's render call every frame; T11 (already
  built) decides whether to draw the empty-state panel or the
  terminal-too-small panel from those inputs. This task does not reimplement
  either panel's content.

**Conventions** — `cargo build/test/clippy/fmt` per `CONTRIBUTING.md`. Prove
R2's panic-restore guarantee with a real test (e.g. force a panic mid-render
under a raw-mode-entered state and assert the terminal is restored), not
just a description of the Drop guard.

**Skills to read and apply** — `code-quality`.

**Acceptance — done when:**

1. `dashboard` builds, connects to a live opencode server the same way the
   existing MCP bridge does (paired via the local password file, no MCP
   process required — `overview.md` R1.2), and renders real session data
   through T11.
2. Terminal is restored correctly on normal exit, `q`/`Esc`-triggered quit,
   Ctrl-C, and a forced panic — proven, not just asserted.
3. All R8 window-control keys behave exactly as specified, including the
   1-60m clamp and no-auto-show-all-past-60 rule.
4. R7.1 navigate wraps correctly at both ends and follows the current
   frame's actual on-screen order (a test can drive this against T11's
   output order directly, without needing a live server).
5. Help overlay opens/closes correctly and lists the real current bindings
   (not a stale hardcoded list independent of what R7.1/R8 actually bind).
6. Footer renders the exact literal format in both windowed and show-all
   modes, with live/idle counts matching this task's own classification.
7. Enter does nothing that resembles a trace/zoom view — either unbound or a
   simple "not yet" affordance.
8. A manual smoke instruction is recorded in the gate report: start
   `dashboard` against a real paired opencode server, confirm it shows live
   sessions, respond to a few keys, quit cleanly, and check the terminal is
   left sane afterward.
9. **Claim wiring, both directions, for every live session including
   subagents.** T11's gate surfaced that nothing in T10's contract or
   `visuals.md` R6.8 assigns responsibility for claiming a *subagent*
   session's name — T11's render layer falls back to the subagent's raw
   harness-native id when T10 has never been asked to claim one. This task
   closes that gap: subagent sessions (`parent_id.is_some()`) are ordinary
   sessions under `client.md` R1.5's identity model and R5.6 renders them
   with a claimed `↳ nick action` line, so the main loop calls T10's claim
   for **every** live session T09 reports — top-level and subagent alike —
   not just top-level ones. Symmetrically, **tombstone-to-claim-release
   wiring**: when the main loop observes a tombstone from T09's reconcile
   sweep for any session or project (subagent included), it calls T10's
   release before the next redraw, and the freed name/category becomes
   assignable to a new session/project in that same or a later frame. Prove
   both directions with tests: a subagent fixture that ends up with a real
   claimed nickname (not the id fallback) after this task's wiring runs, and
   a T09 tombstone fixture whose slot is observably freed in T10's claim
   state — not just a code read.
10. `cargo build/test/clippy/fmt` clean workspace-wide; nothing outside this
    contract's owns-list touched.

**Gate** — report-only (refine-loop).

**Dependencies** — T09, T10, T11 (this is M3's last task).

## Review Frame

*Authored by the advisor. Governs disposition and review budget — never what
the reviewer may look at or discover. It cannot suppress credible severe
evidence.*

**As of** — contract version 3

**Context** — Integration task — wires T09/T10/T11 into a running TUI.
Terminal-restore-on-crash is explicitly at the delivery profile's release bar.

**Expectations** — Terminal restored on normal exit, quit, Ctrl-C, and panic —
proven, not asserted (AC 2, release bar). Window controls exact per R8 (AC 3).
Navigation follows current frame's on-screen order (AC 4). Zoom absent per
Amendment 3 (AC 7).

**Depth** — Terminal lifecycle and key bindings are the release-bar surface.
Footer wording (spec-open) and help-overlay cosmetics are not findings.
2 passes. Say clean if it's clean.
