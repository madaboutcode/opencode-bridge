# opencode Dashboard — Layout Brainstorm Brief

Status: OPEN — this document exists to hand the layout question to other agents/people for
brainstorming. It is not itself a decision. The winning idea gets written back into
`tasks/2026-09-01-opencode-dashboard.requirements.md` §3 (R5).

Full requirements: `tasks/2026-09-01-opencode-dashboard.requirements.md`
Wire-signal ground truth: `docs/internal/opencode-sse-event-catalog-2026-09-01.md`,
`tasks/spikes/2026-09-01-status-signals.md`
Working chrome prototype (card styling, NOT layout — see below): `tmp/dashboard-chrome-prototype/`
Retired treemap code (for reference only, do not extend): `tmp/dashboard-spike/src/squarify.rs`

## 1. The question this brief is asking

Earlier today the layout model (R5) was decided as a "grouped flow grid": project boxes
sized to tightly fit their session cards, cards fixed-size by content, no proportional
area anywhere. The user has now rejected that outcome:

> "not happy with this - it is not doing the masonry - ie; use the full area - boxes scale
> and stack based on available area - that way I can see things better - think of those
> disk space WinDirStat tool display - but we only have two levels - project > session"

So the layout model is reopened. The ask: an area-proportional masonry/treemap-style
layout, like WinDirStat (disk usage visualizer — box area ∝ size, boxes tile to fill the
whole available rectangle with no dead space), restricted to exactly two nesting levels
(project contains sessions; no third level of visual nesting).

A second, related question was raised in the same breath and is NOT yet answered:

> "that reminds me - what about subagents in a session - i want to be able to see it too."

Both questions go to the same brainstorm because they interact: a subagent is spawned
*from* a session, and whether it gets its own box, nests inside its parent's box, or
becomes a third implicit level changes what "two levels" actually means.

## 2. Why this was already tried once and killed — read this before proposing area-proportional sizing again

This is not a fresh idea landing on blank ground. The original `tmp/dashboard-spike`
(screenshot the user reacted to at the start of this whole design session) used exactly
this approach: `squarify.rs`, a squarified treemap (Bruls et al.), box area ∝ a `weight`
field per item. The user's first reaction to that screenshot was that it was cluttered
and wasted space. Working through it with the advisor produced this diagnosis (verbatim
from requirements.md R5.1):

> "a 1-session project outsizing a 3-session project by weight is a misleading signal, not
> a coarse-useful one; card count already shows 'more going on' honestly."

and R5's own rationale for dropping it entirely:

> "once session cards are fixed-size [to fit 3 lines of text], there is no continuous area
> left for a treemap to divide."

Concretely, the failure mode was: a box gets sized proportional to some weight, but the
box's *content* is a fixed amount of text (title, status, current action). If area is
large and content is fixed-size, you get dead whitespace — the "big empty rectangle"
problem, which is what made the first screenshot look bad in the first place. If area is
small (a low-weight session next to a high-weight one), the same fixed text either
overflows or has to be dropped, which contradicts R5.2's "reduce clutter... see more of
the data... to make a judgment call" mandate — the whole reason cards were made bigger
and content-driven earlier today.

**So the actual brainstorm problem is not "should we do masonry" — it's "how do we get
WinDirStat's at-a-glance magnitude signal without recreating the empty-rectangle-or-
overflowing-text problem that got this retired once already."** Options that don't
address that tension directly (e.g. "just turn squarify back on") are not new answers,
they're the same answer that was already rejected — flag that explicitly if a proposal
amounts to this.

Some directions worth having agents actually think through, not because they're the
answer but because they're different bets on where the tension resolves:

- Decouple magnitude from text layout: area still encodes weight, but a box's *content*
  scales too (e.g. small boxes show a color chip only, larger boxes progressively reveal
  status line, then action line — WinDirStat itself does this, small files are just a
  colored rectangle with no label). Caveat worth having agents sit with: WinDirStat's
  domain is a real filesystem tree of arbitrary depth with a physical size unit (bytes)
  at every node. This dashboard is a fixed 2-level hierarchy with no physical unit —
  the analogy may not transfer as directly as it first looks, and a proposal leaning on
  it should say explicitly what does and doesn't carry over.
- Keep fixed-size cards (today's decided content model, R6.3) but add a *separate*
  magnitude cue that isn't box area — thickness of a border, a small bar/sparkline in the
  card, saturation of a background fill — so "how big is this" is visible without the box
  itself changing shape.
- Two different sizing rules per level: sessions stay fixed/content-sized (keep R5.2 as
  is), but *project* boxes size proportionally to session count or aggregate weight, since
  a project box's content is already just "however many cards fit" — there's more genuine
  slack at that level than at the card level.
- Something else entirely — the point of a brainstorm is not to be anchored to the three
  bullets above.

## 3. What "weight" would even mean here

WinDirStat's size is bytes: unambiguous, additive, already known before you draw anything.
A dashboard session has no equivalent single obvious number. Candidates, all live on the
wire per the SSE catalog and status-signals spike:

- `cost` (USD, cumulative, from `session.usage.updated` / REST `cost` field) — ties box
  size to money spent, which is legible but the non-goals list (R10) explicitly excludes
  cost/token analytics as a V1 goal; using cost to size boxes may reopen that.
- `tokens` (input/output/reasoning/cache, cumulative) — same caveat as cost.
- wall-clock elapsed since `time.created` or since last activity — cheap, always present,
  but conflates "long-running and busy" with "long-running and stuck," which is exactly
  the ambiguity R6.7 already spent effort disambiguating for card status (running vs
  needs-you vs idle) via a 3-state model, not elapsed time alone.
- session/message count per project (already implicitly used as an ordering-only signal
  today, R5.6/R5.7) — coarse, but matches the "card count already shows more going on
  honestly" reasoning that killed weight-by-cost the first time.
- Some function of attention state itself (e.g. `needs-you` sessions "weigh more" so they
  literally take more screen space, not just sort first as today) — this would tie area to
  the exact thing R6.7 already tracks, rather than inventing a new metric.

Whichever metric a proposal picks, it must say so explicitly and defend why that number,
not just "weight" as an unexamined given — that vagueness is exactly what let R5.1 sail
through the first design pass before its consequences were noticed.

## 4. Subagent visibility — currently handled, but user flagged it unprompted, so treat as open

Current state (R5.6, already in the chrome prototype): a subagent's session shows as an
ordinary sibling card inside its parent's project box, prefixed `↳`, e.g. `↳ violet-lynx ·
Delegated…` sitting next to `golden-hawk` under project `web`. This treats "subagent" as
just another session, distinguished only by a glyph — not a third visual level.

What's unconfirmed and worth re-checking in this brainstorm:

- **Wire behavior** (confirmed, from the SSE catalog §3 `subagent`): a subagent call spawns
  a fully separate session with its own `sessionID`, never merged into the parent's message
  list. The *only* link back to the parent is that the parent's `tool.success.content`
  contains the child's final answer wrapped in `<subagent sessionID=... state=...>`. There
  is no live "child session started/progressing" event stream visible from the parent's
  side beyond the ordinary tool-call lifecycle (`tool.called` → `tool.progress` →
  `tool.success`) — `tool.progress` for a `subagent` call does carry `metadata.sessionID` +
  `metadata.status` while it's running, so a live child-session pointer does exist during
  the call, it's just not automatically resolved to project/title without a follow-up
  fetch.
- Does the user want the subagent's box to visually *nest inside* the parent card (a true
  3rd level, which they just said they don't want) or sit *next to* it but visually tied
  (arrow/connector/indent, today's `↳` approach)? "I want to be able to see it too" doesn't
  yet say which — that's a live question for the brainstorm, not decided by this brief.
  - **My current best read (not yet confirmed with the user): the user's "we only have two
    levels" line was said in the middle of describing the masonry ask and may have been
    about project/session, not a ruling on subagents — since the very next sentence raised
    subagents as something separately unresolved. Don't treat "two levels" as a settled
    constraint on subagent placement without checking.**
- Does a subagent's weight (whatever §3's metric turns out to be) roll up into the parent
  session's box, the project's box, both, or stay separate? This only matters once a weight
  metric and an area-proportional model are chosen — sequence the thinking accordingly.

## 5. What must NOT move — settled constraints from requirements.md any proposal must respect

- R1–R2: standalone `dashboard` binary, ratatui+crossterm, TUI best practices (alt screen,
  raw mode, panic-safe restore).
- R3/R3.1/R3.2: active-session window `W` (default 10m), idle sessions shown as context
  only inside a project with ≥1 active session.
- R6.3: session card content is 3 lines — title/handle, status+elapsed, current action.
  This can change if the winning layout genuinely needs it to (e.g. the "content scales
  with area" direction in §2), but changing it is a real decision to flag, not a silent
  side effect.
- R6.7: attention model is exactly 3 states — `running / needs-you / idle` — with
  `needs-you` having a question-badge sub-state. This is marked CONFIRMED FINAL and was
  reached only after directly confirming with the user that hangs/stalls don't happen in
  practice — do not reopen `stalled` without a similarly direct check.
- R6.8: nickname scheme (deterministic adjective+noun hash of session ID, both words ≤6
  chars, frozen word list, pure function, no cache/storage).
- Tokyo Night palette (see requirements.md R6 and the chrome prototype's `src/palette.rs`
  for exact hex values already in use).
- R9/R9.1/R9.2: zero-active empty state, `terminal too small` below `40×12`, degrade
  hierarchy readability > project presence > proportionality > 100% fill — note this
  existing principle already ranks proportionality (masonry's whole point) *below*
  readability and project presence. Any masonry proposal has to reconcile with this
  ordering or explicitly argue for changing it.
- R10 non-goals: cost/token analytics, git diff/history, animated reflow, mouse-required
  interaction are out of scope — a weight metric built on cost/tokens should flag that
  tension (§3) rather than ignore it.

## 6. Prototype discipline, if a proposal reaches the "build it" stage

The chrome-axis prototype earlier today was rejected once by the advisor for comparing
options as static text when the actual signal was color, and again for using an evenly-
split, sanitized fixture that would hide real packing problems. Same discipline applies
here, more so — treemap-style layouts are exactly the kind of thing that looks fine on
paper and breaks under a ragged real distribution:

- Any layout proposal that reaches implementation must be judged as a real ratatui/ANSI
  render in an actual terminal, not an ASCII sketch in a doc.
- Test fixture must be ragged (uneven project/session counts, at least one all-idle project
  adjacent to a busy one, at least one project past the overflow cap, at least one subagent
  card, adversarial max-length strings) and rendered at both a normal width and a narrow
  one (80 columns) — narrow width is where area-proportional layouts are most likely to
  produce unreadable slivers (see R5.5's minimum-size constraints, which still apply).

## 7. Questions this brainstorm needs to land on

1. What single number (or composite) does "weight"/size mean, and why that one (§3)?
2. How does area-proportional sizing coexist with the fixed 3-line card content model —
   does content scale with area, does a separate visual cue carry magnitude instead, or
   does something else resolve it (§2)?
3. Does the "two levels" constraint apply to subagents, or was that said only about
   project/session and subagents are still open (§4) — confirm with the user before
   building on this either way.
4. If subagents get area-proportional treatment too, whose box does their weight count
   toward?
5. How does the proposal keep R5.5's minimum-size floor and R9.2's readability-first
   degrade ordering intact once box sizes are no longer uniform?
