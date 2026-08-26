# opencode-bridge

A small **MCP stdio server** that gives Claude Code `Agent`/`SendMessage`-style tools to drive [**opencode2**](https://github.com/sst/opencode) through its HTTP API + SSE event stream — no `opencode2 run` subprocesses.

The headline feature: launch an async task and **get a completion callback pushed back into the launching Claude Code session** the instant the turn goes idle, instead of polling. This callback channel uses Claude Code's cross-agent messaging protocol (AF_UNIX inbox socket) — MCP clients other than Claude Code can use the tools but won't receive async notifications.

```
┌────────────────────┐         stdio (JSON-RPC)        ┌─────────────────┐
│  Claude Code       │  ───────────────────────────▶   │  opencode-      │
│  (MCP client       │  ◀───────────────────────────   │  bridge         │
│   with inbox)      │                                 └────────┬────────┘
└────────┬───────────┘                                          │
         │  AF_UNIX inbox socket                                │ HTTP + SSE
         │  (CLAUDE_CODE_MESSAGING_SOCKET)                      │
         ▼                                                       ▼
   CC's session inbox                                  opencode2 service
   (callback text)                                    (pair /api /api/event)
```

## Tools

Four MCP tools, each broader than they look:

| Tool | What it does |
| --- | --- |
| `opencode_task` | Start a new opencode2 session **or** follow up on an existing one. Async by default — fires the task and notifies your Claude Code session when done (requires CC's inbox protocol; other MCP clients use `wait=true`). |
| `opencode_sessions` | Inspect one session's detail (outcome, cost, tokens, last output) **or** list this CC session's sessions. Survives an MCP restart. |
| `opencode_cancel` | Interrupt a running session's current turn. |
| `opencode_catalog` | Browse the server's agents and models as a scored, agents-first text table with a substring/AND filter. |

The tool schemas in [`src/tools.rs`](src/tools.rs) are the source of truth for arguments.

## Install

```sh
cargo build --release
mkdir -p ~/.local/bin
install -m 0755 target/release/opencode-bridge ~/.local/bin/opencode-bridge
```

Then register with Claude Code (user scope works across all projects):

```sh
claude mcp add -s user opencode-bridge -- ~/.local/bin/opencode-bridge
```

Restart Claude Code. The bridge runs as a child of the CC process — no daemon, no port to manage.

### Prerequisites

- **Rust** ≥ 1.75 (uses the 2021 edition).
- **opencode2** installed and the service running. The bridge calls `opencode2 pair` at startup; if the service is down, the bridge exits with a clear stderr message — start it with `opencode2 service start`.
- The `opencode2` binary location: defaults to `~/.opencode/bin/opencode2`. Override with `OPENCODE2_BIN`.
- **Unix (macOS or Linux).** The bridge uses `AF_UNIX` for the CC callback channel; there is no Windows transport.

## Configuration (env vars)

| Variable | Purpose | Default |
| --- | --- | --- |
| `OPENCODE2_BIN` | Path to the `opencode2` binary | `~/.opencode/bin/opencode2` |
| `CLAUDE_CODE_MESSAGING_SOCKET` | AF_UNIX path for the CC inbox callback channel. Empty ⇒ callbacks disabled (async tasks still complete, you just don't get pinged). | inherited from CC, else unset |
| `CLAUDE_CODE_MESSAGING_TOKEN` | Optional auth token for the CC inbox socket. | inherited from CC, else unset |

Everything else — port, password, model selection — comes from `opencode2 pair` and is rotated automatically on credential rot (see [How it works](#how-it-works)).

## How it works

Three interfaces, one process:

1. **Northbound — MCP stdio.** Newline-delimited JSON-RPC 2.0 on stdin/stdout. The transport is hand-rolled (initialize / notifications/initialized / tools/list / tools/call). Logs go to stderr only; stdout carries protocol frames exclusively.
2. **Southbound — opencode2 HTTP + SSE.** The bridge calls `opencode2 pair` at startup to read the bound URL, username, and password, then basic-auths every request. It opens `GET /api/event` (one global SSE stream for every session) and demuxes by `sessionID`.
3. **Sideband — CC inbox socket (Claude Code only).** When a tracked session goes terminal, the bridge fetches the final output and posts a short summary into the launching CC session's inbox over `$CLAUDE_CODE_MESSAGING_SOCKET`. The CC session sees it as a peer message the next time it idles between turns. Other MCP clients can still use the tools (set `wait=true` to get output inline) — they just don't receive async push notifications.

### Why no subprocess per task?

`opencode2 run` is a TUI — it owns stdin/stdout. Driving many of them from one parent is messy and resource-heavy. opencode2 also exposes its full functionality over HTTP + SSE, with the server as the source of truth for session state. Driving it over HTTP keeps the bridge near-stateless: if it dies, you can re-discover your own sessions from the server's session list (we tag ours with `cc-bridge:<origin>:<slug>` on the title) and call back in to keep going.

### Robustness

The hard part isn't the happy path — it's surviving SSE drops, opencode restarts, and CC killing a long `wait=true` call mid-flight. The hard-earned details live in [`SPEC.md`](SPEC.md) §7:

- **Registration-before-prompt** race guard
- **Reconnect reconcile** + independent **60s periodic sweep** (correctness comes from polling opencode; SSE is just for latency)
- **RAII notification-claim guard** for `wait=true` so a CC-side call cancellation doesn't leak the notify slot and silently suppress the eventual async callback
- **Credential rot** — on connect failure or 401, re-run `pair` once and retry; the opencode service rotates its port/password on restart
- **Per-call concurrency** so a slow `wait=true` doesn't block other concurrent `tools/call`s

### Accepted limitations

Documented in [`SPEC.md`](SPEC.md) §8, not engineered around:

- A human can type into one of our sessions via the opencode TUI — that's a foreign turn on a session we own, and we'd surface its output as ours.
- Two CC sessions sending to one rediscovered session is undefined.
- `origin` is derived from the CC socket's PID. PID recycling can only mislabel a list entry, never misroute a notification (the notify path runs through the live in-process registry, not the title tag).

## Development

```sh
cargo build                  # debug build
cargo test                   # unit tests (pure functions; see SPEC §6)
cargo clippy -- -D warnings  # lint
cargo fmt --check            # formatting
# Manual smoke tests against a live opencode2 service — see below and SPEC §6.
```

To exercise the bridge against a live opencode2 service:

```sh
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | ~/.local/bin/opencode-bridge
```

Should reply with `initialize` and `tools/list` frames on stdout.

For a free test model (the default `deepseek` key returns "Insufficient Balance" 402):

```sh
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"opencode_task","arguments":{"prompt":"Reply with exactly: BRIDGE-LIVE","model":"opencode-go/ox-alpha-free","wait":true}}}' \
  | ~/.local/bin/opencode-bridge
```

## Documentation

- [`SPEC.md`](SPEC.md) — full functional spec: API facts, MCP protocol, tool schemas, robustness contracts.
- [`docs/internal/`](docs/internal/) — provenance and design notes kept private to the repo.

## License

MIT — see [`LICENSE`](LICENSE).
