# T04 Claude Runtime Design

## Problem Model

T02 provides a best-effort command-hook sender and user-scoped Unix-socket
address. T03 provides a typed decoder and a `ClaudeAdapter` channel, but no
process owns the socket yet. T04 closes that runtime gap: the dashboard binary
starts a local listener, each bounded connection is decoded, and valid
envelopes are submitted to T03 without teaching runtime code Claude lifecycle
semantics.

The command hook is executable as `dashboard claude-hook`. It reads one bounded
JSON payload from stdin, delegates parsing and delivery to T02, and returns
success for malformed input or unavailable listeners so Claude is never blocked
by monitoring.

Invariants: no public network socket; one user-scoped socket path only; no
Claude configuration writes or reads; no transcript access; one bounded frame
per connection; category-only runtime logs; valid frames reach only the T03
typed channel; shutdown removes the runtime socket; and OpenCode startup keeps
its existing behavior.

## Discovery Memo

`hook::claude_socket_path` is the single path resolver, `hook::deliver` is the
command-side best-effort sender, `wire::decode_envelope` is the strict decoder,
and `ClaudeAdapter::channel` is the typed receiver seam. `main.rs` builds the
Tokio runtime, starts adapters, and then enters the blocking shell loop.

`crates/dashboard/src/main.rs` has pre-existing uncommitted icon-mode changes.
T04 needs that binary's argument dispatch and startup ordering, so Terra must
explicitly approve a narrow re-scope. T04 may add isolated hunks only; it must
not rewrite, reformat, or stage the existing icon-mode hunk.

Path preparation or bind failure disables Claude monitoring but does not prevent
the OpenCode dashboard from starting. A bad client frame is dropped without
affecting other connections. A client that never completes a frame is closed
after a bounded read deadline. A closed T03 channel ends listener forwarding;
runtime shutdown drops the listener and cleans its socket.

## Candidate Decompositions

### Candidate A: Listener In `main.rs`

Keep socket binding, frame reads, decoding, command handling, and startup logic
in the binary. This minimizes files but makes the binary own protocol details,
is hard to feature-test without dashboard startup, and increases the risk of
touching unrelated main changes.

### Candidate B: Claude Command And Listener Modules

Add `claude/command.rs` for the hook subprocess path and `claude/listener.rs`
for socket lifecycle, bounded connection handling, and decoder submission.
`main.rs` only dispatches the command and starts the listener/adapter before
OpenCode. This keeps runtime mechanics testable and leaves T03 lifecycle
mapping untouched.

### Candidate C: Separate Listener Binary

Add a second binary and change documented hooks to invoke it. This avoids main
dispatch but changes the user-facing command, complicates installation, and
does not satisfy the existing `dashboard claude-hook` contract.

## Recommendation

Choose Candidate B. It is the smallest decomposition that keeps the command and
listener independently testable while making `main.rs` a narrow composition
point. The listener consumes T03's public decoder and channel only; it never
maps `SessionStart`, `StopFailure`, or `SessionEnd` itself.

## Layer Stack

```text
Claude Code command hook
  -> dashboard claude-hook / command.rs
  -> T02 parse + best-effort deliver
  -> user-scoped Unix socket / listener.rs
  -> T03 decode_envelope
  -> T03 ClaudeAdapter channel
  -> provider-neutral SessionEvent / dashboard shell
```

## Component Contracts

### `claude::command`

- Reads at most `MAX_HOOK_INPUT_BYTES + 1` bytes from stdin and never prints to
  stdout.
- Delegates valid UTF-8 input to T02 with a local receipt time, calls T02's
  `deliver`, and exits successfully for every expected malformed/drop/
  unavailable case.
- Logs only categories and never payload values.
- Does not read settings, transcripts, or credentials.

### `claude::listener`

- Binds only the path returned by T02's user-scoped resolver; no shared temp
  fallback and no public TCP endpoint.
- Removes a stale socket file only when `symlink_metadata` proves it is a Unix
  socket; refuses a non-socket or symlink path rather than deleting it.
- Accepts independent short-lived connections, bounds each frame at
  `MAX_ENVELOPE_BYTES`, enforces one frame and a read deadline, decodes through
  T03, and submits only valid typed envelopes to the T03 sender.
- Drops bad frames and saturated connection slots with category-only logs;
  successful and failed clients cannot affect each other.
- Removes its owned socket on listener task shutdown.

### `main.rs` composition

- Recognizes the exact `claude-hook` subcommand before OpenCode pairing or TUI
  startup and runs only the command helper.
- For the dashboard path, binds the Claude listener before starting the Claude
  adapter and OpenCode adapter. A bind failure is a harmless disabled Claude
  capability, not a dashboard startup failure.
- Preserves the pre-existing icon-mode code and all OpenCode behavior.

## Bounds And Shutdown

Use a fixed per-connection read buffer, a maximum concurrent connection count,
and a finite frame-read timeout. The listener processes at most one envelope per
connection. Runtime shutdown drops the listener task and removes the socket;
the adapter remains the owner of lifecycle state and the shared event sink.

## Validation Scenarios

- `dashboard claude-hook` receives a valid payload through stdin, exits 0, and
  delivers one T02 envelope to a real user-scoped test socket.
- The hook command receives malformed, oversized, or unknown events and exits 0
  without writing a frame or leaking values to stdout/stderr.
- A real listener accepts a valid T02 frame, decodes it, and submits it to a
  live T03 adapter channel; the provider-neutral event is observed downstream.
- A malformed, unknown-version, oversized, multi-frame, or unterminated client
  connection is dropped and a later valid client still succeeds.
- A stale socket is replaced safely; a regular file or symlink at the target is
  not deleted and disables only Claude monitoring.
- More than the connection bound cannot starve valid clients, and a silent
  client is released after the read deadline.
- Dropping/aborting the listener removes its socket and does not alter
  OpenCode's existing startup path.

## Open Questions And Deferrals

T05 still owns authenticated Claude lifecycle coverage, async-hook viability,
exit-path reliability, subagent identity, final stale-session policy, and the
complete authenticated hook-to-dashboard proof. T04 does not promote any new
event or alter T02/T03 metadata policy.
