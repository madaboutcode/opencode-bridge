# EVIDENCE.md - T01 Isolated Claude hook and lifecycle evidence

**Contract version** - 2
**Date** - 2026-09-03
**Implementer** - deepseek-flash (via opencode CLI)

## S1: Hook Invocation Contract

### Decision
- Supported Claude version floor: **2.1.259** (observed `claude --version`).
- Exact supported event set (observed with unauthenticated CLI): **SessionStart**, **StopFailure**, **SessionEnd**. All other events (UserPromptSubmit, PreToolUse, PostToolUse, PermissionRequest, Notification, Stop, SubagentStart, SubagentStop, CwdChanged, etc.) are UNVERIFIED / not in the supported set until authenticated evidence exists.
- Payload field presence: verified per isolated trace — `SessionStart` with synchronous hooks; `StopFailure` and `SessionEnd` with asynchronous hooks (see redacted schemas). No combined lifecycle trace is claimed.
- Synchronous vs asynchronous command-hook behavior: 
  - **Synchronous hooks** (default): block Claude until they finish. `SessionStart` fires and completes before CLI proceeds.
  - **Asynchronous hooks** (`async: true`): behavior is **indeterminate**. The async probe (`test-async-behavior.sh`) recorded `StopFailure` and `SessionEnd` and did not record `SessionStart`; no hook ordering relative to CLI exit was measured (the async probe has no common CLI boundary markers). Successful-session async viability is **indeterminate and unavailable to T02 pending T05**.

### Evidence
1. **Observed Claude version metadata**: version `2.1.259` was recorded by the isolated environment during the version check in `test-synchronous-session-start.sh` (2026-09-03T09:17 UTC); this is normalized metadata, not retained raw CLI output.
2. **Hook configuration schema**: from official documentation (fetched via webfetch). Hook handlers accept `type`, `command`, `async`, etc.
3. **Observed event firing with SYNCHRONOUS hooks** (from `test-synchronous-session-start.sh`):
   - `SessionStart` fired with fields: `session_id` (present), `cwd` (present), `source` (present), `transcript_path` (present).
   - `StopFailure` and `SessionEnd` were NOT observed in this synchronous probe (see async trace, item 4 below).
4. **Observed event firing with ASYNCHRONOUS hooks** (from `test-async-behavior.sh`):
   - `StopFailure` and `SessionEnd` were recorded in the async probe log.
   - `SessionStart` was not recorded in the async probe.
   - No ordering of hook events relative to CLI exit was measured; no cross-trace ordering is claimed.
5. **Redacted schemas**: see `redacted-schemas.md` for field presence (keys only) of observed events.

### Open questions
- Does `SessionStart` fire for interactive authenticated sessions? Could not test due to lack of authentication.
- Do asynchronous hooks execute for successful sessions? Cannot test without authentication. Status: **indeterminate**; resolution is deferred to T05 and the result is unavailable to T02 until then.

## S2: Real Turn Lifecycle

### Decision
- Ordered traces for a successful real turn, tool activity, permission wait, and user exit **cannot be observed** without authentication. The contract permits a blocked evidence report in this case.
- **Separate trace observations** (no cross-trace ordering is claimed; each trace stands alone):
  - **Trace A (synchronous hooks, unauthenticated)**: `SessionStart` fires with fields: `session_id`, `cwd`, `source`, `transcript_path` present. `StopFailure` and `SessionEnd` are NOT observed in this probe.
  - **Trace B (asynchronous hooks, unauthenticated)**: `StopFailure` and `SessionEnd` were recorded; `SessionStart` was not recorded in this probe. No ordering relative to CLI exit was measured.
- Negative assertion: sensitive fields (`transcript_path`, `last_assistant_message`, `error`) are present in raw hook payloads but must be filtered out before crossing the hook boundary. Our observer scripts record only field presence, never values.

### Evidence
1. **Trace A (synchronous hooks, unauthenticated)** from `test-synchronous-session-start.sh`:
   - `SessionStart` fired with fields: `session_id` (present), `cwd` (present), `source` (present), `transcript_path` (present).
   - `StopFailure` and `SessionEnd` NOT observed in this probe.
2. **Trace B (asynchronous hooks, unauthenticated)** from `test-async-behavior.sh`:
   - `StopFailure` and `SessionEnd` were recorded in the probe log.
   - `SessionStart` was not recorded in this probe.
   - No inter-event timing or CLI-relative ordering is claimed for the async probe.
3. **Field presence verification**: our observer scripts log only whether fields exist (present/absent), never their values.
4. **Raw payloads are not stored**: all evidence scripts use metadata-only logging.

### Limitations
- No authenticated turn to observe `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PermissionRequest`, `Notification`, `SubagentStart`, `SubagentStop`, `Stop` (non-failure), `CwdChanged`, etc.
- The ordering of events in a successful turn cannot be verified.

## S3: Foreground/Background/Startup-Gap Behavior

### Decision
- `claude agents --json` returns an empty JSON array when no sessions are active, and does not require authentication. It lists background sessions (including those started with `--bg`) but not foreground interactive sessions (based on documentation). The command works without authentication.
- Startup gap: cannot test whether a dashboard started after `SessionStart` can discover live sessions because no sessions exist without authentication. The hook-based approach only receives events that fire after the dashboard is running; there is no replay of missed events.
- Foreground vs background: foreground interactive sessions are not listed by `claude agents --json`; background sessions are listed. This matches the documentation that `claude agents --json` reports active background sessions, not every interactive foreground session.

### Evidence
1. **No sessions discovered**: `claude agents --json` discovered no sessions when run with isolated config and no authentication. This confirms the command works without retaining its raw serialization.
2. **Hook-based discovery**: `SessionStart` fires with synchronous hooks; `StopFailure` and `SessionEnd` fire with asynchronous hooks. No foreground or background sessions are observable without authentication.
3. **Documentation**: official CLI reference states `claude agents --json` prints active sessions as a JSON array, and `--json --all` also includes completed background sessions.

### Limitations
- No evidence for startup-gap behavior with live sessions.
- No evidence for foreground session discovery via hooks (since no foreground sessions exist without auth).
- The reliability of `claude agents --json` as a session source for foreground sessions is unknown.

## S4: Session-End Reliability and Staleness

### Decision
- `SessionEnd` fires after `StopFailure` (authentication failure). The reason field is `"other"`. This indicates that `SessionEnd` is delivered even when the session fails to start due to missing authentication.
- Under ordinary exit, interrupt, terminal close, crash, and machine sleep, `SessionEnd` delivery cannot be tested without an authenticated session. The documentation states `SessionEnd` fires when a session terminates, but reliability across abrupt termination is unknown.
- **Bounded stale-session recommendation (PROVISIONAL)**: Given the inability to test exit paths, the following conservative policy is recommended for initial implementation:
  - **Staleness timeout**: 5 minutes (300 seconds) after the last received event for a session.
  - **Rationale**: Claude sessions typically involve user interaction; a 5-minute gap suggests the user has stepped away or the session is inactive.
  - **Fallback behavior**: When a session exceeds the staleness timeout, mark it as "stale" in the dashboard (dimmed appearance) rather than removing it immediately.
  - **Unblock condition for final policy**: T05 must measure actual `SessionEnd` reliability across exit paths and determine appropriate timeout based on real user workflow patterns.

### Evidence
1. **SessionEnd after StopFailure**: observed in `test-async-behavior.sh` logs (Trace B). The `reason` field is `"other"`. The `reason` enum includes `"clear"`, `"resume"`, `"logout"`, `"prompt_input_exit"`, `"other"`.
2. **Trace B record**: `StopFailure` and `SessionEnd` were both recorded in the async probe log. No combined sequence with `SessionStart` is claimed, and no inter-event timing value is asserted.
3. **No other exit paths observed**: cannot test interrupt, terminal close, crash, or sleep without an active session.

### Limitations
- No evidence for other exit paths (interrupt, terminal close, crash, sleep).
- No evidence for stale sessions (no active sessions to become stale).
- The `reason` field for other exit paths is unknown.
- The 5-minute staleness timeout is provisional and must be validated with real user sessions.

## S5: Identity, CWD, and Subagents

### Decision
- `session_id` is a UUID (observed `affdc5fb-522b-40a3-bb33-18f0ebaedc3b`). It is unique per session and is used as the primary identifier for session tracking.
- `cwd` is the absolute path where Claude was invoked. It matches the current working directory and is stable across events within the same session.
- Subagent identity: not observed (no subagents spawned without authentication). The hooks reference indicates `SubagentStart` and `SubagentStop` events include `agent_id` and `agent_type` fields, but these could not be verified.
- Parent session identity: not applicable (no subagents). The existing `parent_id` field in `ClaudeTrackedSession` can be used if subagent parent identity is provided in hooks.
- Project identity: derived from `cwd` using the existing canonical project identity resolver. The `transcript_path` also reveals project structure but is sensitive and must be filtered.

### Evidence
1. **session_id and cwd** present in `SessionStart`, `StopFailure`, and `SessionEnd` payloads. The values are consistent across events for the same session.
2. **transcript_path** field presence observed in payloads. The field is sensitive and must not cross the hook boundary. No path structure or template is recorded.
3. **No subagent events observed**: cannot verify `agent_id`, `agent_type`, or parent-child relationships.

### Limitations
- No evidence for `agent_id`, `agent_type`, subagent hooks, `CwdChanged`, or worktree/symlink behavior.
- Cannot verify if parent identity is representable in hooks.
- The structure of `transcript_path` reveals internal project organization; must be redacted.

## Authenticated Scenario Status

**BLOCKED** - An authenticated real Claude CLI scenario cannot run in this environment because:
- `ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN`, and `ANTHROPIC_AUTH_TOKEN` are all unset.
- Interactive `claude auth login` / `claude setup-token` require a human and cannot be automated.
- The real `~/.claude` is OFF-LIMITS and cannot be used.

**Unblock condition**: a credentialed rerun or T05 must provide authentication (API key or OAuth token) and run an interactive session that triggers a real model turn, tool activity, and permission request.

## Known Gaps and Limitations

1. **No authenticated sessions**: all evidence is from unauthenticated failures. Lifecycle ordering for successful turns (including tool activity, permission requests, notifications) is unknown.
2. **SessionStart observed only with synchronous hooks**: the async probe (`test-async-behavior.sh`) did not record SessionStart.
3. **Hook asynchronous behavior indeterminate**: no CLI-relative hook timing was measured; successful-session async viability is unavailable to T02 pending T05.
4. **Startup gap**: cannot test whether a dashboard started after `SessionStart` can discover live sessions; no live sessions exist.
5. **Foreground session discovery**: `claude agents --json` does not list foreground interactive sessions; hooks may be the only source for foreground sessions, but cannot verify without authentication.
6. **Staleness policy**: provisional 5-minute timeout recommended; must be validated with real user sessions in T05.
7. **Subagent identity**: not observed; cannot verify parent-child relationship representation.
8. **Exit-path behavior**: `SessionEnd` reason for interrupt, terminal close, crash, sleep unknown.
9. **CwdChanged behavior**: not observed; cannot verify if cwd changes mid-session and how it affects project identity.

## Honest Uncertainties for Reviewer to Probe

1. Does `SessionStart` fire for interactive authenticated sessions? We observed it with unauthenticated `--print` using synchronous hooks.
2. Which events fire when authentication succeeds but the model returns an error? We observed `StopFailure` with an `error` field present. Other error types may produce different event sets.
3. Is `SessionEnd` guaranteed to fire after every `StopFailure`? Our observation shows them in sequence; is there a race condition?
4. The `reason` field in `SessionEnd` is `"other"` for authentication failure. What are the other possible values? (Documentation lists `"clear"`, `"resume"`, `"logout"`, `"prompt_input_exit"`, `"other"`.)
5. The `effort` field in `StopFailure` is an object. Is this always present? What does it represent?
6. What is the actual behavior of asynchronous hooks for successful sessions? Cannot test without authentication.

## Artifact Inventory

- `hook-observer.sh`: metadata-only observer script (records field presence only, never values).
- `test-synchronous-session-start.sh`: test script with synchronous hooks and disposable cwd (captures exit status).
- `test-async-behavior.sh`: test script with asynchronous hooks and disposable cwd (captures exit status).
- `test-async-observation.sh`: test script with async hooks and per-event logs (captures exit status).
- `test-event-observation.sh`: test script with synchronous hooks and per-event logs (captures exit status).
- `test-comprehensive.sh`: test script with configurable sync/async hooks and per-event metadata logs (captures exit status).
- `redacted-schemas.md`: redacted field presence for observed events, with clear separation of observed vs documentation-only fields.
