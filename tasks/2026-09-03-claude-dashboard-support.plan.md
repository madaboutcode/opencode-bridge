# Implementation Plan: Claude Code Dashboard Monitoring

**Status:** scope approved 2026-09-03; evidence spikes required before implementation

## 1. Purpose

Add Claude Code as a second, read-only harness observed by the dashboard. The
dashboard will show live Claude session metadata alongside OpenCode sessions
without reading Claude transcripts, controlling Claude sessions, or changing
Claude configuration.

The integration is event-driven. Claude Code invokes a command hook, the hook
forwards a small allowlisted envelope over a local Unix socket, and the Claude
adapter converts that envelope into the existing provider-neutral snapshot
contract.

## 2. Validated Context

- Claude command hooks receive lifecycle JSON on stdin. HTTP hooks receive the
  same JSON as a POST body. Source: official hooks reference.
- A tested `SessionStart` payload contained `session_id`, `cwd`,
  `hook_event_name`, `source`, and `transcript_path`.
- Claude transcripts are persisted JSONL, but their format is documented as
  internal and version-unstable. They are not a supported source for this
  integration.
- `claude agents --json` reports active background sessions, not every
  interactive foreground session. It is not an authoritative source for the
  dashboard's live session set.
- The existing `HarnessAdapter` accepts whole-session snapshots or `Gone`
  tombstones. `SessionId` already includes the harness kind, and project
  identity already resolves from the canonical git root with a working
  directory fallback.
- All Claude experiments must use isolated `HOME` and `CLAUDE_CONFIG_DIR`.
  The real `~/.claude` must remain untouched.

References:

- https://code.claude.com/docs/en/hooks
- https://code.claude.com/docs/en/sessions
- https://code.claude.com/docs/en/cli-reference
- `crates/dashboard/src/adapter.rs`
- `crates/dashboard/src/snapshot.rs`
- `docs/specs/dashboard/client.md`

## 3. File Tree

Future implementation changes are expected in this area:

```text
crates/dashboard/src/
  adapter.rs                 # only if the shared boundary needs a narrow addition
  claude/
    mod.rs                   # Claude adapter and lifecycle state
    hook.rs                  # stdin parser and local IPC client
  main.rs                    # claude-hook subcommand and listener startup
  snapshot.rs                # only if existing snapshot fields cannot represent metadata-only state

crates/dashboard/tests/
  claude_monitoring.rs       # feature verification through the IPC boundary

docs/specs/dashboard/
  claude.md                  # Claude-specific hook and privacy contract
```

This plan file is the only artifact created in the planning phase. No Claude
configuration or feature code is changed by this plan.

## 4. Data Models

### 4.1 Hook input boundary

The hook helper parses Claude's JSON into an internal allowlisted record and
does not retain the original `serde_json::Value`.

```text
ClaudeHookRecord {
  session_id: non-empty opaque string
  cwd: non-empty path
  event: ClaudeEvent
  received_at: dashboard Timestamp
}

ClaudeEvent:
  SessionStart { source: optional enum }
  UserPromptSubmit
  PermissionRequest { tool_name: non-empty string }
  PreToolUse { tool_name: non-empty string }
  PostToolUse { tool_name: non-empty string }
  PostToolUseFailure { tool_name: non-empty string }
  Notification { notification_type: optional enum }
  SubagentStart { agent_id: optional opaque string, agent_type: optional string }
  SubagentStop { agent_id: optional opaque string, agent_type: optional string }
  Stop
  StopFailure { error_type: optional enum }
  CwdChanged
  SessionEnd { reason: optional enum }
```

The parser may accept additional documented common fields for validation, but
must discard these values before the record crosses the hook boundary:

- `transcript_path` and `agent_transcript_path`
- prompt text and `last_assistant_message`
- `tool_input` and `tool_response`
- arbitrary unknown fields

The accepted event set is finalized by the lifecycle spike. Unsupported or
malformed input is ignored as an observational no-op.

### 4.2 Local IPC envelope

The helper sends only a versioned `ClaudeHookRecord` over a user-owned Unix
socket. The envelope is newline-delimited for simple short-lived command
clients. It has a bounded size and contains no raw hook JSON.

```text
ClaudeIpcEnvelope {
  protocol_version: integer
  record: ClaudeHookRecord
}
```

### 4.3 Adapter state

The adapter owns the mutable state needed to build snapshots:

```text
ClaudeTrackedSession {
  session_id: SessionId                 # HarnessKind("claude") + native id
  project_id: ProjectId                 # canonical cwd identity
  created_at: Timestamp                 # first accepted SessionStart, or first event
  last_seen: Timestamp                  # local hook receipt time
  turn_started: optional Timestamp
  attention: AttentionState
  current_action: optional string        # e.g. "tool: Edit", no arguments
  recent_actions: bounded list<string>  # tool labels only
  parent_id: optional SessionId          # only after Subagent identity is verified
}
```

Snapshot mapping is metadata-only:

- `wire_title`: `None`; Claude title generation is not read from transcripts.
- `final_assistant_text`: `None`.
- `last_user_prompt`: `None`.
- `files_touched`: empty.
- `current_action`: tool name only, never tool arguments or results.
- `recent_actions`: bounded tool-name labels only.
- `last_updated`: local receipt time, not a Claude transcript timestamp.

## 5. Orchestration

### 5.1 User-configured hook path

The user manually adds command handlers to their Claude settings at the
desired scope. The dashboard provides documentation and a validation command;
it does not write or register those handlers.

Conceptual handler:

```json
{
  "type": "command",
  "command": "dashboard claude-hook",
  "async": true
}
```

The final documentation must include the event list, installation scope,
removal instructions, and the fact that monitoring is inactive unless the
user opts in.

### 5.2 Runtime flow

1. The dashboard starts its local Unix socket listener before starting the
   Claude adapter and other adapters.
2. Claude Code fires a configured lifecycle hook.
3. Claude launches `dashboard claude-hook`; Claude writes event JSON to the
   helper's stdin.
4. The helper parses the allowlist, drops sensitive fields, and sends one
   bounded envelope to the socket.
5. The adapter validates the envelope, updates its per-session state, and
   resolves project identity from `cwd` using the existing cache.
6. The adapter emits one complete `SessionSnapshot` for every accepted
   non-terminal event.
7. `SessionEnd` emits `SessionEvent::Gone` and releases the session from the
   adapter's live state.
8. The core continues to consume provider-neutral events and does not import
   Claude hook types.

### 5.3 Lifecycle mapping

The default mapping is:

| Claude event | Dashboard effect |
|---|---|
| `SessionStart` | Admit a live session in a waiting/needs-you state |
| `UserPromptSubmit` | Mark the turn running and set `turn_started` |
| `PermissionRequest` | Mark the session needs-you; retain no request details |
| `PreToolUse` | Mark running and set `current_action` to the tool name |
| `PostToolUse` / `PostToolUseFailure` | Retain a tool-name action and remain running until the next lifecycle signal |
| `Notification` | Apply only documented notification mappings confirmed by the spike |
| `Stop` / `StopFailure` | Mark the live session needs-you; retain no response text |
| `SubagentStart` / `SubagentStop` | Track child state only if the payload provides a stable identity and parent relation |
| `CwdChanged` | Re-resolve project identity on the next accepted event if `cwd` changed |
| `SessionEnd` | Emit `Gone` |

"Running only" means no historical or completed sessions are loaded. It does
not mean only turns currently generating output are shown: a live Claude
process waiting for its next prompt remains observable until `SessionEnd` or
the agreed staleness rule removes it.

### 5.4 Failure and degraded behavior

- The hook helper is observational and must never deny, approve, rewrite, or
  delay a Claude action.
- Prefer asynchronous command hooks. If the tested Claude version cannot run
  this handler asynchronously for a required event, use a very small timeout
  and a best-effort synchronous fallback.
- If the dashboard is not running, the helper exits successfully after a
  bounded socket-connect attempt. It does not create a transcript reader or a
  persistent event journal in MVP.
- A malformed event, unknown event, unavailable socket, or full listener is a
  logged/drop case, not a Claude session failure.
- The adapter must remain alive if one event is bad or one session's project
  path cannot be resolved.

## 6. Boundaries

### IN SCOPE

- Add a second `HarnessAdapter` for Claude Code.
- Add a read-only command-hook ingress path.
- Observe user-configured lifecycle hooks over a local Unix socket.
- Track live Claude session identity, project, lifecycle status, and tool-name
  metadata in memory.
- Emit provider-neutral snapshots and `Gone` tombstones.
- Show Claude sessions alongside OpenCode sessions without identity collision.
- Document manual hook configuration, validation, removal, and degraded mode.
- Add unit, IPC, adapter, and isolated Claude CLI evidence tests.

### OUT OF SCOPE

- Reading, tailing, parsing, or summarizing Claude transcript JSONL.
- Discovering historical, completed, or persisted sessions.
- Controlling Claude sessions: resume, attach, send prompt, interrupt, stop,
  delete, or mutate.
- Writing `~/.claude`, `.claude/settings.json`, `.claude/settings.local.json`,
  managed settings, or any other Claude configuration.
- Installing hooks automatically.
- Sending prompts, assistant output, tool arguments, tool results, file
  contents, transcript paths, or secrets to the dashboard.
- A public network listener or remote monitoring endpoint.
- Treating `claude agents --json` as a universal session source.
- Session zoom or transcript trace views.

### MUST NOT CHANGE

- The real user Claude configuration or transcript files.
- Existing OpenCode adapter behavior and wire interpretation.
- The provider-neutral core's dependence on `HarnessAdapter` and snapshots.
- The OpenCode client's dependency boundary: no TUI or MCP dependencies.
- Existing unrelated dirty worktree changes.

### MUST FOLLOW

- Use `HarnessKind("claude")` in every Claude `SessionId`.
- Resolve project identity from the event `cwd` with the shared canonical
  project identity logic; never use a Claude transcript directory name as a
  project key.
- Keep all Claude JSON parsing, event ordering, and tool vocabulary inside the
  Claude adapter/ingress modules.
- Use strict allowlisting and bounded field lengths at the hook boundary.
- Treat hook delivery as best-effort and non-blocking for Claude.
- Keep all experiments isolated with temporary `HOME` and
  `CLAUDE_CONFIG_DIR`.
- Preserve the whole-snapshot plus tombstone delivery model.

## 7. Pre-Decisions

### Capability

**Decision:** Read-only monitoring only.

**Rationale:** The dashboard is an observer, not a second Claude client. This
avoids control races, permission semantics, and accidental mutation.

### Session population

**Decision:** Live session lifecycles only; no history discovery.

**Rationale:** A hook-admitted session can be monitored without reading
Claude's internal persistence. Completed sessions and sessions never observed
by the dashboard are intentionally absent.

### Data exposure

**Decision:** Metadata only.

**Rationale:** Session identity, working directory, lifecycle state, and tool
names are sufficient for the dashboard. Prompt/response content and raw tool
payloads create unnecessary privacy and secret-leakage risk.

### Hook ownership

**Decision:** The user configures and removes hooks.

**Rationale:** No product process should silently modify Claude settings. The
dashboard owns only its runtime listener and can provide copyable instructions.

### Hook transport

**Decision:** Command hook to a user-scoped Unix socket for MVP.

**Rationale:** Command hooks are already verified on the target Claude version,
avoid a public TCP port, and let the helper fail harmlessly when the dashboard
is absent. HTTP hooks remain a possible later transport if evidence shows a
clear operational advantage.

### Session versus turn

**Decision:** A live session remains visible while waiting between turns.

**Rationale:** `Stop` ends a turn, not necessarily the Claude process. The
dashboard needs a needs-you state for a live session and uses `SessionEnd` for
session removal.

### Startup gap

**Decision:** MVP does not replay missed hook events or scan Claude history.

**Rationale:** This keeps the implementation metadata-only and avoids building
a second persistence system. A dashboard started after `SessionStart` sees the
session only when a later configured event arrives. Promotion trigger: this
causes a real monitoring failure for the user's normal workflow.

### Staleness

**Decision:** Do not invent the final staleness timeout before the lifecycle
spike. The implementation gate must choose a bounded stale-session policy from
observed `SessionEnd` reliability and the user's live-session workflow.

**Rationale:** Hooks are push-only and Claude has no hook heartbeat event. A
timeout that is too short removes a live session while a user is thinking; a
timeout that is too long leaves a ghost session. This is an explicit evidence
gate, not permission to silently retain stale state forever.

## 8. Spike Backlog

Each spike must produce runnable evidence and a decision. Spikes use isolated
temporary configuration only and must not modify the real home directory.

### S1. Hook Invocation Contract

- **Question:** Which supported Claude version features and exact payloads are
  available for the selected command hooks?
- **Scope:** `SessionStart`, `UserPromptSubmit`, `PermissionRequest`,
  `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `Notification`, `Stop`,
  `StopFailure`, `SubagentStart`, `SubagentStop`, `CwdChanged`, and
  `SessionEnd`; synchronous versus `async: true` command behavior.
- **Method:** Use temporary `HOME` and `CLAUDE_CONFIG_DIR`; install a disposable
  settings fixture that captures stdin to a temp file; run safe Claude CLI
  scenarios and record exit status, event name, field presence, and elapsed
  hook time. Never store content fields in evidence.
- **Evidence:** Versioned JSON fixtures with redacted payload schemas, event
  coverage table, async/timeout result, and exact supported-version floor.
- **Decision:** Final allowlisted event set and whether all handlers can be
  asynchronous in MVP.

### S2. Real Turn Lifecycle

- **Question:** What ordering and repetition occur for a successful turn, a
  failed turn, a permission wait, and a user-exited session?
- **Scope:** Start/resume, prompt submission, tool calls, stop/failure,
  notification, and session end. Include one authenticated successful turn;
  the earlier run did not complete this gate.
- **Method:** Run isolated interactive/headless scenarios with a no-op
  observer. Record only event names, session IDs, tool names, notification
  types, and timestamps. Do not capture prompts, responses, tool input, tool
  output, or transcript contents.
- **Evidence:** Ordered event traces and a negative assertion that sensitive
  fields never enter the observer record.
- **Decision:** Lifecycle-to-attention mapping and whether `Notification` or
  `PermissionRequest` is needed for needs-you accuracy.

### S3. Live Session Discovery and Startup Gap

- **Question:** Can a dashboard started after Claude sessions begin discover
  those still-live sessions without reading history?
- **Scope:** `SessionStart` hooks, later lifecycle hooks, `claude agents --json`,
  foreground sessions, background sessions, and dashboard restart.
- **Method:** Start sessions before and after the observer; compare hook events
  with `claude agents --json` output. Test both interactive and background
  sessions. Do not parse transcript files.
- **Evidence:** Matrix of session type versus discoverability, with command
  version and limitations.
- **Decision:** Confirm the no-replay MVP behavior or promote a separate
  metadata-only observer journal as required scope.

### S4. Session-End Reliability and Staleness

- **Question:** Under ordinary exit, interrupt, terminal close, crash, and
  machine sleep, is `SessionEnd` delivered, and what bounded timeout prevents
  ghost sessions?
- **Scope:** Hook delivery failures, long user pauses, dashboard disconnects,
  and reappearance after a later event.
- **Method:** Run isolated sessions through each exit path; record only event
  names and receipt times. Repeat with the dashboard helper unavailable and
  available. Do not infer liveness from transcript file timestamps.
- **Evidence:** Exit-path table, observed event gaps, and a testable timeout
  recommendation.
- **Decision:** Stale handling: removal versus dimming, threshold, claim-map
  release behavior, and whether the existing active-window setting may be
  reused. This decision is required before the adapter is declared complete.

### S5. Identity, CWD, and Subagents

- **Question:** Which fields identify a parent session, child session, and
  project across resume, `/cd`, symlinks, repositories, and worktrees?
- **Scope:** `session_id`, `cwd`, `agent_id`, `agent_type`, subagent hooks,
  `CwdChanged`, and parent relations.
- **Method:** Use temporary repositories and worktrees plus isolated Claude
  sessions. Compare hook metadata to filesystem paths and the existing project
  identity resolver. Do not use transcript paths as identity.
- **Evidence:** Identity matrix and fixtures for root, subfolder, worktree,
  symlink, resumed session, and subagent cases.
- **Decision:** Whether Claude children can be represented by existing
  `parent_id`, and the exact fallback when parent identity is absent.

### S6. Local IPC and Hook Failure Semantics

- **Question:** Does the helper remain fast, safe, and race-free when the
  listener is absent, restarting, concurrent, or fed malformed input?
- **Scope:** Unix socket path selection, permissions, stale socket cleanup,
  concurrent hook processes, bounded writes, malformed JSON, oversized fields,
  and helper exit behavior.
- **Method:** Build a throwaway listener and helper harness. Send concurrent
  fixtures containing sensitive fields and malformed envelopes. Measure helper
  latency and assert no sensitive field crosses the IPC boundary.
- **Evidence:** IPC test results, permissions check, latency bound, and failure
  matrix.
- **Decision:** Socket location, framing, version handling, max record size,
  and exact best-effort exit policy.

### S7. Privacy and Boundary Audit

- **Question:** Can any transcript or raw hook content reach the core, logs, or
  persistent dashboard state accidentally?
- **Scope:** Parser, IPC envelope, adapter state, snapshots, debug logging,
  error paths, and test fixtures.
- **Method:** Run fixtures containing unique sentinel strings in every rejected
  field; inspect serialized envelopes, logs, snapshots, and process arguments.
  Review imports to ensure Claude-specific types stop at the adapter.
- **Evidence:** Automated negative tests plus a manual data-flow checklist.
- **Decision:** Release approval for metadata-only exposure and the final list
  of fields allowed in logs.

## 9. Testing Strategy

- **FEATURE TEST:** send a real `SessionStart`, `UserPromptSubmit`,
  `PreToolUse`, `Stop`, and `SessionEnd` fixture through the local socket and
  assert the adapter emits snapshots with `HarnessKind("claude")`, the
  canonical project ID, the expected attention transitions, tool-name-only
  action text, and a final `Gone`. This test must fail if the Claude adapter or
  IPC path is removed.
- Parser unit tests cover every accepted event, missing optional fields,
  malformed JSON, unknown events, oversized values, and rejected sensitive
  fields.
- Adapter unit tests cover session admission, repeated events, turn state,
  project changes, session removal, duplicate IDs across harnesses, and the
  selected stale policy.
- IPC integration tests use a real Unix socket and concurrent short-lived
  helper processes. No TCP listener is required for MVP.
- Isolated CLI tests use temporary `HOME` and `CLAUDE_CONFIG_DIR`; they verify
  documented hook behavior without reading or asserting transcript internals.
- Existing OpenCode tests and workspace checks remain regression gates:
  `cargo test --workspace --all-targets --locked`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo fmt --all -- --check`.

## 10. Verification Checkpoints

| After step | Verify by | Failure action |
|---|---|---|
| S1-S2 evidence captured | Isolated hook traces match documented schemas and include a successful turn | Do not define the parser contract yet; update the event matrix |
| S3 decision captured | Startup/restart behavior is explicit and reproducible | Keep history scanning and replay out of scope until separately approved |
| S4 decision captured | Exit-path table and stale policy exist | Do not ship a push-only adapter with indefinite live state |
| S5 decision captured | Identity fixtures resolve consistently | Fix the adapter contract before wiring the core |
| S6 complete | Real socket, malformed input, absent listener, and concurrency tests pass | Fix IPC boundary before adapter work |
| S7 complete | Sentinel data is absent from envelopes, logs, snapshots, and state | Remove the leaking field/path before implementation proceeds |
| Adapter implemented | Feature verification test passes | Debug adapter/contract root cause; do not rely on existing OpenCode tests |
| Startup wired | Dashboard runs with no hooks and OpenCode behavior is unchanged | Revert only the new wiring, never user configuration |
| Manual configuration documented | User can add and remove hooks without an installer | Correct docs and validation command |

## 11. Migration and Rollback

### Data migration

- None. Claude monitoring state is in memory only.
- No transcript migration, indexing, or new persistent database is required.

### Configuration migration

- None performed by the binary.
- The user adds the hook entries manually and removes them manually to disable
  monitoring.
- The dashboard remains functional with no Claude hooks configured.

### Code rollback

- Remove or disable the Claude adapter and hook subcommand; the OpenCode
  adapter remains the existing path.
- No rollback needs to repair Claude settings because the dashboard never writes
  them.
- If IPC or privacy tests fail, do not expose the adapter in startup; keep the
  code behind the unadvertised command path until corrected.

## 12. Acceptance Criteria

- [ ] Claude monitoring is opt-in and requires user-configured hooks.
- [ ] The dashboard never writes or registers Claude configuration.
- [ ] Only live hook-observed sessions are shown; no history or transcript scan
      occurs.
- [ ] The core receives only provider-neutral snapshots and tombstones.
- [ ] Claude and OpenCode sessions with the same native ID remain distinct.
- [ ] Project grouping uses canonical `cwd` identity.
- [ ] Session, turn, tool, permission, and end states follow the evidence-backed
      lifecycle mapping.
- [ ] The dashboard exposes metadata only: identity, project, lifecycle, and
      tool names; no prompt/response/tool payload/transcript data crosses the
      boundary.
- [ ] Missing dashboard, malformed hook input, and one bad session do not
      affect Claude execution or other dashboard sessions.
- [ ] Session-end and stale-session behavior is bounded and covered by tests.
- [ ] The feature verification test passes through the real local IPC boundary.
- [ ] Existing OpenCode tests and workspace quality gates pass without new
      failures.

## 13. Task Breakdown

### Task 1: Close hook evidence gates

- **What:** Run S1-S5 in isolated Claude configuration and record payload,
  lifecycle, startup-gap, staleness, and identity decisions.
- **Files:** `tasks/spikes/` evidence files; no production code.
- **Depends on:** none.
- **Agent:** you.
- **Verify:** Every spike has runnable evidence and an explicit decision; no
  real `~/.claude` path appears in commands or artifacts.

### Task 2: Specify and test the local ingress

- **What:** Complete S6-S7, then define the bounded allowlist, IPC envelope,
  socket lifecycle, and privacy assertions.
- **Files:** future `crates/dashboard/src/claude/hook.rs`, tests, and
  `docs/specs/dashboard/claude.md`.
- **Depends on:** Task 1.
- **Agent:** you.
- **Verify:** Real socket tests pass, absent-listener behavior is harmless, and
  sentinel content is absent from all outgoing records.

### Task 3: Implement the Claude adapter

- **What:** Add lifecycle state, project resolution, snapshot construction,
  tombstones, and the evidence-backed stale policy behind `HarnessAdapter`.
- **Files:** future `crates/dashboard/src/claude/mod.rs` and only the minimal
  shared boundary changes required by the contract.
- **Depends on:** Tasks 1-2.
- **Agent:** you.
- **Verify:** Unit tests cover transitions and the feature verification test
  receives expected snapshots.

### Task 4: Wire opt-in runtime support

- **What:** Start the listener and adapter without changing the existing
  OpenCode startup path; add the manual configuration and validation docs.
- **Files:** future `crates/dashboard/src/main.rs`, docs, and startup tests.
- **Depends on:** Task 3.
- **Agent:** you.
- **Verify:** Dashboard starts with no Claude hooks, mixed Claude/OpenCode
  fixtures render, and user instructions add/remove hooks without an installer.

### Task 5: Run release regression gates

- **What:** Run the feature, isolated CLI, workspace test, clippy, and format
  checks; review the privacy and rollback evidence.
- **Files:** gate report only.
- **Depends on:** Task 4.
- **Agent:** you.
- **Verify:** All acceptance criteria pass, with any deferred issue recorded
  explicitly rather than hidden behind existing OpenCode coverage.

## 14. Boundary Sanity Check

- **Root cause:** Claude Code has no dashboard-facing session API for all live
  local sessions, so a hook observer is the supported integration seam rather
  than a transcript parser or process scraper.
- **Cleanliness:** Claude-specific parsing and lifecycle state stay in one
  adapter; the shared core already has the correct snapshot boundary.
- **No workaround through the wrong layer:** project identity remains a shared
  filesystem identity concern, while Claude event semantics and privacy
  filtering remain adapter concerns.
