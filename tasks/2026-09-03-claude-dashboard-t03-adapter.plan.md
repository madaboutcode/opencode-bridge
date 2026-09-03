# T03 Claude Adapter Implementation Plan

## Purpose

Add the second `HarnessAdapter` implementation for the typed T02 Claude hook
envelope. It will keep Claude lifecycle state in memory, resolve project
identity through the shared cache, emit complete provider-neutral snapshots and
tombstones, and remain metadata-only while leaving listener startup to T04.

## File Tree

```text
crates/dashboard/src/claude/
  DESIGN.md              # promoted adapter design
  mod.rs                 # public adapter and channel loop
  state.rs               # Claude state transitions and snapshots
  wire.rs                # versioned envelope decoder
crates/dashboard/src/lib.rs       # register and export Claude adapter
crates/dashboard/tests/claude_adapter.rs  # feature verification
docs/specs/dashboard/client.md    # adapter boundary now includes opt-in Claude
docs/specs/dashboard/overview.md  # second adapter summary
```

## Data Models

```text
ClaudeTrackedSession {
  session_id: SessionId                 # HarnessKind("claude") + native id
  project_id: ProjectId                 # shared cwd identity
  created_at: Timestamp                 # first accepted event
  last_updated: Timestamp               # local hook receipt time
  attention: AttentionState             # provisional NeedsYou mapping
}

ClaudeAdapter {
  input: UnboundedReceiver<ClaudeIpcEnvelope>
}
```

Snapshots set `parent_id`, `current_action`, `wire_title`,
`final_assistant_text`, `last_user_prompt`, `files_touched`, and
`recent_actions` to empty/none because T01c does not verify those values. A
wire decoder returns a typed envelope or a category-only decode error.

## Orchestration

```text
ClaudeAdapter::channel() -> (UnboundedSender<ClaudeIpcEnvelope>, ClaudeAdapter)
ClaudeAdapter::run(sink) -> JoinHandle<()>
decode_envelope(line) -> Result<ClaudeIpcEnvelope, DecodeError>
```

1. T04 receives a bounded line from the local listener and calls the decoder.
2. The adapter loop receives validated envelopes in channel order.
3. State maps the envelope to a Claude session identity and resolves `cwd`
   through `ProjectIdentityCache<GitDirResolver>`.
4. `SessionStart` and `StopFailure` emit one complete `Snapshot`; `SessionEnd`
   removes state and emits `Gone`.
5. The adapter stores only metadata state. The core owns rendering,
   reclassification, and naming claims.

## Boundaries

### In Scope

- Add the Claude adapter, typed wire decoder, in-memory state transitions, and
  public library registration.
- Add a real feature test proving T02 delivery/decode reaches Claude snapshots
  and `Gone` through the provider-neutral channel.
- Update directly affected adapter/overview documentation.
- Preserve the provisional active-window/staleness behavior and record receipt
  timestamps for T05.

### Out Of Scope

- Unix listener startup, command-hook process dispatch, or dashboard startup
  changes; those belong to T04.
- New Claude event support beyond T01c's three observed events.
- Transcript/history access, configuration writes, persistence, replay, or
  session control.
- Authenticated behavior, final staleness policy, subagent semantics, or full
  hook-to-dashboard E2E; those remain T05/deferred.

### Must Not Change

- `SessionEvent`, `SessionSnapshot`, `ProjectIdentityCache`, or OpenCode source.
- Existing dirty files in the worktree.
- T01c evidence, T02 hook ingress, or Claude settings/transcripts.

### Must Follow

- Use `HarnessKind("claude")` and the whole-snapshot/tombstone boundary.
- Keep raw hook JSON and rejected fields out of state and logs.
- Use explicit validation at the wire boundary and cited degraded project
  identity fallback for a missing/unreadable cwd.
- Keep the adapter alive across one malformed wire record or bad project path.
- Put a CONTRACT block in each new source file and promote the design to
  `crates/dashboard/src/claude/DESIGN.md`.

## Pre-Decisions

### Lifecycle Mapping

Choice: `SessionStart` and `StopFailure` produce `NeedsYou { question: false }`;
`SessionEnd` produces `Gone`; all other events are impossible at this boundary
because T02 drops them.

Rationale: this is the only mapping supported by T01c's observed allowlist and
keeps unverified successful-turn semantics out of the adapter.

### Staleness

Choice: record `last_updated` from local receipt time and do not add an
adapter-side expiry timer or removal. The core's existing active-window
reclassification remains the provisional display behavior until T05 chooses a
final policy.

Rationale: the five-minute T01 value is explicitly provisional, while the core
already owns active-window reclassification and `Idle` construction.

### Invalid Wire Input

Choice: category-only drop with no `SessionEvent`; continue receiving later
records.

Rationale: listener input is observational and must not affect Claude or other
sessions.

## Testing Strategy

- FEATURE TEST: deliver a real `SessionStart`, `StopFailure`, and `SessionEnd`
  envelope through a real Unix socket, decode it, send it to the adapter, and
  assert Claude snapshot metadata, canonical project identity, attention
  transitions, and final `Gone`. This fails if the new adapter path is removed.
- Unit-test each lifecycle transition, duplicate start, first-event
  `StopFailure`, same native id across harness kinds, empty snapshot fields, and
  receipt timestamps.
- Unit-test decoder protocol/version/event/field bounds and rejected unknown
  values without retaining raw JSON.
- Test a missing project directory using a fixture resolver or the documented
  degraded fallback; prove the adapter continues with later input.
- Run existing OpenCode tests plus workspace test, clippy, and format checks;
  do not access Claude configuration or transcript JSONL.

## Verification Checkpoints

| After | Verify | Fail action |
|---|---|---|
| Wire decoder | malformed/unknown records produce no events | Fix boundary before state work |
| State transitions | unit tests cover all three supported events | Fix mapping before adapter loop |
| Adapter registration | library compiles and exposes `ClaudeAdapter` | Do not proceed to runtime wiring |
| Feature path | real socket -> decode -> adapter -> snapshot/tombstone test passes | Debug the new path, not existing OpenCode tests |
| Documentation | client/overview claims match opt-in library behavior | Correct docs before gate |

## Migration And Rollback

No data or configuration migration. The adapter owns in-memory state only.
Rollback removes the Claude module registration and adapter files; OpenCode
startup and behavior remain unchanged. T04 will not expose startup support until
the T03 gate is clean.

## Acceptance Criteria

- [ ] `ClaudeAdapter` implements `HarnessAdapter` with `HarnessKind("claude")`.
- [ ] Supported events produce the documented snapshots or `Gone`.
- [ ] Snapshots contain no transcript, prompt, assistant, tool payload, or
      arbitrary fields.
- [ ] Project identity uses the shared canonical resolver and one bad cwd does
      not stop the adapter.
- [ ] Wire input is versioned, bounded, and invalid records are harmless drops.
- [ ] The real feature test passes through socket delivery, decode, adapter,
      provider-neutral events, and tombstone.
- [ ] Existing OpenCode and workspace quality gates remain green.
- [ ] T05 constraints and four deferrals remain unchanged.

## Task Breakdown

### Task 1: Promote adapter design and wire contract

- **What:** Add the module design and decoder contract from this plan.
- **Files:** `crates/dashboard/src/claude/DESIGN.md`, `wire.rs`.
- **Depends on:** none.
- **Agent:** DeepSeek Flash.
- **Verify:** decoder unit tests pass.

### Task 2: Implement state and adapter loop

- **What:** Add lifecycle state, snapshot mapping, channel constructor, and
  `HarnessAdapter` implementation.
- **Files:** `state.rs`, `mod.rs`, `lib.rs`.
- **Depends on:** Task 1.
- **Agent:** DeepSeek Flash.
- **Verify:** adapter unit tests and compile pass.

### Task 3: Prove the feature path

- **What:** Exercise real T02 socket delivery through decoding and adapter
  events, including the terminal tombstone.
- **Files:** `crates/dashboard/tests/claude_adapter.rs`.
- **Depends on:** Tasks 1-2.
- **Agent:** DeepSeek Flash.
- **Verify:** targeted feature test passes and cannot pass without the adapter.

### Task 4: Align adapter documentation and gate

- **What:** Update directly affected adapter/overview claims and run quality
  gates before Luna review.
- **Files:** `docs/specs/dashboard/client.md`,
  `docs/specs/dashboard/overview.md`, gate report.
- **Depends on:** Tasks 1-3.
- **Agent:** conductor.
- **Verify:** documentation inspection and workspace checks pass.
