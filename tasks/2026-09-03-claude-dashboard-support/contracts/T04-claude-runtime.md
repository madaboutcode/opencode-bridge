# T04 - Claude runtime and hook command

**Contract version** - 1

**Context** - goal: wire the T02 command ingress and T03 adapter into the
  dashboard runtime; who uses it: manually configured Claude Code hooks and the
  dashboard process; scale: short-lived concurrent local hook commands on one
  selected-user workstation; criticality: high because a blocking hook,
  unsafe socket cleanup, or leaked payload would affect Claude or user data.

**Delivery profile** - `tasks/2026-09-03-claude-dashboard-support/delivery-profile.md` version 1; task override: one DeepSeek implementation pass, mandatory independent spec validation, and one fresh Luna verification.

**Dependencies** - T01c evidence baseline `401887e`, T02 v6 ingress `aeb8317`,
  and T03 adapter `e631129`. T04 consumes T02/T03 public APIs and does not
  broaden their event or privacy contracts. T05 remains the authenticated
  full-path gate.

**Boundaries** - owns:

- `crates/dashboard/src/claude/command.rs`
- `crates/dashboard/src/claude/listener.rs`
- `crates/dashboard/src/claude/mod.rs` (only T04 runtime exports)
- `crates/dashboard/src/main.rs` (only isolated T04 dispatch/startup hunks;
  preserve the pre-existing icon-mode dirty hunk verbatim)
- `crates/dashboard/Cargo.toml` (Tokio feature additions only)
- `crates/dashboard/tests/claude_runtime.rs`
- `docs/specs/dashboard/claude.md`
- `docs/specs/dashboard/client.md`
- `docs/specs/dashboard/overview.md`
- `tasks/2026-09-03-claude-dashboard-support/spec-delta.md`

T04 must not touch `hook.rs`, `state.rs`, `wire.rs`, `adapter.rs`,
`snapshot.rs`, `project_identity.rs`, OpenCode sources, shell sources, T01c or
T02/T03 evidence/gates, user/project Claude configuration, transcripts, or
unrelated dirty worktree files. Terra must approve the `main.rs` re-scope in
the Review Frame before implementation.

**Conventions** - use T02's `claude_socket_path`, `parse_hook_input`, and
`deliver`; use T03's `decode_envelope` and `ClaudeAdapter::channel`; do not
duplicate path resolution, parsing, serialization, lifecycle mapping, or
snapshot construction. Keep all runtime logs on stderr and category-only.
Use a user-scoped Unix socket only. Put a CONTRACT block in every new Rust
source file. Do not add a public TCP endpoint, shared temp fallback,
configuration mutation, persistence, transcript access, or session control.

**Hook command contract** - the exact `dashboard claude-hook` command reads no
more than `MAX_HOOK_INPUT_BYTES + 1` bytes from stdin, passes valid UTF-8 to T02
with `ReceivedAt::now()`, and invokes T02 best-effort delivery. Malformed,
unknown, oversized, invalid-UTF8, absent-listener, and unavailable-listener
cases exit successfully, emit no stdout, and never print payload values. The
command performs no OpenCode pairing, TUI startup, socket listening, or Claude
configuration/transcript access.

**Listener contract** - normal dashboard startup resolves exactly T02's
user-scoped socket path and binds a local Unix listener before starting Claude
or OpenCode adapters. Bind/path failure disables only Claude monitoring and
does not fail the dashboard. A stale path is removed only when
`symlink_metadata` identifies a socket; regular files and symlinks are refused
and never deleted. Each accepted connection has a fixed concurrency slot, a
finite read deadline, a hard `MAX_ENVELOPE_BYTES` frame bound, and at most one
newline-delimited frame. Only a successful T03 `decode_envelope` result is sent
to the T03 channel. All other cases are category-only drops; later valid
connections continue. Listener shutdown closes the socket and best-effort
removes its owned path.

**Startup contract** - the exact first argument `claude-hook` bypasses normal
dashboard startup. Normal mode retains the existing OpenCode pairing, adapter,
and shell flow, with Claude listener/adapter startup added before those
adapters. The pre-existing icon-mode changes in dirty `main.rs` are preserved
verbatim outside the approved T04 hunks.

**Documentation contract** - update the Claude spec's consumer-visible R11,
R12, R16, and R17 behavior to describe the concrete helper command, manual
opt-in settings shape/scope/removal, listener availability and cleanup, bounded
degraded behavior, and continued no-completeness/authenticated limitations.
Update `client.md` and `overview.md` so they no longer say T04 listener wiring
is absent while preserving opt-in status and T05 review markers. Record every
requirement change in `spec-delta.md`; do not add a seventh spec file.

**Acceptance - done when** - the owned runtime:

- provides the stdout-silent bounded `dashboard claude-hook` command and
  harmless success behavior for every expected drop/unavailable case;
- binds the T02 user-scoped Unix socket before adapters, never uses a shared
  fallback, and does not make OpenCode startup depend on Claude bind success;
- safely handles stale/non-socket/symlink paths, bounded/timed/concurrent
  connections, one frame per connection, category-only logs, and shutdown
  cleanup;
- submits only T03-decoded typed envelopes and never maps lifecycle events or
  constructs shared snapshots in runtime code;
- includes a real feature test proving command -> Unix socket -> T03 decoder ->
  live adapter/provider-neutral event, plus invalid input, timeout,
  saturation, cleanup, and no-leak coverage;
- keeps T02/T03 behavior, OpenCode behavior, all four T05 deferrals, and
  no-global-config/no-transcript/no-session-control boundaries unchanged; and
- synchronizes the three affected specs and writes the spec delta without
  creating another spec file.

No T04 criterion may claim authenticated Claude behavior, final staleness
policy, support for an event outside `SessionStart`/`StopFailure`/`SessionEnd`,
or complete dashboard coverage.

**Testing** - run `cargo test -p dashboard --test claude_runtime`, the T02/T03
targeted tests, `cargo test -p dashboard --all-targets`, dashboard clippy with
`-D warnings`, `cargo fmt --all -- --check`, `cargo check --workspace`, and
`git diff --check`. The mandatory spec validator must inspect
`docs/specs/dashboard/claude.md` and report all rubric items passing. No test
may access `~/.claude`, project `.claude`, credentials, or transcript JSONL.

**Gate** - one implementation pass, one separate spec validation, and one
fresh independent Luna verification. T05 remains the owner of authenticated
full hook-to-dashboard E2E; no T05 work is pulled into T04.

## Review Frame

**As of** - contract version 1

**Context** - Runtime-only Candidate B connects T02 ingress to T03 through a user-scoped listener and exact hook command.

**Expectations** - Approved: isolated `main.rs` dispatch/startup hunks only; preserve the icon-mode dirty hunk verbatim. Accept bounded command/listener, safe cleanup, manual docs, and unchanged OpenCode flow. No lifecycle mapping, payload retention, or new events.

**Depth** - Deep review of runtime bounds, startup/shutdown, feature seam, and dirty-hunk isolation. T05 alone owns authenticated CLI E2E and final staleness; T02/T03 remain authoritative.
