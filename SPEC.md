# opencode-bridge — implementation spec

A single-binary **MCP stdio server** that any MCP client (Claude Code in
particular) launches. It exposes four `Agent`/`SendMessage`-style tools that
drive **opencode2** through its HTTP API and SSE event stream — no
`opencode2 run` subprocesses. When an async task goes idle, it pushes the
task's last output back into the launching client session over its inbox
socket, so the caller is notified without polling.

Three interfaces:

1. **Northbound** — MCP stdio (JSON-RPC 2.0, newline-delimited) to the MCP client.
2. **Southbound** — opencode2 HTTP + SSE at the paired URL (HTTP Basic auth).
3. **Sideband** — MCP client's inbox socket (AF_UNIX) for completion callbacks.

---

## 1. opencode2 HTTP API facts

These were observed live against `opencode2` and are not re-derived from
speculation. If the opencode API changes, update this section first.

**Discovery.** `opencode2 pair` prints:

```
  URLs      http://127.0.0.1:<port>
  Username  opencode
  Password  <password>
```

Parse those three lines. Basic-auth every HTTP/SSE request with
`Username:Password`. The port changes per service instance — always read it
fresh from `pair` at startup; never cache across restarts. `OPENCODE2_BIN`
overrides the binary path (default `~/.opencode/bin/opencode2`).

**REST endpoints used (all under `/api`, HTTP Basic auth, JSON):**

- `POST /api/session` body `{"model":{...},"agent":"<name>","title":"<t>"}`
  (all optional) → `{"data":{"id":"ses_..."}}`. Omit `model` to use the
  server default.
- `POST /api/session/{id}/prompt` body
  `{"text":"...","delivery":"steer"|"queue"}` → returns the queued user
  message immediately: `{"data":{"id":"msg_...","delivery":"steer"}}`. This
  is BOTH "start a task" (fresh session) and "send a followup" (existing
  session). `delivery` `"steer"` interrupts the current turn and injects;
  `"queue"` waits for the current turn to finish. The API default is
  `"steer"`.
- `POST /api/session/{id}/prompt` body `{"text":...,"metadata":{"origin":"cc-bridge:<o>","bridge":true},...}`
  → stamps the durable provenance tag described in §8.
- `GET /api/event` → SSE (`text/event-stream`). ONE global stream for ALL
  sessions. Each frame is `data: {json}` where the JSON is
  `{"type":"...","data":{"sessionID":"ses_...",...},...}`.
- `POST /api/session/{id}/wait` → `204 No Content` when the session goes
  idle (blocks).
- `POST /api/session/{id}/interrupt` → cancel the running turn.
- `GET /api/session/{id}` → `{"data":{"id","outcome":"succeeded"|"failed"|"interrupted",
  "time":{"created","updated","idle",...},"cost","tokens",...}}`. `outcome`
  is present once a turn has completed; `time.idle` is the idle timestamp.
- `GET /api/session/{id}/message` → `{"data":[{...user...},{...assistant...}]}`.
  The assistant message has `content` = a list of parts:
  `{"type":"reasoning","text":...}` and `{"type":"text","text":...}`.
  **Final output** = the concat of `text` from parts where `type=="text"`
  (ignore reasoning parts). Also has `finish`, `cost`, `tokens`.
- `GET /api/model` → `{"data":[{"id","modelID","providerID","name","cost":[...],...}]}`.
- `GET /api/agent` → agent catalog (name, description, mode, hidden, model).
- `GET /api/session` → list sessions. `GET /api/health` → liveness.

**SSE event lifecycle for one turn (observed end-to-end):**

```
session.created                     (only on create)
session.inbox.enqueued              data.inboxID, data.sessionID
session.execution.started           data.sessionID           <- turn begins
session.instructions.updated
session.inbox.delivered
session.reasoning.started/delta/ended
session.text.started/delta/ended    text.ended: data={sessionID, assistantMessageID, ordinal, text}
session.step.ended                  data.finish:"stop", cost, tokens
session.usage.updated               data.cost, data.tokens
session.execution.succeeded         <- TURN IDLE (success)
```

Terminal turn events (observed): `session.execution.succeeded`,
`session.execution.failed`, `session.execution.interrupted`. **The terminal
`session.execution.*` event for a tracked session = "went idle" = fire the
callback.** `session.text.ended.text` carries the finished assistant text
for that turn; capture the last one seen, or re-fetch via `GET /message`
after the terminal event (message list is authoritative — prefer it for
the final result).

Error shape: a failed turn emits `session.execution.failed` with
`data.error={type,message,status}` (e.g.
`{"type":"provider.invalid-request","message":"Insufficient Balance","status":402}`).

The message list is **newest-first** by `time.created`. The assistant message
discriminant is `type:"assistant"` (NOT `role`). Pick the assistant message
by `max(time.created)`, don't trust array order.

---

## 2. MCP client callback protocol

The MCP client (e.g. Claude Code) exports these env vars to the MCP servers
it spawns:

- `CLAUDE_CODE_MESSAGING_SOCKET` — AF_UNIX path. Empty ⇒ callbacks disabled
  (the bridge degrades gracefully: tasks still complete, you just don't get
  pinged).
- `CLAUDE_CODE_MESSAGING_TOKEN` — auth token (optional on macOS/Linux;
  required on Windows).

To post a message into the launching client session, connect the AF_UNIX
stream socket, send newline-delimited JSON frames, then close:

```
{"type":"auth","token":"<TOKEN>"}                                # only if token present
{"type":"user","message":{"role":"user","content":"<text>"}}
```

Send complete `\n`-terminated lines promptly. The server drops a connection
that sends no full line within a timeout. The message arrives in the client
session as a peer/"another session" message and triggers a turn when idle —
good enough for completion notifications.

**Surfacing nuance.** A callback is delivered to the client's socket the
instant the async task finishes, but it only **surfaces** (triggers a turn /
becomes visible) when the client session is **idle** between turns. During a
busy session (e.g. an agent mid-tool-calls), callbacks queue and can
appear delayed and out of phase with whatever the caller was doing. This is
inherent to the peer-message inbox channel, not a bug and not lost
delivery.

---

## 3. Rust design

**Crates (kept lean):** `tokio` (rt-multi-thread, macros, io-std, net,
process, sync, time), `reqwest` (json + streaming body; `stream` feature),
`serde` + `serde_json`, `futures-util` (for stream reading),
`eventsource-stream`. No MCP SDK — the MCP stdio surface we need is tiny
(initialize / tools/list / tools/call) and hand-rolling it avoids depending
on an SDK's framing choices. No error-handling crate (a single
`Box<dyn Error+Send+Sync>` alias is enough). No `tracing`.

**Module layout:**

- `main.rs` — startup: discover server (`pair`), read client env, build
  shared state, spawn the SSE consumer task + periodic backstop sweep, run
  the MCP stdio loop.
- `opencode.rs` — HTTP client wrapper (reqwest + base URL + basic auth).
  One method per endpoint used: `create_session`, `prompt`, `wait`,
  `interrupt`, `get_session`, `list_messages`, `list_sessions`,
  `list_models`, `list_agents`, `health`. Owns all opencode wire-format
  knowledge — callers don't parse opencode responses.
- `sse.rs` — the global SSE consumer. Connect `GET /api/event`, demux by
  `sessionID`, on a terminal `session.execution.*` for a tracked session
  fetch the final text and call `notify::cc(...)`. Auto-reconnect with
  backoff and a read/idle timeout; on every (re)connect reconcile tracked
  in-flight sessions via `GET /api/session/{id}` (see §7, missed-event
  guard).
- `notify.rs` — the AF_UNIX callback to the MCP client (§2).
- `mcp.rs` — JSON-RPC stdio framing + dispatch (§4).
- `tools.rs` — the four MCP tool handlers and their JSON Schemas.
- `registry.rs` — `Mutex<HashMap<String, Tracked>>` where `Tracked{prompt,
  model, agent, notify, status, last_text, created, notified}`.
- `state.rs` — `Arc<AppState>` shared between the MCP loop, the tool
  handlers, and the SSE consumer.

**Concurrency:** tokio. The MCP loop owns stdin/stdout. The SSE consumer
is one spawned task; the periodic sweep is another. Shared `registry` is
behind a `Mutex` (never held across `.await`); the opencode client is
stateless and cheap to share. Each `tools/call` runs on its own spawned task
so a slow `wait=true` never blocks another concurrent `tools/call` (clients
do issue them concurrently).

**Discovery:** at startup run `opencode2 pair` (via
`tokio::process::Command`), parse URL/Username/Password. Fail fast with a
clear stderr message if the server isn't up (suggest
`opencode2 service start`). Verify with `GET /api/health` before serving.

**Statefulness — pure in-memory.** opencode's server is the source of truth
(`GET /api/session/{id}` survives a bridge restart), so the registry is
**in-memory only**. No disk job store, no on-disk notify-intent file.
Rationale: the bridge is a child of the MCP client session — if it dies,
either the client died too (socket + token are gone, notify-intent points
at a corpse) or the client restarted just the MCP server (rare; fallback is
the user calls `opencode_sessions`). A new client session re-discovers
work via `opencode_sessions` backed by opencode's own session list. The
notification gap across an MCP-only restart is known and accepted.

---

## 4. MCP stdio protocol (hand-rolled)

Transport: newline-delimited JSON-RPC 2.0 on stdin/stdout (one JSON object
per line). Read stdin line by line; write one-line responses to stdout;
**all logging goes to stderr** (stdout is the protocol channel — never
print anything else there).

Methods handled:

- `initialize` → reply
  `{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},
   "serverInfo":{"name":"opencode-bridge","version":"0.1.0"}}`.
- `notifications/initialized` → no reply (it's a notification).
- `tools/list` → reply `{"tools":[ ... each tool's {name, description,
  inputSchema} ... ]}`. `inputSchema` is a JSON Schema object per tool (§5).
- `tools/call` params `{name, arguments}` → dispatch to the tool, reply
  `{"content":[{"type":"text","text":"<json-encoded result>"}]}`. On error,
  reply with `{"content":[{"type":"text","text":"<err>"}],"isError":true}`
  (preserves the JSON-RPC `id`).

Requests carry an `id` (echo it). Notifications have no `id` (send nothing
back). Verify the exact `protocolVersion` the client sends in `initialize`
and echo a compatible one.

Concurrency-safe stdout: every tool reply is funneled through one writer
task/channel so concurrent replies never interleave bytes on stdout.

---

## 5. MCP tools

All results returned as JSON text. `session_id` is opencode's `ses_...`.

### `opencode_task`

The single dispatch entry point. `session_id` present ⇒ continue an
existing session (followup); absent ⇒ start a new one. Both branches share
the `wait`/`notify` async-vs-sync tail.

Arguments:

| name | type | default | applies | notes |
| --- | --- | --- | --- | --- |
| `prompt` | string | — | both | Required. The text to send. |
| `session_id` | string | — | continue | Continue this existing session. Omit to start a new one. |
| `wait` | bool | `false` | both | RECOMMENDED: leave `false` (async). Fire the task and get a callback when it finishes, so you don't sit blocked on it. `true` blocks up to 240s, then falls back to the async callback anyway if still running. |
| `notify` | bool | `true` | both | Push a completion message into this client session when the async turn finishes. |
| `model` | string | server default | new | `"providerID/modelID"`, e.g. `"opencode-go/ox-alpha-free"`. Ignored when continuing. |
| `agent` | string | server default | new | opencode agent name, e.g. `"coder"`. Ignored when continuing. |
| `directory` | string | bridge cwd | new | Absolute path the agent's tools (edit/read/bash) operate in — the project to touch. Defaults to the directory the bridge launched from (usually the current project). Ignored when continuing. |
| `title` | string | — | new | Human-readable session title. Ignored when continuing. |
| `delivery` | `"queue"\|"steer"` | `"queue"` | continue | `"queue"` (default) waits for any in-flight turn to finish; `"steer"` interrupts it. Ignored when starting. |

Returns:

- async (`wait=false`): `{session_id, status:"running"}`.
- sync (`wait=true`, finished within 240s):
  `{session_id, output, outcome}`.
- sync (`wait=true`, hit the 240s cap):
  `{session_id, status:"running", note:"still running after 240s; will notify on completion"}`.

### `opencode_sessions`

Inspect opencode2 sessions. `session_id` present ⇒ detail one session
(outcome, running, cost, tokens, and the final assistant output text of its
last turn) — works for any session id, including ones started elsewhere.
Omit `session_id` ⇒ list this client session's own sessions (as `sessions`,
newest first) — scoped to work launched or followed up on in this bridge
process, and still correct across an MCP restart (see §8 rediscovery).
`include_all=true` additionally dumps unrelated server sessions as
`other_sessions` (debug).

Arguments:

| name | type | default | notes |
| --- | --- | --- | --- |
| `session_id` | string | — | The session to detail. Omit to list this bridge's own sessions. |
| `include_all` | bool | `false` | List mode only (no `session_id`): also include unrelated sessions on the shared server — the TUI, other tools, other client sessions. Mainly to grab the id of a session started elsewhere so you can continue it. |

### `opencode_cancel`

Interrupt a running opencode2 session's current turn.

Arguments:

| name | type | notes |
| --- | --- | --- |
| `session_id` | string | Required. |

Returns `{session_id, cancelled:true}`.

### `opencode_catalog`

Look up what's available to run tasks with: the opencode2 server's models
or agents. `kind` selects which list. `query` filters (case-insensitive
substring; space-separated terms ANDed — all must match).

Arguments:

| name | type | default | notes |
| --- | --- | --- | --- |
| `kind` | `"models"\|"agents"` | — | Required. `"models"` = model catalog (search over `providerID/id/name`). `"agents"` = agent tags you pass as `opencode_task`'s `agent` (search over `name/description`). |
| `query` | string | — | Case-insensitive substring filter; space-separated terms are ANDed. Omit to list all. |
| `include_hidden` | bool | `false` | `kind=agents` only: include agents opencode marks hidden. |

Model results are capped at 200 (the response carries `matched`, `returned`,
and `truncated` so the caller knows). Agent rows carry `name`, `mode`,
`description`, `model` (`"providerID/id"` or null if inherited), `variant`
(the effort level — the only thing distinguishing e.g. `luna` from
`luna-high`), and `hidden`.

### Missed-event / reconnect guard

The SSE stream can drop and reconnect, and a terminal event could land in
the gap. Two layers keep this correct:

1. **Registration-before-prompt ordering (race kill).** Register the session
   in the registry BETWEEN `POST /session` (have the id) and `POST /prompt`
   (work starts). Zero window for a terminal event on an unregistered
   session, even for fast-failing tasks (bad model, auth error emitting
   `session.execution.failed` immediately).
2. **Reconnect sweep.** On every SSE (re)connect, for every tracked session
   still marked running, `GET /api/session/{id}`; if it already shows a
   terminal `outcome`, fire the callback then (idempotent via the notify
   flag). The independent 60s periodic sweep (§7.3) is the same logic on a
   timer.

---

## 6. Testing

1. `cargo build` green.
2. **MCP handshake:** pipe `initialize` + `tools/list` as JSON lines into the
   binary's stdin, assert the 4 tools come back on stdout. (stdio smoke
   test, no client needed.)
3. **Live southbound:** with the opencode2 service up, run a trivial
   `opencode_task(wait=true)` against a **free** model
   (`opencode-go/ox-alpha-free` has zero balance issues; the default
   `deepseek` key shows "Insufficient Balance" 402) and assert the output
   text.
4. **End-to-end callback proof:** run the bridge with
   `CLAUDE_CODE_MESSAGING_SOCKET` set to this Claude Code session's own
   socket, launch an async `opencode_task`, and confirm the completion
   message actually arrives in the session. This is the one test that
   proves the whole point; "tools/list works" does not.

Free model for tests: `{"providerID":"opencode-go","id":"ox-alpha-free"}`.

---

## 7. Robustness

SSE is a **latency optimization, not the correctness mechanism.** Correctness
comes from polling opencode (the source of truth); SSE just makes
notifications fast.

1. **Registration ordering (race kill).** Register the session in the
   registry BETWEEN `POST /session` (have the id) and `POST /prompt` (work
   starts). Zero window for a terminal event on an unregistered session,
   even for fast-failing tasks (bad model, auth error emitting
   `session.execution.failed` immediately).
2. **Reconnect sweep.** On every SSE (re)connect, for every tracked
   non-terminal session `GET /api/session/{id}`; if already idle (has
   `outcome`), fire notify then. Idempotent via the notify flag.
3. **Periodic sweep (backstop).** Independent of SSE, every ~60s sweep
   tracked non-terminal sessions the same way. Converts any missed event
   (half-open TCP, opencode restart, frame parse error) from a hang into a
   ≤60s delay.
4. **SSE read/idle timeout.** Set a read timeout on the SSE stream so a
   half-open connection triggers reconnect instead of blocking forever.
5. **`wait=true` cap.** The MCP client enforces its own tool-call timeout
   (e.g. `MCP_TOOL_TIMEOUT`); a long sync call gets killed and the task
   keeps running invisibly. Cap the bridge-side wait (currently 240s,
   configurable). On cap: return
   `{session_id, status:"running", note:"still running; will notify"}` and
   flip the session to notify mode. Disable reqwest's per-request timeout
   on the `/wait` call specifically. A sync task's terminal event still
   flows through SSE — the notify flag must prevent a double report.
   **RAII claim guard.** The pre-claim must be an RAII guard —
   release-on-drop (`notified=false` AND force `notify=true`), explicit
   `.commit()` only on full success. Non-success exits otherwise leak the
   claim and suppress the eventual callback: `/wait` errored, a post-wait
   fetch failed, or the per-call task is dropped when the client cancels
   the MCP call. Release-on-drop makes cancellation + every error path safe
   by construction. Self-heal: a claim released after SSE already dropped
   the terminal event is recovered by §7.3 (re-derives from state:
   `status==Running` + outcome + claim free), not from events.
6. **Per-call concurrency.** Handle each `tools/call` on its own spawned
   task. A `wait=true` call must not block another concurrent call.
7. **stdout purity.** One stray byte on stdout corrupts the MCP protocol.
   ALL logging → stderr, no exceptions. Audit deps for stdout writes.
8. **SSE parsing.** Use `eventsource-stream` rather than hand-splitting
   `data:` lines — multi-line data fields, `:` comment keepalives, and
   frames split across chunks are all real and break naive parsers.
9. **Notify payload cap.** Truncate the injected callback text to ~3KB with
   a tail pointing at `opencode_sessions(session_id=...)` for the full
   output. A raw final text can be tens of KB and burns the caller's
   context.
10. **Credential rot.** If opencode2 restarts, `pair`'s port/password
    change. On connection-refused or 401 from any call, re-run
    `opencode2 pair` once and retry; if still failing, return a clear
    "opencode server unreachable — re-pair" error from the tool.
11. **Notify failure path.** If the AF_UNIX write fails (client gone), log
    to stderr and keep `last_text` in the registry so `opencode_sessions`
    still works. No retry-loop against a dead socket.

---

## 8. Correlation on a shared opencode server

The opencode server is shared — the TUI, cron, other client sessions all
create sessions and run turns on it. Our global `GET /api/event` stream sees
ALL of them; a foreign `session.execution.succeeded` is byte-identical to
ours. Two layers keep us correct:

**Layer 1 — live ownership (primary mechanism).** We mint every session via
our own `POST /api/session` and record its `ses_...` id in the registry
BEFORE the first `/prompt`. The SSE consumer and ALL sweeps filter strictly
on `data.sessionID ∈ registry`. Foreign sessions are never in the registry
→ their events are dropped. Session ids are globally unique, so there is no
ambiguity. Sweeps iterate OUR registry and `GET /api/session/{id}` by id —
they never scan all sessions and never react to a foreign terminal event.

**Layer 2 — durable tag (rediscovery across bridge restart / other client
sessions).** The in-memory registry dies on restart; `opencode_sessions`'s
list-mode fallback `GET /api/session` returns ALL sessions. So stamp ours
with a marker the server persists and the list returns:

- **`title` = `cc-bridge:<origin>:<slug>`** on create. `Session.Info.title`
  IS returned in the session list, so `opencode_sessions` filters by the
  `cc-bridge:<origin>:` prefix in one list call — no N fetches.
- **`<origin>`** = the launching client session's identity so we don't
  reclaim another client session's bridge sessions. Derive it from
  `CLAUDE_CODE_MESSAGING_SOCKET` — the path embeds the client pid
  (`/tmp/cc-socks/89211.sock` → `89211`). Use that pid (or a hash of the
  socket path if the filename isn't a bare pid). Fall back to a
  per-process random id if the socket env is empty.
- **`metadata:{origin:"cc-bridge:<origin>", bridge:true}`** on every
  `POST /prompt` as the authoritative backstop (survives a user renaming
  the title in the TUI; title is user-visible/editable, metadata is not).
  Title = fast list path; metadata = truth if the title was changed. On
  rediscovery: filter by title prefix first; the metadata stamp is free
  provenance but not a scan target (the per-message-fetch N+1 trap).

**INVARIANT — origin is a label, never a capability.** We notify ONLY
sessions THIS bridge process launched (tracked in-memory with a live notify
flag), through THIS process's own client socket. A session rediscovered via
title/metadata origin-match must NEVER auto-notify. Consequence: pid
recycling (a new client session reusing a dead one's pid) can only mislabel
an `opencode_sessions` entry — never misroute a notification. Mitigation:
include each session's created-time in `opencode_sessions` output so stale
entries are obvious. Keep pid-derived origin (it survives an MCP-server-only
restart within the same client session — the one rediscovery case that
matters; a timestamp-salted id would break it).

**Child/forked sessions:** a subagent or `session.fork` gets its OWN
`ses_id` with `parentID` = ours. Our registry filter drops its events (we
don't want a notify per subagent) — correct. A parent that spawns a
subagent blocks on it: observed live, the child's
`execution.succeeded` lands ~7s before the parent's
`execution.succeeded`. Our async "done" callback fires only when the whole
nested turn is complete — it will not lie. The child is a distinct `ses_id`
with `parentID`=parent; our registry filter drops the child's terminal
event → no spurious per-subagent notify.

**Accepted limitations** (documented, not engineered around): a human can
open one of OUR sessions in the TUI and type into it — a foreign turn on a
session we own. We'd report that turn's last output as ours. Two client
sessions both sending to one rediscovered session. Same class, low stakes.
