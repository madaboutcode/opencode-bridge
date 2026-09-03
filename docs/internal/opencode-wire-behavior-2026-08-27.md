# opencode2 observed wire behavior — spike results (2026-08-27)

Source: three live spikes against `opencode2 serve` (beta-17898, pid 65035),
driven by direct curl on the paired REST API (`/api`). Raw dumps under
`/tmp/ocspike/spike{A,B,C}/` (transient — regenerate if the server restarts).
Purpose: settle, on evidence rather than assumption, how the bridge should
determine a turn's final output and whether subagent invocations are handled
correctly. Feeds the "reply-tag" design decision.

## Scenarios run

| Spike | Scenario | Session |
|---|---|---|
| A | plain prompt, no tools (`deepseek-flash`) | baseline |
| B | one bash tool call (`build` + a model) | `ses_fbcaecc54ffeU4r3t7GRYAH5dt` |
| C | primary (`build`) delegates to a subagent (`explore`) | parent `ses_fbcae6ab9ffeIIkXQbUv6pgusI`, child `ses_fbcae4a4affep123YcJ0IibbC2` |

## 1. Turn lifecycle — exactly ONE terminal event per turn

All three scenarios: the session emits `session.execution.started` exactly
once and one terminal `session.execution.succeeded` exactly once. A tool call
or a subagent delegation does NOT open a second execution round — it's one
execution containing multiple internal `session.step.*` boundaries.

Baseline (A) durable seq: `created(0) … execution.started(2) … step.started(5)
reasoning(6-7) text(8-9) step.ended(10) execution.succeeded(11)`.

Tool call (B): one execution, TWO steps — a tool step
(`tool.input.started/ended`, `tool.called`, `tool.success`) then a text step —
then `execution.succeeded(17)`. Still one started, one succeeded.

Subagent (C): parent had one `execution.started(2)` and one
`execution.succeeded(17)`; the terminal fired only AFTER the child finished and
AFTER the parent's own final text.

**Bridge implication:** the current "one terminal event → complete once"
model matches what opencode emits in these cases. The premature-terminal
("false completion") theory did NOT reproduce in any of the three. It is
downgraded to unsupported — NOT disproven (a slow multi-step subagent chain
was not stress-tested). The `OPENCODE_MCP_LOG=debug` seq-log stays as the
instrument if it ever needs re-testing.

## 2. Subagents run in SEPARATE child sessions — the bridge is safe here

- A subagent gets its own session with a distinct `sessionID`, linked to the
  parent by a `parentID` field (present both on `GET /session/{child}` and on
  the child's `session.created` SSE frame).
- The child's `session.execution.*` events carry the CHILD's sessionID, not
  the parent's. They stream on the same global `/event` connection, interleaved
  by time, distinguished only by `data.sessionID`.
- The child's messages live ONLY under `GET /session/{child}/message`. They are
  NOT merged into the parent's message list.

**Bridge implication:** our SSE consumer demuxes by `sessionID` and only acts
on *tracked* sessions (`claim_notification` returns `None` for anything not in
the registry). We only ever register sessions we prompt, never children. So a
child's terminal event is correctly ignored, and the sweep (which iterates
`running_session_ids()`) never sees children either. Subagent lifecycle cannot
fire the parent's callback. **Confirmed correct — no change needed.**

## 3. Assistant message shape — we are discarding useful fields

`GET /session/{id}/message` returns messages; the array of parts on a message
is called **`content`** (the bridge already uses this name). An assistant
message's top-level keys:

```
id, time, type, agent, model, content, finish, rawFinish,
providerState, cost, tokens
```

- `time.completed` (epoch ms) is present on a FINISHED assistant message; the
  user message has `time.created` but no `time.completed`.
- `finish: "stop"` / `rawFinish: "stop"` mark normal completion.
- `agent` names the agent that produced the message; `model` the model.

The bridge's `opencode::Message` currently deserializes only `type`,
`time.created`, `content`, `error` — it throws away `finish`, `time.completed`,
and `agent`.

### Content part types seen

- `reasoning` — thinking; the bridge already skips these. `{"type":"reasoning", ...}`
- `text` — the answer. `{"type":"text","text":"PONG"}` (no per-part time).
- `tool` — a tool call+result, embedded IN the assistant message's `content`:
  ```
  {"type":"tool","id":"call_…","name":"shell","executed":false,
   "state":{"status":"completed","input":{"command":"…"},
            "content":[{"type":"text","text":"…output…"}],
            "metadata":{"status":"completed","exit":0}},
   "time":{"created":…,"ran":…,"completed":…}}
  ```
- A **subagent** delegation appears as a `tool` part with `name:"subagent"`,
  whose `state.content` holds the subagent's answer wrapped as
  `<subagent sessionID="ses_…" state="completed">…answer…</subagent>`.

Step boundaries (`step.started/ended`, `tool.called`, etc.) exist only on the
SSE stream, NOT as message content parts.

## 4. THE empty-response bug — confirmed, bridge-side, no tag needed

`latest_assistant_text` (opencode.rs:149) picks the assistant message with the
**max `time.created`** and concats its `type:"text"` parts. The failure:

An assistant message that only calls a tool / delegates has
`content = [reasoning, tool]` and **zero text parts**. In a normal completed
run the model emits a final *text* message afterward, so max(time.created)
lands on it (verified in B and C). But if a turn's newest completed assistant
message is that text-less tool/delegation message — model cut short, errored
mid-turn, or delegated and stopped without summarizing — the concat is `""` →
the bridge returns an empty output.

This is a real cause of the empty responses observed in use. It is independent
of the reply-tag idea and fixable purely bridge-side.

### The empties split into two distinct causes → two different fixes

- **(ii) Wrong-message pick / mid-turn text-less latest.** Fix: read the newest
  *completed* assistant message (`finish` / `time.completed` set) that actually
  contains a non-empty `type:"text"` part, instead of blind max(time.created).
  Enrich the `Message` struct to read `finish`/`time.completed`/`agent`. Cheap,
  correctness-only, no model cooperation required.
- **(i) Turn genuinely ended with no text** (delegated/tool'd then stopped, or
  failed). No extraction can invent an answer. Options: surface the `error`
  (already built last session — see below), or surface the embedded
  tool/subagent content, or an explicit "(agent produced no text reply)" note.
  The reply-tag idea addresses only this case, and only best-effort (a cheap
  model may ignore the tag). It is NOT a fix for (ii) and NOT a completion
  sentinel.

### Cheapest win first: redeploy the already-built error surfacing

The previous session added `error` surfacing (failed turn → real reason, e.g. a
provider 402) but the running MCP is still the OLD binary. A chunk of observed
"empties" are failed turns that a rebuild/reconnect will already turn into
visible error text. Do this before judging how many empties remain.

## Gotchas found (harness, not bridge bugs)

- Agent ids are **case-sensitive**: `build`/`explore` work; `Build` fails
  instantly with `Agent not found: "Build"` (→ `session.execution.failed`,
  zero cost/tokens, no messages). The bridge passes the caller's string
  through, so a caller that sends `Build` gets a silent failed session.
- `glm-5.2`'s providerID is `crof`, not `opencode-go` (per this server's
  config). A wrong provider/model also fails the session instantly.
- A failed-to-create session still produces a `session.execution.failed`
  terminal with an empty message list — another path that yields no text.
