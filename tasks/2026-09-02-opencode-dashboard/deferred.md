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
