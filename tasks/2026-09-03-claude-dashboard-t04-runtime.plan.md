# T04 Claude Runtime Implementation Plan

## Purpose

Wire the already implemented T02 hook ingress and T03 adapter into the
dashboard runtime. Add the `dashboard claude-hook` command, a user-scoped local
Unix listener, bounded frame handling, startup/shutdown composition, manual
configuration/removal documentation, and executable runtime tests.

## File Tree

```text
crates/dashboard/src/claude/
  command.rs             # bounded stdin hook command
  listener.rs            # socket bind, frame intake, decode, channel submit
  mod.rs                 # expose T04 runtime modules and T03 APIs
crates/dashboard/src/main.rs       # subcommand dispatch and startup ordering
crates/dashboard/Cargo.toml        # Tokio io/net feature requirements
crates/dashboard/tests/claude_runtime.rs  # command/listener feature tests
docs/specs/dashboard/claude.md    # concrete runtime/config/removal behavior
docs/specs/dashboard/client.md    # listener is wired, adapter remains opt-in
docs/specs/dashboard/overview.md  # runtime capability status
tasks/2026-09-03-claude-dashboard-support/spec-delta.md
```

`main.rs` is pre-existing dirty work. Terra must approve the narrow T04
re-scope before implementation; the icon-mode hunk is not T04-owned.

## Data Flow

```text
argv == [dashboard, claude-hook]
  -> command reads bounded stdin
  -> T02 parse_hook_input(ReceivedAt::now())
  -> T02 deliver (one bounded envelope)

dashboard startup
  -> resolve T02 user-scoped socket
  -> prepare stale socket / bind UnixListener
  -> start listener task and T03 ClaudeAdapter
  -> start existing OpenCode adapter
  -> shell receives shared SessionEvent channel
```

Listener connections follow:

```text
accept -> acquire bounded slot -> bounded timed frame read
       -> UTF-8 + T03 decode_envelope
       -> typed ClaudeIpcEnvelope sender
       -> category-only drop on any rejection
```

## Public Contracts

```text
ClaudeHookCommand::run() -> ExitCode
ClaudeListener::bind() -> Result<ClaudeListener, ListenerError>
ClaudeListener::bind_at(path) -> Result<ClaudeListener, ListenerError>
ClaudeListener::run(self, sender) -> JoinHandle<()>
```

`ListenerError` carries only a stable category, not a filesystem path or
payload. The listener sends typed T02 envelopes to `ClaudeAdapter::channel`;
it never constructs snapshots or maps lifecycle events.

## Bounds And Errors

- Hook stdin is capped at `MAX_HOOK_INPUT_BYTES + 1`; over-limit, invalid UTF-8,
  malformed JSON, unknown events, and unavailable delivery all exit success.
- Listener frames are capped at `MAX_ENVELOPE_BYTES`, one frame per connection,
  and use a finite read timeout. A fixed semaphore bounds concurrent clients.
- A stale socket may be removed only after `symlink_metadata` identifies a
  socket. A regular file, symlink, failed parent preparation, or bind failure
  yields a category-only unavailable result and leaves the dashboard usable.
- Decode failures never reach T03 state. Listener accept/read/bind failures do
  not print paths, JSON, or OS error text that could contain user data.
- Socket cleanup is best effort on listener shutdown; failure is category-only.

## Startup And Command Rules

- The exact first argument `claude-hook` selects helper mode. It must not pair
  with OpenCode, enter the terminal, start a listener, or emit stdout.
- Normal dashboard mode binds the listener before starting either adapter. If
  binding fails, the existing OpenCode dashboard still starts.
- The listener's user-scoped path is exactly the T02 resolver result. T04 does
  not introduce another path, fallback, permission widening, or settings write.
- No hook is installed automatically. Documentation gives the user the
  supported event list, manual settings shape, scope choices, removal steps,
  and opt-in behavior.

## Documentation Changes

Update R11/R12/R16/R17 in `claude.md` only where runtime behavior becomes
observable: the exact helper command, listener availability, bounded drops,
shutdown, manual install/removal, and the continued completeness/authenticated
limitations. Update `client.md` and `overview.md` to remove the stale claim that
listener wiring is not present while retaining opt-in and T05 review markers.
Write `spec-delta.md` listing each modified requirement and its reason.

## Testing Strategy

- FEATURE TEST: spawn the built `dashboard claude-hook` command with a real
  Unix listener and isolated socket path, feed a valid JSON hook payload, assert
  exit 0/no stdout, read exactly one newline envelope, decode it, and observe
  the T03 adapter event.
- Test malformed/unknown/oversized hook input exits 0 with no frame and no
  sentinel in output.
- Test listener valid delivery, malformed/unknown/out-of-bounds drops,
  unterminated/silent timeout, multiple-frame rejection, and later-valid-client
  survival.
- Test stale socket replacement, regular-file/symlink refusal, socket cleanup,
  and bind failure without affecting other tests.
- Test concurrent short-lived clients within the bound and connection-slot
  saturation behavior.
- Run targeted runtime/adapter/ingress tests, all dashboard targets, workspace
  check, clippy, and format. No Claude CLI/auth/config/transcript access is
  permitted in T04.

## Must Not Change

- `crates/dashboard/src/claude/hook.rs`, `state.rs`, `wire.rs` behavior;
  provider-neutral adapter/snapshot/project identity types; OpenCode sources;
  shell implementation; user/project Claude configuration; transcripts;
  existing unrelated dirty files, especially the icon-mode hunk in `main.rs`.
- T01c evidence, T02/T03 gates, the four T05 deferrals, or authenticated claims.

## Acceptance Criteria

- [ ] `dashboard claude-hook` is a harmless, stdout-silent, bounded command
      path that invokes T02 parsing/delivery and always exits success for
      expected drop/unavailable inputs.
- [ ] Normal dashboard startup binds the T02 user-scoped Unix socket before
      adapters, without making startup depend on Claude monitoring availability.
- [ ] Listener frame intake is bounded, timed, one-frame-per-connection,
      category-only, and submits only T03 typed envelopes.
- [ ] Stale socket cleanup is safe; non-socket/symlink paths are not deleted;
      listener shutdown removes its socket best-effort.
- [ ] Real feature tests prove command -> socket -> decode -> adapter events,
      plus invalid-input, timeout, concurrency, and cleanup behavior.
- [ ] R11/R12/R16/R17 documentation and spec delta match the runtime behavior,
      with manual opt-in/removal and T05 limitations intact.
- [ ] Existing OpenCode behavior and all quality gates remain green.

## Verification Checkpoints

| After | Verify | Fail action |
|---|---|---|
| Command helper | bounded stdin, exit 0, no stdout, no payload logs | Fix command boundary |
| Listener | real socket, frame bounds, decoder/channel path, cleanup | Fix listener before main wiring |
| Main composition | hook mode bypasses OpenCode; normal mode remains intact | Preserve dirty hunk and fix narrow dispatch |
| Docs/spec delta | requirements/scenarios and runtime claims agree | Correct docs and rerun spec validation |
| Full gate | targeted + workspace checks and independent review pass | Do not expose T05/T06 work |
