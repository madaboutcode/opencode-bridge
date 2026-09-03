# opencode2 SSE event catalog — live-captured, 2026-09-01

Source: three live captures against the running paired server (`127.0.0.1:49374`,
auth from `~/.config/opencode/service.json` + `opencode2 pair`), taken while
working the dashboard requirements doc. Not documentation review — every shape
below came from `curl -u opencode:<pw> http://127.0.0.1:49374/api/event`,
scoped to a real session ID and read back from the saved output. Files:
`/tmp/oc_event_sample.txt` (45s, ambient traffic from an unrelated live
session), `/tmp/oc_ask_spike.txt` (a session told to ask a clarifying
question), `/tmp/oc_full_tool_spike.txt` (a session told to exercise every
tool it has, in a scratch dir `/tmp/oc_tool_spike`).

Gotcha worth flagging for the next person: a naive `grep -o '"type":"[a-z.]*"'`
over raw SSE text overcounts. Content parts nested inside a reasoning event's
`state.reasoningDetails[]` carry their own `type` key (`"reasoning.text"`),
and tool-result content parts carry `"type":"text"` — neither is a real
top-level SSE event type. Only trust `type` after parsing each `data: {...}`
line as JSON and reading the outer object's `type` key.

## 1. Top-level event types, confirmed live (outer `type` key only)

| type | seen | notes |
|---|---|---|
| `server.connected` | yes | fires once per SSE connection |
| `session.created` | yes | |
| `session.reasoning.started/delta/ended` | yes | bursts between tool calls and before final text |
| `session.text.started/delta/ended` | yes | final assistant text streaming |
| `session.step.started/ended` | yes | one step per tool-call-or-text segment inside a turn; `step.ended` carries `finish`/`rawFinish` + per-step `cost`/`tokens` |
| `session.tool.input.started/ended` | yes | see §2 — `input.started` is the ONLY event with the tool `name` |
| `session.tool.called` | yes | full `input` args, no `name` |
| `session.tool.progress` | yes | fires mid-flight for long tool calls (shell, subagent); see §2 |
| `session.tool.success` | yes | result `content` + tool-specific `metadata` |
| `session.usage.updated` | yes | running cost/token totals |
| `shell.created` | yes | fires when a `shell` tool call spawns a real subprocess; see §2 |
| `session.execution.started/succeeded/failed/interrupted` | **documented only, never observed live** in any of these 3 captures (~180 events total) across 4 different sessions. The bridge's own 2026-08-27 spike observed these directly in its own captures, so they're real — just didn't fire in this window's traffic. Don't assume they're rare; assume this sample didn't catch one. |
| `shell.exited`, `shell.deleted` | documented (2026-08-27 spike), not re-observed here | |

## 2. Tool-call event shape — the name/args split

`session.tool.called` does **not** carry the tool name. Only
`session.tool.input.started` does, keyed by the same `data.id` (call ID) that
`input.ended`/`tool.called`/`tool.success` all share. To build a "what is this
session doing" line you must join `input.started` (name) with `tool.called`
(args) by `id`, scoped per `sessionID`:

```
tool.input.started  {id, name}                    ← name lives here only
tool.input.ended    {id, text: "<raw json args>"}  ← args as a JSON string
tool.called         {id, input: {...}, executed}   ← args as a real object
tool.success        {id, content: [...], metadata: {...}}
tool.progress       {id, metadata: {...}}          ← optional, mid-flight only
```

`shell.created` fires as a side effect of a `shell` tool call — separate
event, own `id` (`sh_...`), linked back via `data.info.metadata.sessionID`.
`tool.progress` for a shell call carries `metadata.shellID` matching that
`sh_...` id; for a `subagent` call it carries `metadata.sessionID` (the
child's session) + `metadata.status`.

## 3. Per-tool confirmed shapes

9 tool names confirmed to exist (from message-history scan across 15 live
sessions): `shell, edit, grep, write, skill, subagent, glob, patch, read`.
7 of them exercised live in one deliberate test session
(`ses_fa2aaa04effeOueRYokRM4gWtB`, scratch dir `/tmp/oc_tool_spike`) — real
`input` and `tool.success.content`/`metadata` below. `glob` and `patch` were
offered to the model but it didn't use them this run — still unconfirmed.

### `grep`
```json
"input": {"pattern": "line", "path": "/tmp/oc_tool_spike"}
"success.content": [{"type":"text","text":"Found 1 matches\n/tmp/oc_tool_spike/seed.txt:\n  Line 1: line one\n"}]
"success.metadata": {"matches": 1, "truncated": false}
```

### `write`
```json
"input": {"path": "/tmp/oc_tool_spike/scratch.txt", "content": "alpha bravo charlie\ndelta echo foxtrot\ngolf hotel india\n"}
"success.content": [{"type":"text","text":"Created file successfully: scratch.txt"}]
"success.metadata": {"truncated": false}
```

### `edit`
```json
"input": {"path": "/tmp/oc_tool_spike/scratch.txt", "oldString": "delta echo foxtrot", "newString": "delta echo FOXTROT"}
"success.content": [{"type":"text","text":"Edited scratch.txt (1 replacement)"}]
"success.metadata": {"files": [{"file": "...", "patch": "<unified diff>", "status": "modified", "additions": 1, "deletions": 1}], "truncated": false}
```
(Same `files[].patch` shape confirmed separately in the 2026-08-27 wire spike
on a real edit in another project.)

### `shell`
```json
"input": {"command": "wc -l scratch.txt", "workdir": "/tmp/oc_tool_spike"}
"success.content": [{"type":"text","text":"       3 scratch.txt\n"}, {"type":"text","text":"Command exited with code 0."}]
"success.metadata": {"status": "...", "exit": 0, "truncated": false}
```
Also seen with `timeout` in `input` on another session (`{"command","timeout","workdir"}`).

### `subagent`
```json
"input": {"agent": "clerk", "description": "Ping pong reply", "prompt": "Reply with the single word: pong"}
"success.content": [{"type":"text","text":"<subagent sessionID=\"ses_...\" state=\"completed\">\npong\n</subagent>"}]
"success.metadata": {"sessionID": "ses_...", "status": "...", "truncated": false}
```
The delegated child runs as a fully separate session (own `sessionID`,
confirmed elsewhere never merged into the parent's message list). The
parent's tool-call result embeds the child's answer wrapped in a
`<subagent sessionID=... state=...>` tag — this is how a dashboard would
render "delegated to subagent, got back: pong" without following the child
session itself.

### `skill`
```json
"input": {"id": "opencode-config"}
"success.content": [{"type":"text","text":"<skill_content name=\"opencode-config\">\n# Skill: opencode-config\n..."}]
"success.metadata": {"name": "opencode-config", "directory": "...", "truncated": false}
```

### `read`
```json
"input": {"path": "/tmp/oc_tool_spike/scratch.txt"}
"success.content": [{"type":"text","text":"Read file /tmp/oc_tool_spike/scratch.txt, lines 1-3\n1: alpha bravo charlie\n2: delta echo FOXTROT\n3: golf hotel india"}]
"success.metadata": {"truncated": false}
```

### `glob`, `patch` — still unconfirmed
Offered to the model in the same test run; it chose not to use them
(picked `grep` over `glob` for a text search, and had no multi-hunk diff
task that would justify `patch` over `edit`). Needs a more targeted prompt
next time (e.g. "find all *.txt files" to force `glob`; "apply this 3-hunk
diff" to force `patch`).

## 4. Session-detail fields — additions to the 2026-09-01 session-shape spike

Confirmed on two bridge-launched test sessions (`GET /api/session/{id}`):

- **`subpath`** — not in the earlier session-shape inventory. Seen as
  `"subpath": "tmp/oc_tool_spike"` alongside `location.directory:
  "/tmp/oc_tool_spike"`. Looks like a project-relative path, worth checking
  whether it's populated for sessions inside a git repo too (both test
  sessions were in `/tmp`, not a git working tree).
- **`projectID: "global"`** — a literal string, not the 40-char hex hash
  documented in the session-shape spike, when the session's directory isn't
  inside a recognized project (both `/tmp` sessions got this). The hex-hash
  form is presumably per-project; `"global"` is the fallback.
- **`title` for bridge-launched sessions is NOT natural language.** Both
  test sessions here were started via this repo's own MCP bridge
  (`opencode_task` with an explicit `title` param) and got back exactly the
  literal string passed in, formatted as `"cc-bridge:17060:<slugified-title>"`
  — e.g. `"cc-bridge:17060:spike-exercise-all-tools-for-sse-event-c"`. The
  session-shape spike's example (`"Reviewing opencode..."`) was model-generated,
  from a session NOT started with an explicit title. A dashboard reading
  `title` verbatim will show these bridge-formatted strings for
  bridge-launched sessions and natural-language summaries for everything
  else — worth deciding whether to detect and reformat the `cc-bridge:` prefix.

## 5. No wire signal for "agent is asking a question and stuck"

Directly tested: dispatched a session with the prompt "stop and ask me which
file to delete, then wait for my answer." It responded with plain
`session.text.delta` text — *"Which file would you like me to delete?..."*
— and the turn ended normally:

```json
{"outcome": "succeeded", "time": {"idle": 1788272019805}}
```

Wire-identical to a session that finished a task with a plain summary.
opencode2 has no permission-gate or ask-question protocol event — a
clarifying question is just ordinary text that happens to end the turn. The
only way to tell "waiting because done" from "waiting because it needs a
reply" is to read the final assistant text and apply a heuristic — see the
dashboard requirements doc, R6.7.

## 6. `session.step.*` and `session.usage.updated` shapes

```json
// step.started
{"sessionID","agent":"build","model":{"id":"hy4-preview","providerID":"opencode-go"},"assistantMessageID"}
// step.ended
{"sessionID","assistantMessageID","finish":"tool-calls","rawFinish":"tool_calls","cost":0.000746821,"tokens":{"input":194,"output":24,"reasoning":53,"cache":{"read":9344,"write":0}}}
// usage.updated (session-level running total, not per-step)
{"sessionID","cost":0.00849556,"tokens":{"input":9316,"output":75,"reasoning":53,"cache":{"read":9664,"write":0}}}
```

`step.ended.finish` is the per-step outcome tag; `"tool-calls"` means the
step was a tool call (not final text) — matches the 2026-08-27 spike's note
that a text-less step is the root cause of the empty-response bug.
