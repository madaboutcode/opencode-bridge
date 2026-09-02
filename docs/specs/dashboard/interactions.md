# Dashboard — Interactions

## Purpose

Keyboard navigation, zoom, and the active-window controls — everything the
user does with their hands once the dashboard is running. Source:
`tasks/2026-09-01-opencode-dashboard.requirements.md`, section 5
("Interactions"), R7-R8.1.

## Contents

- [At-a-glance view and navigation](#at-a-glance-view-and-navigation) — R7-R7.1
- [Window controls](#window-controls) — R8-R8.1

No child spec files — this is a leaf file in the tree; see `overview.md`
for the full five-file map.

## Scope

Covered: the at-a-glance main screen and its keyboard controls — navigate,
zoom, back/quit, help, and the footer (R7-R7.1) — and the six active-window
keys plus the fixed-defaults/no-rebinding rule (R8-R8.1).

Not covered: there is no mouse support in V1 (`overview.md` R10) — every
screen and control in this file is reachable and fully operable from the
keyboard alone. The content of the empty-state panel and the trace-view's
layout are `layout.md`'s and `client.md`'s domains respectively; this file
only specifies the keys and the footer text.

## At-a-glance view and navigation

- **R7** — Without opening anything, the dashboard's main screen shows every
  active session across every project at once, grouped by project, each
  showing its attention state at a glance.

  The original requirement text names four states — running, waiting,
  needs-reply, idle — written before the attention model was finalized.
  `visuals.md` R6.7 (CONFIRMED FINAL, 2026-09-02) settled on three: `running`, `needs-you` (which carries a
  distinct "question" sub-badge — what the original wording called
  "needs-reply"), and `idle`. This file uses R6.7's three-state vocabulary;
  "waiting"/"needs-reply" from the original text are not a fourth state.

  Scenario: Given sessions are running across 3 different projects, when the
  dashboard is showing its main screen (no zoom, no help overlay open), then
  all three projects and their sessions are visible at once, each showing
  running/needs-you/idle per `visuals.md` R6.7, with no extra keypress
  needed to reveal any of them.

- **R7.1** — Keyboard controls on the main screen:

  - **Navigate.** `j`, `k`, and the arrow keys move the selected tile.
    Selection follows the tiles' current on-screen reading order — left to
    right, top to bottom, exactly as the Mosaic layout (`layout.md` R5,
    forward reference) has placed them for this frame. That on-screen order
    is not guaranteed to stay the same from one frame to the next
    (`layout.md` R5.7, forward reference). Moving forward past the last tile
    in that order wraps to the first tile; moving backward past the first
    wraps to the last.

    [REVIEW: the source lists "`j/k` or arrow" as one undifferentiated
    group and doesn't say whether all four arrow keys should behave
    identically to `j`/`k` (single step forward/back in reading order), or
    whether Up/Down should instead jump by on-screen row instead of by
    single tile. This file assumes all four keys are equivalent single-step
    forward/backward until decided otherwise.]

    Scenario: Given 6 session tiles in the current frame's reading order
    T1...T6 with T3 selected, when the user presses `j` (or the right/down
    arrow), then T4 becomes selected; when T6 is selected and the user
    presses `j` again, then selection wraps to T1.

  - **Zoom.** Enter (`↵`) zooms the current selection to a full-screen view
    of that session's trace: the chronological list of tool calls that fed
    its tile's one-line "current action" summary (`client.md` R6.4-R6.5,
    forward reference), not just the latest line.

    [REVIEW: the source says Enter zooms the "selected tile/project box",
    but Navigate above only ever selects a tile — there's no described way
    to select a project box as a distinct unit. It's unclear whether
    zooming a project is meant to be a second selection mode, or whether
    "project box" here just means the tile's containing project is shown
    for context inside the same trace view. This file specifies only the
    tile-zoom (single-session trace) case; project-level zoom content is
    undefined until this is resolved.]

    Scenario: Given tile T4 (session "Holmes") is selected, when the user
    presses Enter, then the screen switches to a full-screen trace view for
    session Holmes, showing its tool calls in order, replacing the
    at-a-glance screen.

  - **Back / quit.** `q` and `Esc` do the same thing: back out one level.
    From the full-screen trace view or the help overlay, they return to the
    at-a-glance main screen. From the at-a-glance main screen itself (no
    zoom, no overlay open), they quit the dashboard, restoring the terminal
    exactly as `overview.md` R2 describes.

    Scenario: Given the full-screen trace view is open, when the user
    presses `q`, then the dashboard returns to the at-a-glance main screen
    with the same tile selected as before zooming in; when `q` is pressed
    again from the main screen, then the dashboard process exits and the
    terminal is restored to normal mode.

  - **Help.** `?` opens a help overlay listing the current key bindings
    (this file's own R7.1 and R8 content is the source of truth for what it
    should show). Pressing `?` again, or `q`/`Esc`, closes the overlay and
    returns to whatever screen was showing underneath it.

    Scenario: Given the at-a-glance main screen is showing, when the user
    presses `?`, then a help overlay appears listing the navigate/zoom/quit
    and window-control keys; when `Esc` is pressed, then the overlay closes
    and the main screen is showing again, unchanged.

  - **Footer.** A footer bar is always visible on the at-a-glance and
    trace-view screens (not shown inside the `?` help overlay, which
    replaces it), showing the current window value and live/idle counts in
    the literal form `window: W (N live / M idle)` — e.g. `window: 10m (3
    live / 5 idle)` — where `N`/`M` are the active/idle counts per
    `overview.md` R3/R3.1, plus a short reminder of the available keys.
    When the window is set to "show all" (R8's `a`), every session is
    active by definition (`overview.md` R3.2); the footer shows `window:
    all (N live / 0 idle)` instead of a minute count.

    This footer is distinct from the empty-state panel shown in the body
    when zero sessions are active — that panel's exact copy is specified in
    `layout.md` R9; this file does not repeat it.

    [REVIEW: the source specifies the `window: W (...)` pattern exactly but
    not the wording of the "hints" reminder. Exact hint text is left to
    implementation as long as it accurately reflects the keys in this
    section.]

    Scenario: Given `W` is 10 minutes with 3 sessions active and 5 idle,
    when the footer renders, then it reads `window: 10m (3 live / 5 idle)`
    plus a key-hint reminder; given the user then presses `a`, when the
    footer re-renders, then it reads `window: all (8 live / 0 idle)`.

## Window controls

- **R8** — Six keys adjust the active-window threshold `W` (defined in
  `overview.md` R3/R3.1 — the minutes-since-last-update cutoff that decides
  active vs. idle):

  | Key | Effect |
  |---|---|
  | `]` | `W += 5m` |
  | `[` | `W -= 5m` |
  | `w` | Reset `W` to the default, 10m |
  | `a` | Set `W` to "show all" — every session counts as active regardless of age |
  | `Shift+]` | `W += 1m` (fine-tune) |
  | `Shift+[` | `W -= 1m` (fine-tune) |

  `W` is clamped to 1-60 minutes: `[`/`Shift+[` never take it below 1m, and
  `]`/`Shift+]` never take it above 60m. [REVIEW: the source doesn't say
  whether `]`/`Shift+]` past 60m should clamp at 60m or auto-transition into
  "show all". This file picks clamp-at-60 — reaching "show all" only
  happens via the dedicated `a` key, never by raising `]` past 60 — until
  decided otherwise.] Every one of these keys triggers an immediate layout
  recompute (`layout.md` R5.4, forward reference) with no animation and no
  debounce — the new layout appears on the very next drawn frame.

  [REVIEW: carried forward from the requirements doc's Open Questions —
  the overflow behavior when `a` (show all) is active with many (50+)
  sessions is unspecified. The per-project degrade rules (`layout.md`
  R5.6/R9.2, forward reference) still apply per project, but there's no
  defined global `+N` strategy across projects under `a` at that scale.]

  Scenario: Given `W` is 10m, when the user presses `]`, then `W` becomes
  15m and the layout reflows on the next frame with no animation.

  Scenario: Given `W` is 60m, when the user presses `]` again, then `W`
  stays at 60m (clamped) — it does not become "show all".

  Scenario: Given `W` is 10m, when the user presses `a`, then `W` becomes
  "show all" and every session (regardless of last-update age) is shown as
  active.

- **R8.1** — The six bindings in R8 (plus R7.1's navigate/zoom/quit/help
  bindings) are V1's fixed defaults. There is no in-app way, in V1, to view
  or change what key triggers what action — no settings screen, no config
  file read for this purpose. A future version may add rebinding; V1 does
  not need to reserve any visible UI for it, but picking these particular
  keys was a "good enough for now" choice, not a permanent commitment — see
  the requirements source's own framing ("use sensible defaults - we can
  switch them later").

  Scenario: Given the dashboard is running V1, when a user looks for a way
  to change what `]` or `j` do, then no such option exists anywhere in the
  UI — the bindings in R7.1/R8 are the only ones available.
