# Dashboard — Client / Harness-Adapter Contract

## Purpose

The full data contract between the dashboard's core (session tracking,
layout, rendering) and whatever is watching a coding-agent tool ("harness")
on the core's behalf. `overview.md` R1.3 only establishes that this boundary
exists; this file is the full contract — what an adapter must produce, how
sessions and projects are identified, how staleness is handled, and (for the
adapters that ship) exactly how the opencode adapter meets that contract and
how the opt-in, experimental Claude hook adapter meets the same boundary.
An implementer building the `HarnessAdapter` boundary and either adapter
should be able to work from this file without returning to the requirements
doc. This file does not repeat the exact rendered content
of a session snapshot (status vocabulary, tile text, nickname rules) — those
live in `visuals.md` and `layout.md`, per this spec tree's file split
(`docs/specs/CLAUDE.md`'s File organization table); this file owns the
boundary and delivery contract those fields travel over.

Source: `tasks/2026-09-01-opencode-dashboard.requirements.md`, R1.3 (full),
R1.4–R1.8, R4, R6.4–R6.6.

## Contents

- [Harness-adapter boundary](#harness-adapter-boundary) — R1.3 (full)
- [Session snapshot model](#session-snapshot-model) — R1.4
- [Session identity](#session-identity) — R1.5
- [Project identity](#project-identity) — R1.6
- [Staleness](#staleness) — R1.7
- [Harness-tag slot](#harness-tag-slot) — R1.8
- [The Claude hook adapter (opt-in, experimental)](#the-claude-hook-adapter-opt-in-experimental)
- [The opencode adapter's own mechanism](#the-opencode-adapters-own-mechanism) — R4
- [Tool-call correlation (opencode adapter internals)](#tool-call-correlation-opencode-adapter-internals) — R6.4
- [Action-line rendering (opencode adapter internals)](#action-line-rendering-opencode-adapter-internals) — R6.5-R6.6
- [Known implementation gap: adapter's session-metadata source](#known-implementation-gap-adapters-session-metadata-source)

No child spec files — this is a leaf file in the tree; see `overview.md`
for the full six-file map.

## Scope

Covered: the `HarnessAdapter` boundary contract (R1.3 full), the session
snapshot delivery model (R1.4), session identity (R1.5), project identity
(R1.6), staleness (R1.7), the harness-tag slot (R1.8), and — specific to the
adapters that ship — the opencode adapter's REST/SSE mechanics (R4) and its
tool-call-to-action-line rendering (R6.4-R6.6), plus the opt-in, experimental
Claude hook adapter's lifecycle mapping (see [`claude.md`](claude.md)).

Not covered: the exact rendered content of a session snapshot (status
vocabulary, tile text, nickname rules) — those live in `visuals.md` and
`layout.md` per `docs/specs/CLAUDE.md`'s File organization table; this file
owns the boundary and delivery contract those fields travel over, not their
final on-screen form.

## Harness-adapter boundary

- **R1.3 (full)** — The dashboard's core never talks to a harness's wire
  protocol directly. It depends on a shared boundary, the `HarnessAdapter`:
  one adapter per harness the dashboard can watch. An adapter's sole job is
  to watch its harness however that harness requires — the mechanism is
  entirely the adapter's own choice and invisible to the core (opencode:
  REST + SSE, see R4 below; a hypothetical future hook-based harness: a
  local listener its hooks POST to; another: file-tailing) — and to produce
  session snapshots (R1.4) in the shared shape.

  Everything harness-specific stays inside the adapter and never crosses
  into the core. This explicitly includes:
  - the needs-you/question-detection heuristic (owned by `visuals.md` R6.7)
    — a shared, text-based heuristic any harness with no real wire signal
    for "waiting on you" falls back to (opencode's `Stop` and Claude's
    `Stop` both re-derive it the same way, from the harness's own final
    text); a harness with an actual "waiting on you" signal reports it
    directly, not re-derived from text.
  - the call_id → name tool correlation (R6.4, this file).
  - tool-name-to-action-line rendering (R6.5/R6.6, this file).

  The core only ever receives the already-rendered result (e.g. a
  current-action string like `"editing: foo.rs"`), never a raw tool
  name/args pair. This is what stops a second harness's own tool vocabulary
  from leaking into the shared model through the back door.

  V1 builds the opencode adapter (meeting R4/R6.4/R6.5 below) against this
  boundary, plus an opt-in Claude hook adapter (`claude.md` R11-R17, mapped
  in this file) whose Unix listener is wired into dashboard startup with
  T04; it remains opt-in, and its authenticated lifecycle evidence still
  depends on T05 — see ["The Claude hook adapter"](#the-claude-hook-adapter-opt-in-experimental)
  below. Nothing else is implemented; the core's data model must not assume
  SSE, a REST reconcile sweep, a hook-based listener, or any other
  harness-specific mechanism exists anywhere in its shape.

  Scenario: Given a tool call arrives on the opencode adapter's SSE stream
  carrying a raw tool name and a JSON arguments object, when the adapter
  processes it and hands a session snapshot to the core, then the
  snapshot's current-action field contains only the already-rendered text
  (e.g. `"editing: foo.rs"`) — the raw tool name, the raw arguments, and the
  SSE event itself never appear anywhere in what the core receives.

## Session snapshot model

- **R1.4** — Adapters push **session snapshots**, not fine-grained events:
  the full current state of one session (an upsert keyed by session identity,
  R1.5) or an explicit "session gone" message, all on one shared channel. The
  core stores only the latest snapshot per key, bucketed by project (R1.6),
  and recomputes layout every frame from current snapshots — it never patches
  a snapshot incrementally or holds state derived *from snapshot content*
  between frames (consistent with `layout.md`'s "no caching, no stability
  logic" behaviour, R5.4/R5.7).

  **Exception: identity-keyed allocation state.** This rule is about session
  *content* — the core must not accumulate or infer anything about a session
  beyond what its latest snapshot says. It does not prohibit small state
  keyed by identity rather than derived from content: the naming claim-map
  (`visuals.md` R6.8, forward reference) — which category each live project
  holds, which word each live session holds, and the cooldown bookkeeping
  behind it — is exactly this kind of state, and it lives here, at the core,
  not inside any adapter. It has to: cross-project category exclusivity
  (R6.8's second guarantee) requires visibility across every live project at
  once, which no single adapter has. `visuals.md` R6.8 defines the claim
  scheme's guarantees; this file owns the claim-map's lifecycle — created on
  a project's/session's first claim, released per R1.7's eventual staleness
  rule (forward reference), same as a snapshot would be.

  Exact snapshot field contents (title, status, current
  action text, etc.) are specified where they're rendered — `visuals.md`
  R6.3/R6.7/R6.8, `layout.md` R5.2/R5.3 — this file only specifies the
  delivery model: whole-state upserts or tombstones, one channel, no
  incremental events.

  **Why not a fine-grained event stream.** The rejected alternative was a
  normalized event stream (session created/updated, tool-call
  started/finished, etc.) mirroring the harness's own wire events. It was
  dropped because an adapter already has to keep this same per-session state
  itself, to build things like the recent-actions ring and files-touched list
  (`layout.md` R5.3) — an event stream would fold that same state a second
  time inside the core, for no benefit: the core would just be
  reconstructing the same snapshot the adapter already built. At real usage
  scale (`overview.md` R5.8, ~8 sessions, redrawn every 250ms) snapshot
  chattiness costs nothing, so there's no performance case for the
  fine-grained alternative either. Anyone tempted to reintroduce a granular
  event bus into the core should read this as the reason it was rejected,
  not re-litigate it from scratch.

  Scenario: Given a session's tool call transitions from "shell running" to
  "shell finished", when the adapter reports this, then it sends one
  complete, replaced snapshot of that session's current state on the shared
  channel — not a "tool finished" event that the core would need to fold
  into some state it's tracking itself.

## Session identity

- **R1.5** — Session identity is the tuple `(harness kind, harness-native
  session id)`. The core does no cross-harness reconciliation: if a
  harness's own resume/restart behaviour produces a new native id for what a
  user would consider "the same session," making that continuity visible (or
  not) is that harness's adapter's problem, not the core's. Nickname hashing
  (`visuals.md` R6.8) uses this full tuple, not the raw id alone, precisely
  so two different harnesses can't collide on id format (e.g. both using
  small sequential integers).

  Scenario: Given two different harnesses each happen to produce a session
  with the native id `"123"`, when the core computes identity for each, then
  they are treated as two distinct sessions, because harness kind is part of
  the identity — no collision, no merge.

## Project identity

- **R1.6** — **[CONFIRMED by spike, see `tmp/2026-09-02-project-identity-spike/EVIDENCE.md`]**
  Project identity is the canonical git repository toplevel path of a
  session's working directory. If that directory is not inside a git
  repository, project identity falls back to the canonicalized working
  directory itself — never an adapter-specific placeholder (e.g. opencode's
  own internal `"global"` project id, which collides every non-git directory
  into one bucket; confirmed as the literal value opencode returns for any
  non-git session, per EVIDENCE.md's note on opencode's own `projectID`).

  **Canonicalization**, required before comparing any two paths for project
  identity:
  - Resolve symlinks.
  - Strip trailing slash.
  - Normalize case on case-insensitive filesystems.
    [REVIEW: this clause is evidence-confirmed-as-unverified — the spike's
    build machine (macOS APFS) is case-insensitive but case-preserving, so
    no fixture actually produced a case mismatch to exercise; see
    EVIDENCE.md, "Untested corner of R1.6's canonicalization clause".]
  - Apply the same canonicalization to git's own `--show-toplevel` output,
    not only to the input directory — both need it (e.g. macOS's `/var` is
    itself a symlink to `/private/var`, so an uncanonicalized toplevel and a
    canonicalized working-directory fallback would silently disagree on the
    same physical location).

  **Caching obligation (implementation note).** Resolving to git toplevel
  means spawning a subprocess (`git rev-parse --show-toplevel` against the
  session's directory). A session's directory→toplevel mapping cannot change
  during that session's lifetime, so the resolver must cache this mapping
  per session rather than re-spawning git on every snapshot or every redraw.
  The T01 spike measured correctness only, not call frequency — this
  obligation was never itself a tested requirement, only a documented
  implementation note from advisor review at the M1 milestone.

  **Worktree, subfolder, and subagent behaviour**, all confirmed by the T01
  spike against real opencode sessions and filesystem fixtures (repo root,
  subfolder, parent+subagent, explicit bridge `directory` param, symlink,
  two worktrees — 9 checks, all matched, see EVIDENCE.md):
  - Two git worktree checkouts of the same repository resolve to **different**
    toplevels and get **separate** project boxes. There is no
    worktree-merging logic anywhere in this resolution.
  - A session whose working directory is a subfolder inside a git repository
    (including a subfolder of a monorepo) resolves to that repository's
    single toplevel — it is grouped under the same project box as a session
    at the repo root, never split out per subfolder.
  - A subagent/child session inherits its parent's working directory
    exactly. Because project identity is a pure function of working
    directory, the child lands in the same project box as its parent
    automatically — no special-casing is needed at this layer (subagents are
    still handled specially at the session-identity/rendering layer, per
    R1.5 and `layout.md`'s R5.1/R5.6, just not here).

  Scenario: Given two sessions — one with a working directory at a git
  repository's root, another with a working directory in a subfolder of that
  same repository — when the adapter resolves project identity for each,
  then both resolve to the same canonicalized git toplevel path and are
  grouped in the same project box; given a third session running in a
  separate git worktree checkout of that same repository, when its project
  identity is resolved, then it resolves to a different canonicalized path
  (that worktree's own toplevel) and appears as a separate project box.

## Staleness

- **R1.7** — No session may go silently stale forever because a push-only
  adapter missed a message. The core requirement, independent of any
  adapter's transport: track time-since-last-snapshot per session, and treat
  one that has gone quiet past a threshold as stale rather than assuming it
  is still accurate forever. The opencode adapter gets a free safety net
  from its own 60-second REST reconcile sweep (R4) — a dropped SSE
  connection self-corrects on the next sweep regardless of this rule. The
  opt-in Claude hook adapter is push-only: it has no REST to reconcile
  against, so this rule is what keeps its sessions from freezing on a
  missed message forever. The core's active-window reclassification (R3)
  currently treats long-quiet Claude sessions as idle; the adapter itself
  records receipt timestamps and removes nothing.

  [REVIEW: OPEN, see requirements doc — the exact staleness threshold and
  its on-screen treatment (dimmed? a distinct status? dropped from view
  entirely?) are not decided. T05 owns the final policy and its evidence.
  Not urgent for V1 since the opencode adapter's reconcile sweep already
  covers it in practice; this becomes load-bearing once the opt-in Claude
  adapter is enabled.]

  Cross-reference: `visuals.md` R6.8's naming-claim scheme depends on
  whatever this rule eventually decides — both claim layers there (project→
  category, session→word) must release their claim when a session is judged
  stale under this rule, not only on an explicit "gone" tombstone, or a
  silently-stale session leaks its claimed seat forever.

  Scenario: Given a session's last snapshot arrived long enough ago to cross
  the (not-yet-decided) staleness threshold, when the dashboard evaluates
  that session, then it is treated as stale rather than assumed current —
  the exact threshold value and the resulting on-screen treatment are
  `[REVIEW: OPEN, see requirements doc]`.

## Harness-tag slot

- **R1.8** — When sessions from more than one harness kind are present at
  the same time, each tile reserves a small slot identifying which harness
  that session came from. The slot must exist in the tile layout so this
  isn't a retrofit later. The slot is hidden/absent whenever only one
  harness kind is present in the current data — the default render, since
  the Claude adapter is opt-in (T04 wires its listener into startup, but
  sessions appear only once the user configures hooks).

  [REVIEW: OPEN — the exact glyph and its placement within the tile are not
  decided; that's a `visuals.md`/`layout.md` tile-content-ladder detail to
  design once a second adapter actually renders live data.]

  Scenario: Given only the opencode adapter is active — the default, since
  Claude monitoring requires the user to configure hooks — when the
  dashboard renders any tile, then no harness-tag glyph is shown; given a
  build with Claude hooks configured and both harness kinds live at the
  same time, when the dashboard renders a tile, then a small reserved slot
  identifies which harness that session came from — exact glyph and
  placement `[REVIEW: OPEN]`.

## The Claude hook adapter (opt-in, experimental)

Claude monitoring is a second, opt-in harness in this spec tree
(`claude.md` R11-R17). Its adapter is implemented on exactly this
`HarnessAdapter` boundary, and T04 wires the Unix listener into dashboard
startup: normal startup opens the user-scoped listener and starts the
adapter, alongside the `dashboard claude-hook` hook command (R11), so a
configured hook's events reach the dashboard. Claude sessions still appear
only after the user configures hooks — the capability is opt-in, and
nothing at startup reads or writes Claude configuration; the hooks the
user adds are the sole switch (R11-R12). The adapter remains an
experimental capability pending T05's authenticated end-to-end evidence.

The adapter accepts the fifteen events listed in `claude.md` R13 and maps them
onto the shared snapshot stream. Event-specific lifecycle and attention
semantics are defined by `claude.md` R13; bounded content fields and their
privacy limits are defined by R14-R15. Depending on the event, snapshots may
carry identity, attention, current action, recent actions, question content,
tool content, or assistant text; fields not supplied by an event remain absent
or unchanged rather than being treated as universally empty. This section does
not duplicate the Claude event matrix.

`SessionEnd` removes the session and emits a `Gone` tombstone — the same
whole-state upsert / tombstone contract R1.4 defines, exactly as the opencode
adapter uses it.

Snapshots carry identity (`HarnessKind("claude")` + harness-native id, R1.5),
canonical project identity resolved through the same `ProjectIdentityCache`
as the opencode adapter (R1.6, including the same documented degraded
uncanonicalized fallback when a cwd cannot be resolved), and `created_at` /
`last_updated` / the `NeedsYou` basis from local hook receipt times
(`claude.md` R14 — the only timestamp that crosses the boundary). Content and
activity fields are populated only where an accepted event provides them and
remain absent or unchanged otherwise. The adapter records receipt timestamps
but implements no expiry or removal of its own: R3's active/idle window
reclassification, computed by the core from `last_updated`, is the provisional
treatment.

[REVIEW: T05 owns the final stale-session policy (the R1.7 threshold and
its on-screen treatment) and the authenticated lifecycle evidence
(successful-turn behavior, subagent identity, async-hook viability,
exit-path reliability — `claude.md` R17). Until T05
closes, this adapter makes no completeness claim: it shows only sessions
whose hook events were delivered while the adapter ran, and it cannot
verify authenticated Claude behavior.]

Scenario: Given the opt-in Claude hooks are configured and a `SessionEnd`
envelope arrives for a session the adapter has seen, when the adapter
processes the envelope, then it removes that session and emits one `Gone` —
the same tombstone contract the opencode adapter uses for a session that
vanished from its server.

## The opencode adapter's own mechanism

**This section is opencode-adapter-specific, not a core requirement — see
R1.3's boundary above.** It describes how the opencode adapter meets the
`HarnessAdapter` contract; nothing here constrains how any other adapter
(including the Claude hook adapter above) would meet the same contract.

- **R4** — The opencode adapter fetches all sessions via `GET /api/session`
  (paginated) and per-session details on zoom, then keeps them live via
  `GET /api/event` SSE plus a 60-second periodic reconcile sweep, emitting
  session snapshots (R1.4) from the result. Correctness comes from the
  polling/sweep side; SSE exists for latency, not as the sole source of
  truth (same pattern as the existing MCP bridge, `SPEC.md:7`). This
  reconcile mechanism exists only because opencode happens to expose a
  pollable REST API — a push-only adapter (e.g. a hook-based harness) has no
  equivalent and relies on R1.7's staleness rule instead.

  Scenario: Given the opencode adapter's SSE connection silently drops
  without either side detecting the break, when the next 60-second reconcile
  sweep runs, then it re-fetches session state via REST and corrects any
  snapshot that the dropped SSE connection would otherwise have left stale.

## Tool-call correlation (opencode adapter internals)

- **R6.4** — Confirmed field shapes on opencode's SSE wire: a
  `session.tool.input.started` event carries `{id, name}` — this is the only
  event that carries the tool's name. A `session.tool.called` event carries
  `{id, input}` (the call's arguments) with no name at all. The two are
  joined by the shared `id` (the call id), scoped per session.

  The opencode adapter keeps a per-session `call_id → name` map: populated
  when `input.started` arrives, consumed when the matching `tool.called`
  arrives (to build the display line — see R6.5), then dropped once
  consumed. The core dashboard never sees the raw SSE events or this map —
  only the already-rendered action line, inside the session snapshot (R1.4).
  This tracking is in addition to the SSE consumption already needed for the
  terminal `session.execution.*` events (R4).

  Scenario: Given a `session.tool.input.started` event with `id: "call_1"`,
  `name: "edit"` arrives, followed later by a `session.tool.called` event
  with `id: "call_1"`, `input: {path: "foo.rs", ...}`, when the adapter
  processes both, then it produces the rendered line `"editing: foo.rs"`
  from the joined id and discards the `call_1` entry from its map — the core
  dashboard's snapshot carries only that rendered line, never `"edit"`,
  `"call_1"`, or the raw input object.

## Action-line rendering (opencode adapter internals)

- **R6.5** — Coarse tool → action-line mapping, built only from tool names
  confirmed on the wire (verbatim from the requirements doc):
  - `shell` → `input.command`, single line, collapse newlines to `" · "`,
    truncate to tile width with `…`.
  - `edit` → `"editing: "` + basename of `input.path` (fall back to the full
    relative path if it fits).
  - any other/unrecognized `name` → `"running: <name>"` (generic fallback,
    no arg parsing).

  The action line updates on each `tool.input.started` → `tool.called` pair;
  it holds its last value between tool calls until the next one arrives, or
  the session's terminal `session.execution.*` event fires. [REVIEW: OPEN —
  what happens to the line when the terminal event fires (clears, or shows
  an outcome) is explicitly TBD at implementation in the requirements doc,
  not decided here.] No per-tool detail beyond this mapping (no diff
  preview, no truncated grep pattern, etc.) is in scope — that's R6.6's
  phase 2.

  Scenario: Given a session's most recent tool call is `shell` with
  `input.command: "cargo test\n--release"`, when the adapter builds the
  action line, then it renders `"cargo test · --release"` (newlines
  collapsed to `" · "`), truncated with `…` if it exceeds the tile's
  available width.

- **R6.6** — Full tool-name vocabulary confirmed to exist on opencode's wire:
  `shell, edit, grep, write, skill, subagent, glob, patch, read`. Confirmed
  live `input`/`success` shapes for 7 of the 9 — `shell, edit, grep, write,
  skill, subagent, read` — see
  `docs/internal/opencode-sse-event-catalog-2026-09-01.md` §3 for the exact
  field shapes of each.

  [REVIEW: OPEN — `glob` and `patch` are unconfirmed. Both were offered to
  the model in the same test session that confirmed the other 7, but it
  chose not to use either (picked `grep` over `glob` for a text search, had
  no multi-hunk diff task that would justify `patch` over `edit`). A more
  targeted prompt (e.g. "find all *.txt files" to force `glob`; "apply this
  3-hunk diff" to force `patch`) is needed to confirm their shapes.]

  [REVIEW: OPEN — per-tool action-line formatting beyond R6.5's `shell`/
  `edit` rules is deferred to phase 2 for all of `grep, write, skill,
  subagent, glob, patch, read`. Until that phase-2 design happens, R6.5's
  generic `"running: <name>"` fallback covers every one of these tools.]

  Scenario: Given a `grep` tool call completes — a tool with a confirmed
  wire shape but no dedicated action-line rule yet — when the adapter builds
  the action line, then it falls back to R6.5's generic form
  `"running: grep"` rather than inventing a grep-specific rendering; the
  same applies to `write, skill, subagent, read, glob,` and `patch` until
  their phase-2 formatting is designed.

## Known implementation gap: adapter's session-metadata source

[REVIEW: see deferred.md] The MCP bridge already has its own model of a
session's metadata — a `SessionInfo` struct at `src/opencode.rs:29` — but it
only deserializes `id, outcome, time, cost, tokens, title`. It silently
drops `location`, `projectID`, and `subpath` even though the opencode server
sends all three on every session response (confirmed by the T01 spike, which
had to curl the server directly instead of using the bridge's own tools —
see EVIDENCE.md, "Why direct curl instead of the MCP tools for part 1").

This matters here because R1.6 (project identity, above) depends on reading
`location.directory` for every session, and the opencode adapter has no
other source for it. Building the opencode adapter means extending this
struct (or reading the server's session response independently of it) to
carry `location`, `projectID`, and `subpath` through. This is an M3
implementation task, not yet scoped elsewhere — recorded in
`tasks/2026-09-02-opencode-dashboard/deferred.md` (T01 section) and in the
requirements doc's Open Questions as `[OPEN, M3-relevant]`.
