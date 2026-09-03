# Spike: Authentication Discovery — service.json vs opencode2 pair

**Date:** 2026-09-01
**Goal:** Determine how a standalone dashboard process should authenticate to the local opencode2 server.

---

## 1. Candidate Credential Files Found

| Path | Relevance | Keys Present |
|------|-----------|-------------|
| `~/.config/opencode/service.json` | **Primary** — local server auth | `password` (only) |
| `~/.local/share/opencode/auth.json` | Provider API keys (OpenRouter, OpenAI, DeepSeek, etc.) — not local server auth | `openrouter`, `fireworks-ai`, `nvidia`, `openai`, `cerebras`, `deepinfra`, `cloudflare-workers-ai`, `deepseek`, `kimi-for-coding`, `google`, `xai`, `opencode-go` |
| `~/.local/share/opencode/account.json` | Account registry for provider keys — not local server auth | `version`, `accounts`, `active` |
| `~/Library/Application Support/opencode/` | **Does not exist** | N/A |
| `~/.opencode/**/service.json` | **Does not exist** | N/A |

**service.json contents (keys only):**
```json
{
  "password": "<redacted>"
}
```
No `host`, `port`, or `username` key present. Confirmed via both direct read and programmatic key extraction.

---

## 2. opencode2 pair Output vs service.json

### opencode2 pair output:
- **URLs:** `http://127.0.0.1:49374`
- **Username:** `opencode`
- **Password:** `<redacted>` (matches service.json password exactly)

### service.json:
- **host:** not present
- **port:** not present
- **username:** not present
- **password:** matches pair output

### Port comparison:
| Source | host | port | username |
|--------|------|------|----------|
| `opencode2 pair` | `127.0.0.1` | `49374` | `opencode` |
| `service.json` | — | — | — |

**The port 49374 in `pair` output matches the current listening port** (confirmed via lsof). But service.json contains no port — only the password.

---

## 3. Is 127.0.0.1:49374 Current or Stale?

**Current.** `lsof -iTCP -sTCP:LISTEN` confirms:

```
opencode2 65035 ajeesh  7u  IPv4 ...  TCP 127.0.0.1:49374 (LISTEN)
```

Process: `opencode2`, PID 65035, listening on `127.0.0.1:49374`.

However, per SPEC.md §1: _"The port changes per service instance — always read it fresh from pair at startup; never cache across restarts."_ The port is **not guaranteed stable across restarts**.

---

## 4. Health Check

```
GET http://127.0.0.1:49374/api/health
Basic Auth: opencode:<password-from-pair>
Result: HTTP 200
```

The server is alive and accepts the credentials from `opencode2 pair`.

---

## 5. How the Existing Bridge Authenticates

From `src/opencode.rs` (lines 196–227):

- `pair()` runs `opencode2 pair`, parses stdout for `URLs`, `Username`, `Password`.
- `Creds` struct: `{ base_url, username, password }`.
- `Client` stores creds in `RwLock<Creds>`, refreshes via `repair()` → re-runs `pair()`.
- All requests use HTTP Basic Auth with username:password.
- **Never reads service.json.** Relies entirely on `pair` for host, port, and credentials.

From `src/main.rs` (lines 67–75):

- Startup: `pair(&bin)` → `health()` → serve MCP.
- On 401 or connection error: `repair()` re-runs `pair()` and swaps creds.

---

## 6. Evidence: Can a Second Process Auth with service.json Alone?

**No — not with service.json alone.** Here is the gap:

| What auth needs | service.json provides | Missing |
|----------------|----------------------|---------|
| host (`127.0.0.1`) | ❌ | hardcoded or from pair |
| port (dynamic) | ❌ | must come from pair |
| username (`opencode`) | ❌ | must come from pair |
| password | ✅ | — |

`service.json` contains only the password. A standalone dashboard would need **all four pieces**: host, port, username, password. The password in `service.json` is the same as `pair` output — but the other three values are absent.

**Options the dashboard has:**
1. **Call `opencode2 pair`** (like the bridge does) — gets all four values. Requires the binary to be available.
2. **Hardcode `127.0.0.1` + known username `opencode` + read password from service.json** — but port is dynamic and not in service.json.
3. **Read service.json + discover port via some other mechanism** (e.g., lsof, a lockfile, an env var) — but no such mechanism was found in the filesystem.

**The requirements assumption (R1.2)** that the dashboard can authenticate via `127.0.0.1:49374` + `service.json` password is **only valid while the server is running on that specific port**. The username `opencode` is also not in service.json — it comes from `pair` output and is effectively a constant (always `opencode` for local instances), but that's an observation, not a documented guarantee.

---

## 7. Summary

- `service.json` exists at `~/.config/opencode/service.json` with key `password` only (no host/port/username).
- `opencode2 pair` returns host, port, username, and password — port 49374 matches the current listener.
- The existing bridge never touches `service.json`; it uses `pair` exclusively.
- **A standalone process cannot authenticate with service.json alone** — it lacks host, port, and username. It needs either `opencode2 pair` or some other discovery mechanism for the port.
- The password in `service.json` is identical to the `pair` output password (same server instance).
