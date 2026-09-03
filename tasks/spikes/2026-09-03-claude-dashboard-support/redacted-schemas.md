# Redacted Schemas for Claude Hook Events

These schemas record only field presence (keys) and their sensitivity classification. No values are stored. Each field is marked as:
- **observed**: field presence verified in actual hook payloads
- **documentation-only**: field documented in official hooks reference but not observed in our tests
- **unknown**: field may exist but not verified

## SessionStart (observed with synchronous hooks)

**Source**: `test-synchronous-session-start.sh` (2026-09-03T09:17 UTC).

| Field | Type | Sensitive | Status | Notes |
|-------|------|-----------|--------|-------|
| `session_id` | UUID string | No | observed | Unique session identifier |
| `transcript_path` | string | **Yes** | observed | Absolute path to transcript file; reveals project structure and session UUID |
| `cwd` | string | No | observed | Current working directory |
| `hook_event_name` | string | No | observed | Literal `"SessionStart"` |
| `source` | string | No | observed | Enum: `"startup"`, `"resume"`, `"clear"`, `"compact"`, `"fork"` |
| `model` | string | No | documentation-only | Optional; not observed in unauthenticated sessions |
| `agent_type` | string | No | documentation-only | Optional; not observed |
| `session_title` | string | No | documentation-only | Optional; not observed |
| `seconds_since_last_response` | number | No | documentation-only | Optional, only for resume/fork |
| `context_tokens` | number | No | documentation-only | Optional |
| `prompt_cache_likely_expired` | boolean | No | documentation-only | Optional |
| `estimated_cache_write_usd` | number | No | documentation-only | Optional |

**Redacted schema** (what may cross the hook boundary):
```json
{
  "session_id": "<uuid>",
  "cwd": "<path>",
  "hook_event_name": "SessionStart",
  "source": "<enum>"
}
```

## StopFailure (observed)

**Source**: async probe (`test-async-behavior.sh`, Trace B); field-presence metadata aggregated from isolated observations.

| Field | Type | Sensitive | Status | Notes |
|-------|------|-----------|--------|-------|
| `session_id` | UUID string | No | observed | Unique session identifier |
| `transcript_path` | string | **Yes** | observed | Absolute path to transcript file |
| `cwd` | string | No | observed | Current working directory |
| `prompt_id` | UUID string | No | unknown | Not verified in recent tests |
| `effort` | object | No | unknown | Not verified in recent tests |
| `hook_event_name` | string | No | observed | Literal `"StopFailure"` |
| `error` | string | **Yes** | observed | Error type (sensitive, enum-agnostic) |
| `last_assistant_message` | string | **Yes** | unknown | Not observed in recent tests (filtered by observer) |

**Redacted schema** (what may cross the hook boundary):
```json
{
  "session_id": "<uuid>",
  "cwd": "<path>",
  "hook_event_name": "StopFailure",
  "error_type": "<enum>"
}
```

## SessionEnd (observed)

**Source**: async probe (`test-async-behavior.sh`, Trace B); field-presence metadata aggregated from isolated observations.

| Field | Type | Sensitive | Status | Notes |
|-------|------|-----------|--------|-------|
| `session_id` | UUID string | No | observed | Same as above |
| `transcript_path` | string | **Yes** | observed | Same as above |
| `cwd` | string | No | observed | Same as above |
| `prompt_id` | UUID string | No | unknown | Not verified in recent tests |
| `hook_event_name` | string | No | observed | Literal `"SessionEnd"` |
| `reason` | string | No | observed | Enum: `"clear"`, `"resume"`, `"logout"`, `"prompt_input_exit"`, `"other"` |

**Redacted schema** (what may cross the hook boundary):
```json
{
  "session_id": "<uuid>",
  "cwd": "<path>",
  "hook_event_name": "SessionEnd",
  "reason": "<enum>"
}
```

## General Notes

- All timestamps are recorded as Unix epoch seconds in logs but not part of the hook payload.
- The `transcript_path` field is sensitive because it reveals the internal file system structure and session UUID. It must be filtered out before crossing the hook boundary.
- The `last_assistant_message` field is sensitive and must be filtered out.
- The `error` field may contain sensitive information (e.g., authentication details) and should be treated as sensitive.
- The `cwd` field is not sensitive as it is the user's working directory (already known to the user).

## Evidence of Filtering

Our observer scripts record only field presence (present/absent), never values. They do not store `transcript_path`, `last_assistant_message`, `error`, or other sensitive field values. The final production hook helper must filter out all sensitive fields before forwarding to the Unix socket.