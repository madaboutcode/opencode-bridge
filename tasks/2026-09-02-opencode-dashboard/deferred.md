# Deferred findings — opencode-dashboard run

Real-but-out-of-scale or out-of-boundary findings, parked here by each task's
runner at gate time. Not a backlog of nice-to-haves — only things a reviewer
or implementer actually found and judged real.

## T01 — project-identity spike

- The MCP bridge's own `SessionInfo` struct (`src/opencode.rs:29`) doesn't
  deserialize `location`, `projectID`, or `subpath` even though the opencode
  server sends them on session metadata — the spike had to curl the server
  directly for evidence instead of using `opencode_sessions`. Real gap,
  relevant to M3's opencode adapter work, out of scope for T01 (its boundary
  excludes `src/`).
- R1.6's "normalize case on case-insensitive filesystems" clause is untested —
  the machine used for this spike has a case-preserving filesystem, so no
  fixture produced an actual case mismatch to exercise. Recorded as an
  explicit gap in `tmp/2026-09-02-project-identity-spike/EVIDENCE.md`, not
  silently passed. Worth a dedicated check if a case-insensitive-filesystem
  environment becomes relevant to the dashboard.
- `std::fs::canonicalize` requires the path to exist. A session whose working
  directory was deleted after the session started, or lives on an unmounted
  volume, makes project-identity resolution error with no defined fallback.
  Happy-path-first for this run (per PLAN.md), so not a new task — flagged by
  advisor at M1 milestone review, recorded here rather than silently
  unhandled.

## T08 — Cargo workspace migration

- `crates/opencode-client/src/opencode.rs` (a byte-for-byte move, unchanged
  content per the contract's migration rule) carries comments that reference
  bridge-internal modules — `tools.rs`, `sse.rs`,
  `registry::Registry::claim_notification` — which made sense when the file
  lived inside the bridge crate. Now that it's in the shared client library,
  these describe a consumer's internals from inside the library. Assumption:
  cosmetic only, doesn't affect correctness or the crate boundary itself.
  Consequence: a reader of the library in isolation sees references to
  modules it can't see. Trigger: a comment-cleanup pass on
  `opencode-client/src/opencode.rs`, or the next time that file is touched
  for another reason.
- A real `cargo publish -p opencode-bridge` (registry publish, not
  `--no-verify` packaging) currently fails: `opencode-client` isn't itself
  published to crates.io, so cargo can't resolve the path dependency against
  the registry index. Surfaced by the implementer, not the reviewer.
  Assumption: this project's actual release process is tag-push → GitHub
  Release with binary artifacts (confirmed in `CONTRIBUTING.md`), never
  `cargo publish`; CI only runs `cargo package --list --no-verify`, which
  skips registry resolution and doesn't catch this. Consequence: none under
  today's release process; would block a future switch to `cargo publish`.
  Trigger: this project ever adopts `cargo publish` as its release path, or
  needs `opencode-client` published independently (e.g. for `crates/dashboard`
  or a third consumer to depend on it via crates.io rather than a path/git
  dependency).

## T09 — HarnessAdapter boundary, core session model, opencode adapter

- Several fallback constructs inside the opencode adapter (behind the
  boundary, not on the release-bar surface) lack a `code-quality`-style
  `FALLBACK-OK:` citation: `opencode/action_line.rs:62,66`,
  `opencode/reconcile.rs:59,104,111`, `opencode/mod.rs:190,228`. Found by the
  reviewer (`ask_opus`). The one with a visible downstream effect is
  `reconcile.rs:59` — if a session ever arrived with no `location` at all
  (T01's spike found this doesn't happen on real wire traffic), the resolver
  would key a project identity off an empty path, producing a phantom
  project box. Assumption: `location` is always present on real opencode
  sessions. Consequence: degraded display of one session, never corruption
  of another. Trigger: any of these fallbacks producing a visible rendering
  bug (empty action line, phantom empty-path project box) — add citations
  and tighten handling at that point.
- `TrackedSession::start_turn` (`opencode/session_state.rs:61-64`) clears
  `files_touched` on a new turn but not `current_action` — a session's
  action line from the previous turn's last tool call persists into the new
  `Running` snapshot until the first tool call of the new turn overwrites
  it. Found by the reviewer. Assumption: this window is a few seconds at
  most (until the first tool call lands) and self-corrects. Consequence: a
  momentarily stale action line at the start of a turn. Trigger: reported as
  visually confusing in real use — clear `current_action` in `start_turn`.
- R6.5's edit-action fallback ("full relative path if it fits") is not
  implemented anywhere — the opencode adapter always renders `basename`
  only, on the reasoning that "if it fits" is a tile-width (render-time,
  T11) quantity the adapter can't evaluate. Self-flagged by the implementer.
  Assumption: T11's render layer either doesn't need this fallback or will
  need to build its own width-aware truncation/fallback logic independently,
  since nothing in T09's snapshot carries enough information (the full
  relative path) to reconstruct it if T11 only receives the basename.
  Actually: the adapter renders `"editing: " + basename`, a single string —
  T11 has no separate access to the full relative path if the basename
  string doesn't fit. Consequence: R6.5's fallback behavior is silently
  unreachable in the current design. Trigger: T11's implementer or reviewer
  should check whether this gap needs the full relative path to reach the
  snapshot (a straightforward additive field) before T11 gates.
- `crates/dashboard`'s opencode adapter calls `GET /api/session` as a single
  unpaginated page — `opencode_client::Client::list_sessions` doesn't loop
  pages. Self-flagged by the implementer. Assumption: fine at the spec's
  confirmed ~8-session design center (R5.8); the existing "50+ session
  overflow" deferral in the delivery profile already covers stress scale
  more broadly. Consequence: sessions beyond one page silently never appear
  in the dashboard at real stress scale. Trigger: same as the profile's
  existing 50+ session deferral — the user reports missing/incomplete
  session lists, or routinely runs enough sessions to plausibly exceed one
  page.
## T10 — Naming and claim-map (R6.8)

- `crates/dashboard/src/naming/claim_map.rs:377`'s `release_session` has an
  `.unwrap_or(0)` on a lookup that should always succeed once
  `session_location` has an entry for the session (the method returns early
  otherwise). Found by the reviewer (`ask_opus`), self-dispositioned by the
  reviewer as style, not correctness — the fallback triggers the same
  cleanup path `remaining == 0` would, so an impossible state self-heals
  instead of panicking. Per `code-quality`'s `FALLBACK-OK:` convention this
  should carry a citation or become an `expect(...)`. Assumption: the
  invariant (a tracked session's project is always in
  `project_live_sessions`) genuinely never breaks given the module's own
  bookkeeping. Consequence if wrong: silent no-op instead of a loud failure
  in a state that should be provably impossible. Trigger: next time this
  file is touched for another reason, or if the invariant is ever suspected
  broken (e.g. a project's live-session count looks wrong after a release).
- The delivery profile's existing R6.8 capacity-edge-case deferral (more
  live projects than categories, or more live sessions than a category has
  words) assumed the consequence would be "a duplicate name, i.e. the
  guarantee silently fails." T10's actual implementation degrades instead of
  silently duplicating: category overflow sets a project's assignment to
  `shared: true` (word-claims stay category-scoped, so the per-project
  guarantee still holds even when shared); word overflow appends a numeric
  suffix (`"Apollo-2"`) with the smallest unused suffix, so per-project
  uniqueness holds even past a category's word count. The profile's original
  trigger ("either count approaching its list length") still stands — this
  note is so whoever hits that trigger knows the actual behavior to expect
  (a shared category or a suffixed name) rather than a raw duplicate.

- `AttentionState::Running`'s `turn_started` timestamp is set from the
  adapter's own observation of `session.execution.started` over SSE; when
  the reconcile sweep is what first discovers a running session (e.g. the
  dashboard starts up mid-turn, or SSE was down when the turn actually
  started), there is no wire field carrying true turn-start time, so it
  falls back to `last_updated`. Self-flagged by the implementer. Assumption:
  this only affects the "running for Nm" elapsed-time display, only at
  adapter/dashboard startup or after an SSE outage, and self-corrects at the
  next turn boundary. Consequence: an understated "running for" duration in
  that narrow window. Trigger: reported as a visibly wrong elapsed time in
  real use.

## T11 — Mosaic layout and card rendering (spike promotion)

- R6.5's "fall back to the full relative path if it fits" width-aware
  fallback stays unreachable in v1, by design. T09's `SessionSnapshot` only
  carries the already-rendered `"editing: " + basename` string
  (`current_action: Option<String>`), never a separate full relative path a
  width-aware choice would need to evaluate. T11's render layer passes
  `current_action` straight through unmodified — it does not attempt to
  reverse-engineer a path from the rendered string, since that would couple
  T11 to T09's internal rendering format instead of its declared type.
  Reviewed and signed off by `ask_opus` (T11's reviewer) as correct given
  `snapshot.rs`'s own doc comment on `current_action` ("never a raw tool
  name or argument object" — i.e. intentionally a pre-rendered,
  non-decomposable string). Assumption: basename-only is an acceptable v1
  simplification of R6.5. Consequence: a long path that would fit in a wide
  tile is still shown truncated to its basename; readability floor is still
  met, just not the spec's stated "prefer full path when it fits" behavior.
  Trigger: if this is reported as a real usability complaint, or if a future
  task needs the fallback, promote by adding an additive field to T09's
  `SessionSnapshot` (e.g. a full relative path alongside the pre-rendered
  action string) — that's a T09-boundary change, not a T11 one.
- Subagent nicknames fall back to the harness-native session id
  (`view.rs`'s `nickname_or_fallback`) when T10's `NamingClaimMap` has no
  claimed nickname for that session. Found by the implementer while wiring,
  not present in T09's original deferral list. Root cause: neither T10's
  contract nor `visuals.md` R6.8 specifies who is responsible for calling
  `claim_batch`/`claim_session` for a *subagent* session — `LiveSession`
  doesn't distinguish top-level from subagent sessions at all, and T11's
  boundary limits it to reading T10's public output, never driving claims
  itself. If T12's eventual wiring only ever claims top-level sessions, a
  subagent's `nickname_of()` lookup always returns `None`. Assumption: this
  fallback (truncated harness-native id, still harness-agnostic since T09
  already hands it over as an opaque string) is an acceptable placeholder
  rather than a crash — `render.rs`'s `draw()` runs every frame under T12's
  main loop, so panicking over a missing cosmetic label would be
  disproportionate. Reviewed and signed off by `ask_opus` as meeting the
  `FALLBACK-OK` bar. Consequence: subagent tiles show a raw id fragment
  instead of a claimed nickname, unless/until T12's wiring claims them too.
  Trigger: at T12 scoping time, decide whether subagent sessions get claimed
  in the `NamingClaimMap` (making this fallback path dead code in practice)
  or whether the fallback is the accepted permanent v1 behavior — this
  needs an explicit call, not a default assumption either way.
