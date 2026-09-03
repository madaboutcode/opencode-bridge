# T03 - Claude adapter and provider-neutral state

**Contract version** - 1

**Context** - goal: convert T02's typed Claude hook envelopes into the shared
  `SessionEvent` stream; who uses it: T04 listener/runtime wiring and the
  dashboard core; scale: short-lived local hook deliveries and a small number
  of live sessions on one selected-user workstation; criticality: high because
  a Claude-specific field or unverified lifecycle claim crossing the core would
  violate the provider-neutral boundary.

**Delivery profile** - `tasks/2026-09-03-claude-dashboard-support/delivery-profile.md` version 1; task override: one DeepSeek implementation pass and one fresh Luna verification after T03 design/decomposition sign-off.

**Dependencies** - T01c evidence baseline `401887e` and T02 v6 ingress
  `aeb8317`; T04 listener/runtime wiring is downstream. The four T05 deferrals
  remain active.

**Boundaries** - owns:

- `crates/dashboard/src/claude/DESIGN.md`
- `crates/dashboard/src/claude/mod.rs`
- `crates/dashboard/src/claude/state.rs`
- `crates/dashboard/src/claude/wire.rs`
- `crates/dashboard/src/lib.rs`
- `crates/dashboard/tests/claude_adapter.rs`
- `docs/specs/dashboard/client.md`
- `docs/specs/dashboard/overview.md`

The existing T02 `hook.rs` is consumed unchanged. T03 must not touch
`hook.rs`, `adapter.rs`, `snapshot.rs`, `project_identity.rs`, OpenCode files,
`main.rs`, shell/runtime files, user Claude configuration, project `.claude`,
transcripts, or unrelated dirty worktree files.

**Conventions** - use `HarnessKind("claude")` for every Claude `SessionId` and
  the existing whole-snapshot plus tombstone `HarnessAdapter` boundary. Decode
  only protocol version 1 and T02's allowlisted envelope fields. Keep raw JSON
  transient and never retain or log rejected values. Use the existing
  `ProjectIdentityCache<GitDirResolver>` and its documented degraded identity
  fallback when a cwd cannot be resolved. Put a CONTRACT block in every new
  source file and keep `claude/DESIGN.md` synchronized with the implementation.

**Lifecycle contract** - because T01c supports only `SessionStart`,
  `StopFailure`, and `SessionEnd`, the adapter maps the first two to one
  complete `NeedsYou { question: false }` snapshot and maps `SessionEnd` to one
  `Gone` event while removing the session. A first `StopFailure` admits a
  session with its receipt time as `created_at`. Duplicate starts update the
  existing session without resetting its creation time. All snapshot content
  fields not supported by T01c are `None` or empty.

**Staleness contract** - set `last_updated` to the local hook receipt time for
  every accepted snapshot. Do not invent an adapter timer, expiry, or removal
  policy: the core's existing active-window reclassification remains the
  provisional treatment and T05 owns final stale-session evidence and policy.

**Acceptance - done when** - the owned adapter:

- implements `HarnessAdapter`, exposes a channel for T04 to submit typed T02
  envelopes, and reports `HarnessKind("claude")`;
- decodes only version 1 bounded envelopes, drops malformed/unknown records
  without terminating the adapter, and emits no raw wire values;
- maps the three observed events to complete snapshots/tombstones exactly as
  specified, preserves creation/receipt timestamps, and keeps all unsupported
  snapshot fields empty;
- resolves project identity from envelope `cwd` using the shared cache and
  continues after one resolution failure using the documented degraded path;
- keeps Claude and OpenCode native IDs distinct and does not add Claude fields
  to provider-neutral types;
- includes a real feature test proving socket delivery -> envelope decode ->
  adapter snapshot -> `Gone`, plus unit coverage for all failure and lifecycle
  cases;
- updates `client.md` and `overview.md` so their adapter descriptions no
  longer claim OpenCode is the only available harness, while retaining T05's
  authenticated and final-staleness review markers; and
- leaves the four T05 deferrals, no-global-config/no-transcript rule, and T04
  listener ownership unchanged.

No T03 criterion may claim authenticated Claude behavior, support an event
outside T01c's allowlist, read history/transcripts, or control a session.

**Testing** - the feature test must use a real Unix socket and T02's
  `deliver_to`, decode the received bytes, submit the typed envelope to a live
  adapter task, and assert provider-neutral snapshots and the terminal
  tombstone. Unit tests must cover protocol/version validation, malformed and
  unknown input, all three lifecycle events, duplicate/first events, project
  fallback, metadata-only empty fields, receipt timestamps, channel closure,
  and same-native-id cross-harness separation. Run targeted dashboard tests,
  workspace tests, clippy with `-D warnings`, and format checks.

**Gate** - report-only refine-loop: one implementation pass and one independent
  Luna verification. T04 remains blocked until this gate is clean and committed.

## Review Frame

**As of** - contract version 1

**Context** - Adapter-only Candidate B turns three evidence-backed envelopes into complete provider-neutral events through typed wire, state, and channel layers.

**Expectations** - Accept only v1 `SessionStart`/`StopFailure`/`SessionEnd` metadata mapping, fallback identity, and no adapter expiry; no Claude fields or raw data enter shared core. T04 owns listener/startup; T05 owns authenticated E2E and final staleness.

**Depth** - Deep review of decoder, state, snapshot/tombstone feature seam, and identity fallback; exclude hook ingress, runtime wiring, and shared-core redesign.
