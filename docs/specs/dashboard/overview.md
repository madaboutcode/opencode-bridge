# Dashboard — Overview

## Purpose

What the opencode dashboard is, its scope, and the conventions every sibling
spec in this tree builds on. Source: `tasks/2026-09-01-opencode-dashboard.requirements.md`.

## Contents

- [File map](#file-map) — which R-numbers live in which sibling file
- [Scope](#scope) — R1-R2
- [Data & active window](#data--active-window) — R3-R3.2
- [Real usage scale](#real-usage-scale) — R5.8
- [Non-goals (V1)](#non-goals-v1) — R10

Sibling spec files: [`client.md`](client.md), [`layout.md`](layout.md),
[`visuals.md`](visuals.md), [`interactions.md`](interactions.md),
[`claude.md`](claude.md) — see the File map table below for what each one
covers.

## File map

This spec tree covers the dashboard in six files. Start here; follow the
pointers below for detail this file doesn't carry.

| File | Covers |
|---|---|
| `overview.md` (this file) | R1, R1.1, R1.2, R1.3 (summary only), R2, R3-R3.2, R5.8, R10 |
| `client.md` | R1.3 (full contract), R1.4-R1.8, R4, R6.4-R6.6 |
| `layout.md` | R5-R5.11, R9-R9.2 |
| `visuals.md` | R6, R6.1-R6.3, R6.7, R6.8 |
| `interactions.md` | R7-R7.1, R8-R8.1 |
| `claude.md` | R11-R17 (the opt-in Claude-monitoring capability: hook ingress, privacy boundary, local socket, and the adapter's lifecycle mapping — listener wired into startup with T04; opt-in and experimental pending T05) |

## Scope

- **R1** — The dashboard is its own program, `dashboard`, separate from the
  existing `opencode-mcp` binary. The two share one library for talking to
  the opencode server (authentication, plus the session-list, session-message,
  and event-stream calls) so that logic isn't duplicated between them.

  Scenario: Given the repo is built, when you look at its output binaries,
  then both `dashboard` and the existing MCP binary exist independently, and
  both link the same shared client library rather than each having its own
  copy of the server-talking code.

- **R1.1** — The shared client library carries no TUI code and no MCP code.
  Only the dashboard binary carries the TUI dependencies; only the MCP binary
  carries the MCP dependencies. This keeps each binary's dependency footprint
  to what it actually needs.

  Scenario: Given the shared client library's own dependency list, when you
  inspect it, then it contains neither a TUI toolkit nor an MCP toolkit —
  those appear only in the dashboard's and the MCP binary's own dependency
  lists, respectively.

- **R1.2** — The dashboard talks to the opencode server directly, using the
  same local password file the server itself writes. It does not need the
  MCP process running to work. If session metadata happens to show the
  session was started via MCP, the dashboard shows a small "via MCP" tag as
  a bonus — but its absence changes nothing else about how a session is
  shown.

  Scenario: Given the MCP process is stopped, when you start the dashboard,
  then it still connects to the opencode server and shows live session data;
  no session shows a "via MCP" tag, since there's no MCP metadata to read.

- **R1.3 (summary)** — The dashboard's core — everything that tracks session
  state or decides what to render — does not talk to the opencode server's
  wire format directly. It sits behind a boundary called the `HarnessAdapter`:
  each coding-agent tool the dashboard can watch ("harness") gets its own
  adapter, and only the adapter knows how that harness's wire protocol works.
  V1 ships the opencode adapter; a second, **opt-in** Claude hook adapter
  (`claude.md` R11-R17) is implemented on the same boundary with its Unix
  listener wired into dashboard startup (T04); it remains opt-in and
  experimental pending T05's authenticated evidence. The core is
  written so any harness can be added without changing how the core works.
  The full contract for this boundary — what an adapter must
  produce, how sessions and projects are identified, how staleness is
  handled — lives in `client.md` (R1.3 full, R1.4-R1.8). This file only
  establishes that the boundary exists.

  Scenario: Given a session snapshot arriving from the opencode adapter, when
  the dashboard's core processes it, then the core reads only the shared
  snapshot shape (title, status, current action, etc.) and never touches an
  opencode-specific field (like a raw SSE event or tool name) directly — see
  `client.md` for what that shared shape contains.

  [REVIEW: T04 wires the Unix listener into dashboard startup; Claude
  monitoring is still opt-in (active only after the user configures hooks,
  `claude.md` R11-R12), and T05 retains the final stale-session policy
  (`client.md` R1.7) and the authenticated Claude lifecycle evidence
  (`claude.md` R17).]

- **R2** — The dashboard follows standard terminal-app engineering practice:
  it takes over the full terminal screen while running and always restores
  the terminal (cursor, screen mode) on exit, even if it crashes; it reacts
  to terminal resize; and it responds to keyboard input within about 250ms
  without busy-waiting the CPU.

  Scenario: Given the dashboard is running full-screen, when the process
  exits normally, is killed, or panics, then the terminal is left in the
  same state it was before the dashboard started (cursor visible, normal
  screen buffer, normal input mode) — not stuck in raw/alternate-screen mode.

## Data & active window

- **R3** — A session counts as "active" based on how long ago the dashboard
  last received an update for it, not on any single harness's own notion of
  "last updated." A session is active if that time is within a configurable
  window, `W`.

  Scenario: Given a session's last update arrived 5 minutes ago and `W` is
  10 minutes, when the dashboard evaluates that session, then it is
  classified active.

- **R3.1** — `W` defaults to 10 minutes and can be changed from the keyboard
  while the dashboard is running (keys specified in `interactions.md` R8).

  Scenario: Given the dashboard has just started with no window change made,
  when you check the active window, then it reads 10 minutes; when you then
  press a window-control key, then the window changes accordingly.

- **R3.2** — A session outside window `W` is "idle," not gone — it still
  exists on the server. An idle session is never silently dropped from view
  if its project has at least one active session; it's shown as context
  instead (exact presentation is `layout.md`'s concern, R5.2/R5.6).

  Scenario: Given a project with one active session and one session last
  updated 25 minutes ago (`W` = 10 minutes), when the dashboard renders that
  project, then the idle session is still visible somewhere in that
  project's display, not hidden.

## Real usage scale

- **R5.8** — Real usage is small: typically 2 sessions per project, up to 4
  projects at a time (around 8 sessions total). This is the scale every
  layout and interaction tradeoff in this spec tree is actually designed
  for — not the 10-50+ session case some earlier exploratory designs assumed.
  The dashboard must not break badly at larger scale, but larger scale is a
  secondary check, not the design target. Full layout consequences are in
  `layout.md` (R5.1, R5.6).

  Scenario: Given 4 projects open with 2 sessions each (8 sessions total),
  when the dashboard lays out the screen, then this is the scale its design
  is validated against — not a stress case treated as secondary.

## Non-goals (V1)

- **R10** — The following are out of scope for V1: cost/token analytics, git
  diff or history views, pagination beyond the live window, subagent nesting
  more than one level deep, animated (as opposed to instant) layout reflow,
  and any interaction that requires a mouse. Sizing tiles by status (e.g.
  making a stalled session's tile bigger than a running one) is a V2 idea,
  not built now.

  Scenario: Given the dashboard running in V1, when you look for token/cost
  totals, a diff view, or any control that only works with a mouse, then
  none of these exist — every dashboard action is reachable from the
  keyboard alone (see `interactions.md`).
