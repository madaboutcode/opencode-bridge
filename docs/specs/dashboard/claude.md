# Dashboard — Claude Monitoring (Hook Ingress)

## Purpose

How the dashboard monitors Claude Code as a second, opt-in, read-only
harness. Claude is the only harness in this spec tree that the dashboard
cannot poll: the integration seam is a command hook the user adds to their
own Claude settings, which forwards a strict, bounded, metadata-only
lifecycle envelope over a local Unix socket. This file specifies what a
user observes when they configure, use, and remove that capability, what
the dashboard will and will not claim about Claude sessions, and the exact
privacy boundary on what may cross from a hook into the dashboard. It is
the sixth dashboard spec file; the other five (`overview.md`, `client.md`,
`layout.md`, `visuals.md`, `interactions.md`) describe the shared
dashboard those sessions render into (`docs/specs/CLAUDE.md`'s File
organization table).

Source: `tasks/2026-09-03-claude-dashboard-support.plan.md` (Claude sections),
evidence baseline `tasks/spikes/2026-09-03-claude-dashboard-support/`
(EVIDENCE.md, redacted-schemas.md), and the T01 deferrals in
`tasks/2026-09-03-claude-dashboard-support/deferred.md`.

## Contents

- [Scope](#scope) — what this file covers and what it explicitly does not
- [Manual opt-in hook configuration](#manual-opt-in-hook-configuration) — R11
- [Removal and deactivation](#removal-and-deactivation) — R12
- [Event allowlist](#event-allowlist) — R13
- [Metadata-only privacy boundary](#metadata-only-privacy-boundary) — R14
- [Bounded versioned local socket ingress](#bounded-versioned-local-socket-ingress) — R15
- [One complete bounded envelope per event](#one-complete-bounded-envelope-per-event) — R15.1
- [Best-effort degraded delivery](#best-effort-degraded-delivery) — R16
- [Completeness boundary and the authenticated end-to-end gate](#completeness-boundary-and-the-authenticated-end-to-end-gate) — R17

No child spec files — this is a leaf file; see `docs/specs/CLAUDE.md`'s
File organization table for the full six-file map.

## Scope

Covered: what the user must do for Claude monitoring to exist at all
(R11), how it is turned off (R12), which Claude hook events are accepted
(R13), what data may cross from Claude into the dashboard and what may
never (R14), the local socket and envelope contract (R15-R15.1), behavior
when the dashboard is absent or overwhelmed (R16), and the documented
completeness limits pending the authenticated end-to-end gate (R17).

Not covered: how Claude sessions are turned into dashboard tiles or
attention states after they cross the boundary (that is the adapter's
mapping, fed only identity, project, and lifecycle metadata by this file's
R13/R14; presentation follows the shared rules in `visuals.md`
and `layout.md`); the operation of the dashboard's own listener process;
and anything about the authenticated, real-session Claude flow, which is
deferred to T05 (see R17).

## Manual opt-in hook configuration

- **R11** — Claude monitoring is opt-in and manual: the dashboard shows a
  Claude session only after the user configures Claude Code's command hooks
  to run the dashboard's hook helper on Claude lifecycle events. The helper
  is the standalone command `dashboard claude-hook`: each hook entry binds
  one supported event (R13) to that command, and Claude runs the command
  with that event's payload on stdin whenever the event fires. The user
  chooses the scope — Claude Code's user-level or project-level hook
  settings — and nothing in the system ever installs, writes, edits, or
  registers those entries; neither starting nor running the dashboard
  creates, modifies, or reads any Claude settings file. In normal mode the
  dashboard opens the listener the helper writes to (R15-R16) before its
  adapters start, so a configured hook's events reach the dashboard. With
  no hooks configured, the dashboard is a pure OpenCode dashboard: no Claude
  session ever appears in it and nothing about Claude is touched.

  Scenario: Given a user has never configured Claude hooks, when they start
  the dashboard and use Claude Code normally, then the dashboard shows only
  OpenCode sessions and no Claude configuration file is created or modified
  — the hooks are the sole switch, and they are off.

## Removal and deactivation

- **R12** — Removing the hook entries a user added (or uninstalling the
  dashboard) disables Claude monitoring completely and permanently for that
  user's configuration. Because the dashboard never wrote those entries,
  the only user action is deleting them: with no entry invoking
  `dashboard claude-hook` (R11), no event is ever forwarded, there is no
  cleanup step and no repair, and no leftover setting silently re-enables
  forwarding later. When the dashboard stops, it closes its listener and
  removes the listener's socket (R16), so a removed capability also leaves
  no stale socket behind.

  Scenario: Given a user has configured hooks and then deletes them from
  their Claude settings, when they continue using Claude Code, then the
  dashboard receives no Claude events — nothing is forwarded, nothing is
  retained to replay later, and no file exists that could re-enable
  monitoring by itself.

## Event allowlist

- **R13** — From a hook payload, the dashboard accepts **exactly three
  events**: `SessionStart`, `StopFailure`, and `SessionEnd`. Every other
  event name — `UserPromptSubmit`, `PreToolUse`, `PostToolUse`,
  `PermissionRequest`, `Notification`, `Stop`, `SubagentStart`,
  `SubagentStop`, `CwdChanged`, or anything not listed — is ignored as a
  no-op: it is not forwarded, not retained, and nothing about the
  dashboard's view changes. This allowlist is the conservative set observed
  in the T01c spike baseline
  (`tasks/spikes/2026-09-03-claude-dashboard-support/EVIDENCE.md`).

  [REVIEW: all other events are unverified, not rejected by design —
  observation was impossible without an authenticated session (EVIDENCE.md,
  "Authenticated Scenario Status"). Whether `UserPromptSubmit`,
  `PreToolUse`, `PostToolUse`, `PermissionRequest`, `Notification`, `Stop`,
  `SubagentStart`, `SubagentStop`, or `CwdChanged` can be supported — and
  which non-sensitive labels they would carry — requires T05's
  authenticated end-to-end gate (see `deferred.md`, deferral 1). Until
  then these events stay dropped.]

  Scenario: Given a configured hook fires a `UserPromptSubmit` event, when
  the hook helper processes it, then nothing is sent over the socket and
  the dashboard's view of that session is unchanged — the event is
  a silent no-op.

  [REVIEW: `StopFailure`'s observed value field (`error`) is sensitive and
  discarded (R14); no bounded non-sensitive error classification was
  observed in the T01c baseline, so `StopFailure` forwards identity and
  event name only. A bounded error-type label is possible only after T05
  provides evidence about its schema.]

## Metadata-only privacy boundary

- **R14** — From an accepted event, only the following may cross from the
  hook into the dashboard, by exact field: `session_id`, `cwd`, the event
  name (`hook_event_name`), that event's allowlisted metadata (`source` for
  `SessionStart`, `reason` for `SessionEnd`), and `received_at` — the local
  time the dashboard received the event, never a Claude timestamp.
  Everything else in the payload — prompt text, assistant text,
  `transcript_path`, `agent_transcript_path`, `last_assistant_message`,
  tool input, tool output, `error` details, secrets, and any field not
  listed here — is discarded before anything leaves the hook helper. Those
  rejected values never appear in the delivered envelope, in any log line,
  in retained state, or anywhere Claude's output could surface them;
  rejecting them never depends on reading them.

  Scenario: Given a `SessionStart` payload contains a transcript path, an
  assistant message, and an unknown secret field, when the hook helper
  parses and forwards the event, then the delivered envelope contains
  only the session id, working directory, event name, source, and receipt
  time — and none of the transcript path, assistant text, or secret appears
  in the envelope, in logs, or in anything the dashboard retains.

## Bounded versioned local socket ingress

- **R15** — Each accepted event is forwarded as one envelope: a single
  newline-delimited JSON object carrying a protocol version (`1` in this
  release) plus the allowlisted record, and the socket it is sent to is a
  Unix socket scoped to the same user — an explicit per-user path, or a
  per-user location such as the user's runtime directory or home — never a
  shared, system-wide fallback. The limits are hard: a hook payload larger
  than 64 KiB is dropped whole; a session id longer than 128 UTF-8 bytes, or
  a working directory longer than 4096 UTF-8 bytes, drops the whole event;
  and a serialized envelope is at most 8 KiB. Nothing partial is sent and
  nothing is truncated to fit — exceeding any bound means the event is
  dropped.

  Scenario: Given a hook payload whose session id is longer than the
  128-character bound, when the hook helper parses it, then the entire
  event is dropped and nothing is written to the socket.

### One complete bounded envelope per event

- **R15.1** — A valid payload (all of R15's bounds satisfied) arrives as
  exactly one complete envelope: the listener receives exactly one line — a
  versioned JSON envelope ending in a newline and within the R15 size bound —
  never a partial write, never a truncated line, and never a second line.
  Nothing about the envelope's one-per-event shape depends on which
  allowlisted event it carries.

  Scenario: Given a valid hook payload, when the hook helper sends it, then
  the listener receives exactly one line: a versioned JSON envelope ending
  in a newline.

## Best-effort degraded delivery

- **R16** — The hook helper never delays, blocks, or fails Claude. If the
  dashboard's listener is absent, unreachable, busy, or full, the helper
  finishes successfully without sending, and Claude continues exactly as if
  the hook had not fired. The entire delivery attempt — from resolving the
  user-scoped socket path through checking the socket file to connecting
  and writing — runs on a single deadline of at most half a second, and the
  helper gives up as soon as that budget expires, so a hook never waits
  more than half a second for delivery and no part of the attempt (metadata
  checks included) can run past it. A malformed event, an unknown event, or
  a dropped record has the same effect: success, silence, no impact.
  Concurrent short-lived hooks are independent — one slow or failing helper
  cannot affect another session's delivery.

  The listener side is equally non-blocking and bounded. Normal dashboard
  startup binds the listener before any adapter starts; if no user-scoped
  socket path can be established (R15) or binding fails, Claude monitoring
  is simply unavailable and the OpenCode dashboard continues normally. The
  listener serves short-lived concurrent hook connections up to a fixed
  bound; a connection that is malformed, carries an unknown version or
  event, is out of bounds, oversized, unterminated, silent past its read
  deadline, or carries more than one frame is dropped without affecting any
  later connection. When the dashboard stops, the listener is shut down and
  its socket is removed (best effort), so a later dashboard takes the path
  over cleanly.

  Scenario: Given the dashboard is not running when a configured hook
  fires, when the hook helper runs, then it exits successfully and quickly
  and the user's Claude session proceeds with no error and no delay —
  nothing about the absence is visible to the user.

## Completeness boundary and the authenticated end-to-end gate

- **R17** — The dashboard claims no completeness for Claude sessions: it
  shows only sessions the configured hooks delivered while the dashboard
  was running. A session that started before the dashboard, or while the
  dashboard was down, is absent until a later observed event for it
  arrives — and if none ever arrives, it never appears. The dashboard never
  scans transcripts, history, or session lists to fill that gap. Wiring the
  listener into dashboard startup (T04) changes none of this: the
  capability remains discovery-less, replay-less, and authenticated-gated.
  Authenticated successful-turn behavior (event ordering, async-hook
  viability, exit-path reliability, subagent identity) is explicitly not
  part of this spec's guarantees until T05's end-to-end gate provides it.

  [REVIEW: T05 must run an authenticated real Claude flow through hook,
  helper, socket, adapter, and dashboard and record the four deferred
  evidence areas (`deferred.md`): authenticated lifecycle ordering,
  startup-gap/foreground discovery, async-hook viability for successful
  sessions, and exit-path reliability plus subagent identity. Until then
  this spec guarantees exactly the three observed events (R13), the
  no-replay boundary above, and metadata-only exposure (R14) — nothing
  more.]

  Scenario: Given a Claude session starts while the dashboard is not
  running, when the dashboard starts later, then that session does not
  appear until a later observed hook event for it arrives — and if no such
  event ever arrives, it never appears, with no transcript or history scan
  ever performed.
