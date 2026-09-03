# Spike: OpenCode2 Session HTTP JSON Shape

**Date:** 2026-09-01  
**Purpose:** Inventory the live HTTP API shape of sessions to inform "project" design decisions.

---

## Authentication

- **Method:** `opencode2 pair` via `~/.opencode/bin/opencode2`
- **Base URL:** `http://127.0.0.1:49374` (ephemeral port per pair invocation)
- **Auth:** HTTP Basic (`opencode:<redacted>`)

---

## List Endpoint: `GET /api/session`

### Pagination Shape

```json
{
  "data": [ ... ],
  "cursor": {
    "previous": "<base64 anchor>",
    "next": "<base64 anchor>"
  }
}
```

- Bare `cursor` object with `previous`/`next` (base64-encoded anchors).
- **No** `limit`, `total`, `offset`, or `hasMore` fields.
- First page returned 50 items.

### Array Length (first page): 50

---

## Field Inventory: LIST vs DETAIL

| Field | LIST? | DETAIL? | Example (redacted) | Notes |
|---|---|---|---|---|
| `id` | ✅ | ✅ | `"ses_fa314b..."` | Always present |
| `parentID` | ✅ | ✅ | `"ses_fa31ad..."` | **ABSENT** on root sessions (not `"null"`, key is missing entirely) |
| `projectID` | ✅ | ✅ | `"4f05975d56..."` | SHA1-like hex string; always present |
| `agent` | ✅ | ✅ | `"deepseek-v4-flash"` | Agent name |
| `model` | ✅ | ✅ | `{"id":"...","providerID":"crof","variant":"default"}` | Nested object |
| `cost` | ✅ | ✅ | `0.00063` | Float, 0.0 for freshly created |
| `tokens` | ✅ | ✅ | `{"input":7602,"output":145,"reasoning":0,"cache":{"read":2570,"write":0}}` | Nested object |
| `outcome` | ❌ | ✅ | `"interrupted"` | **DETAIL ONLY.** Key missing on list items that don't have it. Values seen: `"interrupted"`, `"succeeded"` |
| `time` | ✅ | ✅ | see below | Nested object |
| `time.created` | ✅ | ✅ | `1788265318441` | Epoch ms |
| `time.updated` | ✅ | ✅ | `1788265318445` | Epoch ms |
| `time.idle` | ❌ | ✅ | `1788265185945` | **DETAIL ONLY** (absent on child sessions too) |
| `time.viewed` | ❌ | ✅ | `1788265185945` | **DETAIL ONLY** |
| `title` | ✅ | ✅ | `"Reviewing opencode..."` | Human-readable summary |
| `location` | ✅ | ✅ | `{"directory":"/Users/ajeesh/..."}` | Nested object |
| `location.directory` | ✅ | ✅ | `"/Users/ajeesh/projects/madaboutcode/opencode-mcp"` | **Present on every item observed** |

---

## Explicit YES / NO / ABSENT

| Field | LIST | DETAIL | Evidence |
|---|---|---|---|
| **`directory`** (top-level) | ❌ NO | ❌ NO | Never a top-level key. Always nested under `location.directory`. |
| **`location`** | ✅ YES | ✅ YES | Present on every session, both list and detail. |
| **`location.directory`** | ✅ YES | ✅ YES | Full filesystem path. Present on every item observed (all 50 list items, both sampled details). |
| **`project`** | ❌ NO | ❌ NO | No field named `project` anywhere. |
| **`projectID`** | ✅ YES | ✅ YES | Always present. SHA1-like hex string (40 chars). |
| **`parentID`** | ✅ YES (conditional) | ✅ YES (conditional) | Key is **ABSENT** (not null) on root sessions. Present with a session ID string on child sessions. |
| **`title`** | ✅ YES | ✅ YES | Always present. |
| **`time.updated`** | ✅ YES | ✅ YES | Epoch milliseconds. |
| **`time.idle`** | ❌ NO | ✅ YES | Detail only. Absent on freshly created child sessions even in detail. |
| **`outcome`** | ❌ NO | ✅ YES | Detail only. Values: `"interrupted"`, `"succeeded"`. |
| **`location.directory`** | ✅ YES | ✅ YES | See above. |

---

## Message Endpoint: `GET /api/session/{id}/message`

Wrapper: `{ "data": [...], "cursor": { "previous", "next" } }`  
Observed 12 messages in sampled parent session.

### Message `type` values (message-level)

- `"user"`
- `"assistant"`

### Content part `type` values (inside `content[]` on assistant messages)

- `"text"` — plain text output
- `"reasoning"` — reasoning/thinking trace
- `"tool"` — tool call (has `state` field with tool call state)

### User message top-level keys

`id`, `type`, `text`, `time`, `agents`, `files`, `skills`

### Assistant message top-level keys

`id`, `type`, `agent`, `model`, `content`, `time`, `cost`, `tokens`, `finish`, `providerState` (conditionally present)

---

## Key Observations

1. **`parentID` is the only way to distinguish root vs child sessions on the list endpoint.** It is absent (not null) for roots, present for children. Root sessions in this sample have no `parentID` key at all.

2. **`projectID` is always present** and appears to be a content-addressed hash of the project directory (40-char hex, likely SHA1). Multiple sessions share the same `projectID` when they share a working directory. This is the closest thing to a "project" concept on the wire.

3. **`location.directory` is always present** on both list and detail — never absent in the 50-item sample. It is always nested under `location`, never top-level.

4. **`outcome` is detail-only** — you cannot determine session success/failure from the list endpoint alone.

5. **`time.idle` is detail-only** and absent even on some detail responses (freshly created child sessions).

6. **Pagination is cursor-based** with base64-encoded anchors, not offset-based.
