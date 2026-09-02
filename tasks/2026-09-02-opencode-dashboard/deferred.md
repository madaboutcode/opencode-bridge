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
