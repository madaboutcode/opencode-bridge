# T02 - Local ingress and privacy contract

**Contract version** - 6

**Context** - goal: implement and test the strict Claude hook parser and
  best-effort local Unix-socket ingress after T01's evidence decisions; who uses
  it: the future Claude adapter and the user-configured hook command; scale:
  short-lived concurrent hook processes on one selected-user workstation;
  criticality: high because privacy leakage or a blocking helper is a release
  stopper.

**Delivery profile** - `tasks/2026-09-03-claude-dashboard-support/delivery-profile.md` version 1; task override: none.

**Boundaries** - owns: `crates/dashboard/src/claude/hook.rs`,
  `crates/dashboard/tests/claude_ingress.rs`, and
  `docs/specs/dashboard/claude.md` plus the required convention and index
  updates in `docs/specs/CLAUDE.md`, `docs/specs/README.md`, and the dashboard
  registry `docs/specs/dashboard/overview.md`, including unit tests inside
  `hook.rs` and
  the Cargo-executed real-socket integration tests in `claude_ingress.rs`; must
  not touch `crates/dashboard/src/claude/mod.rs`,
  `crates/dashboard/src/lib.rs`, `crates/dashboard/src/main.rs`, the core
  adapter/snapshot types, user Claude configuration, transcript files, or
  unrelated dirty worktree files.

**Conventions** - invoke `writing-specs` before creating the spec and follow
  `docs/specs/CLAUDE.md`: stable requirement identifiers, one Given/When/Then
  scenario per requirement, consumer-facing behavior, and `[REVIEW: ...]` for
  unresolved evidence questions. Keep Claude-specific JSON types and filtering
  inside the ingress module. Use bounded lengths and framing, version the local
  envelope, use a user-scoped Unix socket, and treat malformed input, unknown
  events, unavailable/full listeners, and oversized values as logged/drop cases
  that exit successfully without affecting Claude. The complete delivery
  deadline must include socket-path resolution and every filesystem metadata
  operation, including `symlink_metadata`, not only connect/write calls. Add a
  real busy/full-listener test that asserts the `ListenerUnavailable` outcome.
  Never retain the original `serde_json::Value` or forward rejected fields.

**Skills to read and apply** - `writing-specs`, `writing-unit-tests`,
  `code-quality`, `software-design`.

**Acceptance - done when** - the owned ingress module, Cargo-executed ingress
  tests, spec, and updated spec-tree convention/index/registry provide an
  evidence-backed allowlist for supported Claude events and optional fields,
  parse into an internal record, emit only a bounded versioned newline-delimited
  envelope, and expose a best-effort helper command path that:

- exits successfully when the listener is absent or cannot accept promptly;
- rejects malformed JSON, unknown events, empty/oversized IDs and paths, and
  sensitive fields as specified without sending their values;
- sends only session ID, CWD, event metadata, and local receipt time plus the
  approved tool/notification/subagent labels;
- survives concurrent short-lived sends and stale/restarting sockets within the
  selected bounds; and
- keeps the entire helper attempt, including filesystem metadata work, within
  the R16 delivery deadline; and
- exercises a real busy/full listener and asserts the resulting
  `ListenerUnavailable` outcome; and
- has negative sentinel tests proving prompt text, assistant text, transcript
  paths, tool input/output, and arbitrary unknown fields do not cross the IPC
  boundary or appear in logs/serialized envelopes.

The integration tests import the owned hook module and use a real Unix socket,
so the ingress behavior is actually compiled and executed before T03 wires the
module into the library. They validate ingress independently; T05
validates the complete integrated Claude-to-dashboard path after T03/T04 wire
the adapter and runtime. No test may read or write global Claude configuration
or transcript files.

**Gate** - report-only (refine-loop).

**Dependencies** - T01c; consumes the adopted current evidence baseline and
  its four T05 deferrals. Failed T01/T01b artifacts are historical records, not
  dependencies.

## Review Frame

**As of** - contract version 6

**Context** - Bounded ingress correction closes user-scope, deadline, documentation, and privacy-evidence gaps before M1 can advance.

**Expectations** - Require no shared fallback; deadline covers metadata; align all six-file registries and split R15. Verify busy listener maps to `ListenerUnavailable` and sentinels never reach logs.

**Depth** - Deep review only of socket/IPC, specs, and executable privacy tests; exclude adapter/runtime and reserve authenticated E2E for T05.
