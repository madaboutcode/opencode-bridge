# Dashboard — Claude Monitoring (Hook Ingress)

## Purpose

How the dashboard monitors Claude Code as a second, opt-in, read-only
harness. Claude is the only harness in this spec tree that the dashboard
cannot poll: the integration seam is a command hook the user adds to their
own Claude settings, which forwards a strict, bounded lifecycle-and-activity
envelope over a local Unix socket — the tool being called, its arguments and
result, the prompt and final assistant text, and subagent identity, each
individually size-bounded, never the raw transcript. This file specifies
what a user observes when they configure, use, and remove that capability,
what the dashboard will and will not claim about Claude sessions, and the
exact boundary on what may cross from a hook into the dashboard. It is
the sixth dashboard spec file; the other five (`overview.md`, `client.md`,
`layout.md`, `visuals.md`, `interactions.md`) describe the shared
dashboard those sessions render into (`docs/specs/CLAUDE.md`'s File
organization table).

Source: `tasks/2026-09-03-claude-dashboard-support.plan.md` (Claude sections),
evidence baseline `tasks/spikes/2026-09-03-claude-dashboard-support/`
(EVIDENCE.md, redacted-schemas.md), and the T01 deferrals in
`tasks/2026-09-03-claude-dashboard-support/deferred.md`. R13/R14/R15/R17 were
revised on 2026-09-05 against the published Claude Code hooks reference
(https://code.claude.com/docs/en/hooks, fetched 2026-09-05) and a live capture
from this repository's own Claude session, which resolves the R13 `[REVIEW]`
question the T01c baseline could not: every field this spec now allowlists
is a documented field on the named event's hook payload, not an inference.

## Contents

- [Scope](#scope) — what this file covers and what it explicitly does not
- [Manual opt-in hook configuration](#manual-opt-in-hook-configuration) — R11
- [Removal and deactivation](#removal-and-deactivation) — R12
- [Event allowlist](#event-allowlist) — R13
- [Bounded activity fields](#bounded-activity-fields) — R14
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
attention states after they cross the boundary — that is the adapter's
mapping, fed only identity, project, and the bounded activity fields this
file's R13/R14 allow through, with presentation following the shared rules
(see dashboard/visuals.md R6) and (see dashboard/layout.md R5.3); the
operation of the dashboard's own listener process; and anything about the
authenticated, real-session Claude flow, which is deferred to T05 (see
R17).

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

- **R13** — An event is accepted only if it answers one of three questions a
  session tile exists to answer: **is it alive and what is it doing**
  (`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`,
  `SubagentStart`), **does it need me, and has that cleared**
  (`PermissionRequest`, `PermissionDenied`, `Elicitation`,
  `ElicitationResult`, `Notification`), or **did it finish, and how**
  (`Stop`, `StopFailure`, `SessionEnd`, `SubagentStop`,
  `PostToolUseFailure`). That is **fifteen events**. `Notification` sits in
  the second bucket because its documented `notification_type` values
  include attention-worthy states (`agent_needs_input`, `permission_prompt`,
  `idle_prompt`) even though its own sub-type set overlaps the dedicated
  permission/elicitation events above — R14 admits the label for that
  reason. The adapter maps `idle_prompt` to a needs-you state without the
  question flag, and maps `permission_prompt` and `agent_needs_input` to a
  needs-you state with the question flag. An absent or unrecognized subtype
  leaves the session's existing attention state unchanged; the mapping remains
  adapter-owned, outside this file's implementation scope (see Scope above).
  Every other
  event name is ignored as a no-op: not forwarded, not retained, nothing
  about the dashboard's view changes. Excluded events fail the same three
  questions, not an unstated one:

  - `CwdChanged` — redundant, not missing. Every accepted event already
    carries the session's current `cwd` (R14); a change is observable on
    the next event of any kind without a dedicated one. (Project-region
    grouping must re-derive from each event's own `cwd`, never freeze on
    the first-seen value, or this redundancy argument does not hold.)
  - `MessageDisplay`, `PostToolBatch`, `FileChanged` — excluded by a rate
    rule, not a semantic one: R16's cost model assumes one hook process per
    discrete lifecycle moment. A per-token or per-batch event would change
    the process-spawn rate by an order of magnitude and break that model.
    Any future event that fires more than once per tool call or turn is
    excluded on this same rate ground before its semantics are even
    considered.
  - `UserPromptExpansion`, `TaskCreated`/`TaskCompleted`, `ConfigChange`,
    `InstructionsLoaded`, `DirectoryAdded`, `WorktreeCreate`/
    `WorktreeRemove`, `PreCompact`/`PostCompact`, `PreModelSwitch`/
    `PostModelSwitch`, `Setup` — none answers any of the three questions:
    they describe environment or configuration state, not session activity
    or attention.
  - `TeammateIdle` — **[REVIEW: OPEN]** this is the needs-you signal for
    Claude's agent-team/teammate model, which this repository's own Claude
    sessions actually use, so excluding it risks missing that case
    entirely. Not decided here because the published hooks reference does
    not state whether a teammate fires under its own `session_id` or the
    lead's (everything in R15 keys on `session_id`; if teammates share the
    lead's id there is no slot to hold separate teammate state). Deferred
    to the live-proof phase (R17) to observe directly rather than guess.

  Scenario: Given a configured hook fires a `PreToolUse` event, when the
  hook helper processes it, then an envelope carrying that tool's name and
  bounded argument preview is forwarded — see R14.

  Scenario: Given a configured hook fires a `Notification` event with
  `notification_type: permission_prompt`, when the hook helper processes it,
  then the session is represented as needing the user's attention with the
  question indicator; an absent or unrecognized subtype leaves its existing
  attention state unchanged.

  Scenario: Given a configured hook fires a `WorktreeCreate` event, when the
  hook helper processes it, then nothing is sent over the socket and the
  dashboard's view of that session is unchanged — the event is a silent
  no-op.

  Scenario: Given a `PreToolUse` for a tool a policy denies, when the hook
  helper receives the resulting `PermissionRequest` and then
  `PermissionDenied`, then the dashboard shows the session needing
  attention after the first event and no longer needing it after the
  second — the tile never gets stuck asking for a decision that already
  happened.

## Bounded activity fields

- **R14** — From an accepted event, the following may cross from the hook
  into the dashboard, by exact field — every field named here is a
  documented field on that event's own hook payload (see the Source note
  above); nothing here is inferred:

  | Event | Fields carried |
  |---|---|
  | `SessionStart` | `source` (closed set), `model` (label) |
  | `UserPromptSubmit` | `prompt` (bounded) |
  | `PreToolUse` | `tool_name` (label), `tool_use_id` (label), `tool_input` (bounded, serialized), `agent_id` (label), `agent_type` (label) |
  | `PostToolUse` | `tool_name`, `tool_use_id`, `tool_input` (bounded), `tool_response` (bounded), `agent_id`, `agent_type` |
  | `PostToolUseFailure` | `tool_name`, `tool_use_id`, `tool_input` (bounded), `error` (bounded), `error_type` (label), `agent_id`, `agent_type` |
  | `PermissionRequest` | `tool_name` (label), `tool_input` (bounded), `tool_use_id` (label) |
  | `PermissionDenied` | `tool_name` (label), `tool_use_id` (label), `denial_reason` (bounded) |
  | `Notification` | `notification_type` (label), `notification_message` (bounded) |
  | `Stop` | `last_assistant_message` (bounded), `agent_id` (label), `agent_type` (label) |
  | `StopFailure` | `error_type` (label) |
  | `SubagentStart` | `agent_id` (label), `agent_type` (label), `agent_prompt` (bounded) |
  | `SubagentStop` | `agent_id` (label), `agent_type` (label), `last_assistant_message` (bounded), `stop_reason` (label) |
  | `Elicitation` | `tool_use_id` (label), `server_name` (label), `elicitation_request` (bounded) |
  | `ElicitationResult` | `tool_use_id` (label), `server_name` (label), `user_response` (bounded) |
  | `SessionEnd` | `reason` (closed set) |

  `PermissionRequest`'s needs-you state clears on **any subsequent accepted
  event carrying the same `tool_use_id`** — `PermissionDenied`,
  `PostToolUse`, or `PostToolUseFailure` all qualify, whichever arrives.
  This file does not fix the exact ordering Claude uses between the
  permission decision and the tool's own `PreToolUse`/`PostToolUse` pair —
  the live-proof phase (R17) records that ordering as evidence — but the
  clearing rule holds regardless of which specific event comes next, so it
  cannot be broken by an ordering assumption turning out wrong.
  `Elicitation`/`ElicitationResult` clear the same way, one `tool_use_id` at
  a time. An accepted `PermissionRequest` or `Elicitation` with no matching
  `tool_use_id` ever appearing again is a known, accepted gap — R17 already
  claims no completeness guarantee for any Claude session.

  Every event also carries `session_id`, `cwd`, the event name
  (`hook_event_name`), and `received_at` — the local time the dashboard
  received the event, never a Claude timestamp. `transcript_path` and
  `agent_transcript_path` are never read by name and never cross this
  boundary under any event — the dashboard does not follow them to the
  transcript (see R17). Any field not named in the table above, and any
  unrecognized field on a named event, is discarded before anything leaves
  the hook helper and never appears in the delivered envelope, in any log
  line, in retained state, or anywhere Claude's output could surface it.

  A field marked "(bounded)" is UTF-8 byte-capped per R15 rather than
  dropped whole: content past the cap is cut at a valid UTF-8 boundary and
  the field carries a trailing truncation marker. `tool_input` is the
  tool's argument object serialized to compact JSON text before the same
  cap applies — the dashboard never parses it back into structured data.

  A field marked "(label)" (`tool_name`, `tool_use_id`, `agent_id`,
  `agent_type`, `notification_type`, `error_type`, `stop_reason`,
  `server_name`, `model`) is an opaque, open-ended value validated only
  against the R15 length bound, never against a fixed value set, because
  tool and agent names come from an open, user-extensible set (an MCP tool
  name, for example) and a documented "e.g." example list is not a closed
  one. `denial_reason` is "(bounded)" free text, not a label — it is
  Claude's own explanation of why a tool was denied, not a short opaque
  code. Only `source`
  (`SessionStart`) and `reason` (`SessionEnd`) are closed sets, validated
  against the specific documented values already listed under R13's
  history; every other label is length-only. A label over its length bound
  drops the whole event, the same as an invalid `session_id` or `cwd`
  (R15); a bounded free-text field over its length is truncated, never
  dropped (above).

  Allowlisted content — every field in the table above, once it has
  crossed the boundary — is treated as sensitive for as long as the
  dashboard holds it, and that "as long as" is bounded: it exists only in
  memory, only for the lifetime of the tracked session it belongs to.
  `SessionEnd`/`SubagentStop` (R13) already remove that session's tracked
  state as a tombstone (no separate retention step is needed or added by
  this requirement); the session's content does not outlive that removal.
  This bound has a known gap, not a hidden one: if `SessionEnd` is never
  observed — the Claude process crashes, or the event is one of the ones
  R16's rate paragraph drops under load — that session's content is held
  for as long as the dashboard keeps running, the same discovery-less gap
  R17 already claims for the session's existence generally. No independent
  time- or idle-based eviction exists for Claude content today.
  While tracked, it is never written to a log line (the existing
  category-only logging discipline applies to accepted content exactly as
  it already applies to rejected content), never persisted to disk, and
  never included in a crash report or panic message — this is a claim
  about the dashboard's behavior that a QA pass should confirm directly
  (grep for any state-dump/snapshot-to-disk path and prove Claude content
  is excluded from it) rather than take on the strength of this sentence
  alone. This replaces the previous "metadata-only" guarantee, which is no
  longer this file's privacy position — content now crosses by design —
  but the fields that still never cross under any event are unchanged:
  `transcript_path` and `agent_transcript_path` are never read by name and
  never appear anywhere in the dashboard (see below), and any field not
  named in the table above is discarded the same way it always was.

  Scenario: Given a `PostToolUse` payload whose `tool_response` is 40 KiB of
  file content, when the hook helper parses and forwards the event, then
  the delivered envelope carries the tool name and a `tool_response` field
  truncated to the R15 per-field bound with a truncation marker — the event
  is still forwarded, not dropped.

  Scenario: Given a `SessionStart` payload contains a `transcript_path` and
  an unknown field, when the hook helper parses and forwards the event,
  then the delivered envelope contains `source`/`model` plus the common
  fields, and neither the transcript path nor the unknown field appears in
  the envelope, in logs, or in anything the dashboard retains.

## Bounded versioned local socket ingress

- **R15** — Each accepted event is forwarded as one envelope: a single
  newline-delimited JSON object carrying a protocol version (`1` in this
  release) plus the allowlisted record, and the socket it is sent to is a
  Unix socket scoped to the same user — an explicit per-user path, or a
  per-user location such as the user's runtime directory or home — never a
  shared, system-wide fallback. Two different kinds of limit apply, and
  they never both reject the same oversized content:

  - **Whole-event drops** (nothing sent): a hook payload larger than 2 MiB
    (raised from 64 KiB, then 256 KiB — a bounded field's *raw*, untruncated
    size can legitimately be several hundred KiB, e.g. a `Read` of a large
    file, and the parser must be able to read a payload that big before it
    can truncate anything out of it); a session id longer than 128 UTF-8
    bytes; a working directory longer than 4096 UTF-8 bytes; or any R14
    "(label)" field over 256 UTF-8 bytes. Above 2 MiB is a residual, rare
    case (a very large minified file or JSON tool result), not the common
    path R14's truncation is designed for; the tile's behavior when this
    drop happens is the same as any other dropped event under R16's rate
    paragraph below — `current_action` lags by one tool call, never a
    corrupted or wrong tile.
  - **Truncation, not a drop** (event still sent): any R14 "(bounded)" field
    is capped at 4096 UTF-8 bytes, cut at that boundary with a truncation
    marker — this is the *only* thing the 2 MiB raw-payload limit and the
    24 KiB envelope limit (raised from 8 KiB — the largest event,
    `PostToolUse`, carries two bounded fields plus labels) exist to make
    room for. A payload's raw content field being oversized is never by
    itself a reason the event is dropped whole.

  Nothing partial is ever sent for a whole-event drop.

  Scenario: Given a hook payload whose session id is longer than the
  128-UTF-8-byte bound, when the hook helper parses it, then the entire
  event is dropped and nothing is written to the socket.

  Scenario: Given a `PostToolUse` payload whose raw, undelivered
  `tool_response` is 300 KiB (well under the 2 MiB whole-payload limit, well
  over the 4096-byte field cap), when the hook helper parses it, then the
  event is accepted and forwarded with `tool_response` truncated to 4096
  bytes — the 300 KiB size alone never triggers a whole-event drop.

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

  **Rate.** R13's allowlist (`PreToolUse`/`PostToolUse` per tool call) means
  a session doing many small tool calls in quick succession — a bulk edit
  run, for example — forks one hook process per event, each opening a
  connection to the listener. The listener accepts at most 8 connections at
  once; a connection beyond that bound waits for a free slot within the
  same per-hook delivery deadline above, and if none frees up in time it is
  dropped exactly like an absent listener — same outcome, same harmlessness
  to Claude. This is where the "never delays... Claude continues exactly as
  if the hook had not fired" language above stops being the typical case:
  a hook caught behind a full bound can wait close to the full half-second
  deadline before giving up, not near-zero time — still within the stated
  bound, but the flood path is measurably slower than the ordinary one, not
  identical to it. On the dashboard side, a burst that drops some events
  never corrupts a tile: every accepted event replaces the tracked
  session's whole state (never an incremental diff), so the visible
  consequence of a dropped event under load is a `recent_actions` entry
  that never appears or a `current_action` that lags by one tool call —
  never a wrong or inconsistent tile.

  Scenario: Given a session fires 20 `PreToolUse`/`PostToolUse` events
  within one second, when more than 8 of their hook processes are
  connecting at once, then the excess wait for a slot and the slowest are
  dropped after the delivery deadline — the tile shows a `recent_actions`
  list that may be missing an entry, never a stale or corrupted snapshot.

## Completeness boundary and the authenticated end-to-end gate

- **R17** — The dashboard claims no completeness for Claude sessions: it
  shows only sessions the configured hooks delivered while the dashboard
  was running. A session that started before the dashboard, or while the
  dashboard was down, is absent until a later observed event for it
  arrives — and if none ever arrives, it never appears. The dashboard never
  scans transcripts, history, or session lists to fill that gap, and never
  opens `transcript_path`/`agent_transcript_path` even though every event
  carries it (R14) — activity comes from the hook payload only, never from
  reading the session's own recorded output. Wiring the listener into
  dashboard startup changes none of this: the capability remains
  discovery-less and replay-less.

  [REVIEW: the R13 event/field set is now evidence-backed against the
  published hooks reference and a live capture from this repository's own
  session (see the Source note), which resolves T01c's original blocker —
  no authenticated session was available to observe. What remains open is
  operational, not schema: authenticated lifecycle ordering under a real
  registered hook, startup-gap/foreground discovery, async-hook viability
  for successful (non-`StopFailure`) sessions, exit-path reliability, and
  whether `TeammateIdle` (R13) fires under a teammate's own `session_id` or
  the lead's — undocumented, and this repository's own sessions can
  exercise it directly. Closing this REVIEW requires registering the hook
  against a real session and recording that evidence — tracked as the
  live-proof step of the 2026-09-05 revision, not a further schema spike.]

  Scenario: Given a Claude session starts while the dashboard is not
  running, when the dashboard starts later, then that session does not
  appear until a later observed hook event for it arrives — and if no such
  event ever arrives, it never appears, with no transcript or history scan
  ever performed.
