# opencode-bridge — Handoff Document

> **LIVING DOCUMENT**: Section 1 is updated each session. Section 2 is append-only.
>
> ## Instructions for Future Agents
>
> **On session start:**
> 1. Read this file and any linked spec/design files
> 2. Check "Current Phase" and "Open Items" to know where to continue
>
> **During session:**
> - Update "Current Phase" if you advance to a new phase
> - Update "Open Items" as items are resolved or new ones surface
>
> **On session end:**
> 1. Update Section 1 with current state
> 2. Append a new entry to Section 2 (Session Log)

---

## Section 1: Current State

### Objective

`opencode-bridge` (`/Users/ajeesh/projects/madaboutcode/opencode-mcp`) is a single-binary Rust MCP stdio server that gives Claude Code (and any MCP client) four tools to drive **opencode2** over HTTP + SSE — no `opencode2 run` subprocesses. Async tasks push completion callbacks into the launching CC session over its AF_UNIX inbox socket. This session prepared the existing working codebase for **open-source release under MIT**.

### Current Phase

**OSS Release Prep + Catalog Ranking — COMPLETE (2 refine-loop passes, gates green)**

The repo is shippable as OSS. The catalog text-output change is complete, with BM25 ranking over strict AND matches, plain-text MCP delivery, and internal task-file package exclusions. A fresh `luna-high` review is clean after the cap-edge fix. Gates are green (`cargo build --all-targets --locked`, `fmt --check`, `clippy -D warnings`, `cargo test --all-targets --locked` 27 tests, `cargo package --list` excludes `docs/internal` and `tasks`, MCP handshake and live catalog smoke checks).

### Decisions & Rationale

1. **Repo name `opencode-bridge` (keep)**: Crate name already `opencode-bridge`, consistent, no rename. (Rejected: rename — no benefit.)
2. **Handoff moved to `docs/internal/handoff-2026-08-26.md` + `docs/internal/README.md`**: Prior build-session handoff is session-private provenance — valuable for maintainers but not front-page OSS. `docs/internal/` is excluded from `cargo package` via `Cargo.toml` `exclude`.
3. **SPEC.md rewritten §5**: Original SPEC §5 described 7 tools (`opencode_run`/`opencode_send` world); implementation consolidated to 4 (`opencode_task`, `opencode_sessions`, `opencode_cancel`, `opencode_catalog`). Rewrote §5 to 4-tool surface; `src/tools.rs` doc comment `7 MCP tools → 4`. Removed session-private references (`fable`, `greybeard`, Python prototype `~/.opencode-bridge/server.py`, SDK-layout-surprise line). Kept robustness §7 and correlation §8 contracts verbatim.
4. **Cargo.toml crates.io metadata**: Added `description`, `license="MIT"`, `repository`, `keywords`, `categories`, `rust-version="1.75"`, `readme`, `exclude`.
5. **CI `Swatinem/rust-cache@v2`**: Pass-1 P1 — `actions/swfriedberg/cargo-cache@v2` does not exist; fixed. Pass-2 added `cargo package --list` anti-leak check + MSRV check.
6. **30s default HTTP timeout (pass-1)**: `reqwest::Client::new()` had no timeout — ordinary calls could hang forever, blocking the 60s sweep. Added `DEFAULT_REQUEST_TIMEOUT=30s` on the client. **Pass-2 fix**: `/event` SSE stream was killed by that 30s total timeout — changed `send_raw` to `Option<Duration>` (ordinary calls `Some(30s)`, `/wait` `Some(300s)`, `/event` `None`; per-read idle timeout stays in `sse.rs`).
7. **`wait=true` timeout (pass-2 P1)**: Pass-1 `wait()`→`request_with_timeout(Some(300s))` edit failed to apply (oldString mismatch); `wait()` was still on `request()` → 30s. Pass-2 fixed to `Some(300s)` so `WAIT_CAP` (240s) races against app-level timeout, not HTTP layer.
8. **MCP double-wrap (pass-2 Critical)**: `handle_tools_call` returned a full JSON-RPC envelope, `handle_request` wrapped again → `result.result`. Fixed: `handle_tools_call` returns MCP payload (`{"content": [...]}` + `isError` at result level), `handle_request` wraps once via `reply_opt`. Also fixed `isError` placement (was inside content item, now at result level per MCP spec). Added `tools_call_result` helper.
9. **MCP protocol negotiation + notification reply**: Hardcoded `2024-11-05`; now echoes client's `params.protocolVersion` when in allowlist (`is_supported_protocol_version` + `DEFAULT_PROTOCOL_VERSION`). `reply_opt` returns `None` for missing `id` (notification-style requests get no reply).
10. **Registry phantom on prompt failure**: `task()` registered before `/prompt`; if `/prompt` failed, phantom "running" entry remained. Fixed: `unregister` on failure (START branch and foreign-session followup). For already-tracked followups, `snapshot` before `reset_for_followup` and `restore` on failure — prevents sweep from seeing phantom running turn and emitting stale callback with prior output.
11. **Sync claim race (pass-2 High)**: `wait=true` pre-claim was taken in `wait_and_finish` after `/prompt` — fast-fail terminal could win SSE before claim (double report). Fixed: pre-claim taken in `task()` BEFORE `/prompt` (wait=true only) and threaded into `wait_and_finish(pre_claim)`. RAII Drop handles `/prompt` failure; `unregister` is no-op for Drop when entry gone.
12. **Strict arg validation (pass-2 Medium)**: `opt_bool`/`opt_str` silently defaulted on wrong types (`session_id: 42` → new session, `notify: "false"` → true). Now `opt_bool`→`Result<bool>`, `opt_str_strict`→`Result<Option<String>>`; all call sites use `?`; `delivery` enum validated (`queue`/`steer`).
13. **Metadata fallback honesty**: SPEC §8 claimed `metadata` as authoritative backstop for rediscovery; code only filters `title` (N+1 trap). Updated SPEC to say current implementation is title-only; metadata is provenance, rename drops tag until next prompt.
14. **Unix-only**: Added "Unix (macOS/Linux)" to README Prerequisites, SECURITY.md threat model, SPEC §2; `notify.rs` still `UnixStream` — correctly no Windows transport.
15. **--version/--help**: `main.rs` now handles `--version`/`-V` and `--help`/`-h` before discovery (CONTRIBUTING referenced `--version` but it didn't exist).
16. **README fixes**: `mkdir -p ~/.local/bin` before `install`; token row "optional" (was "required on Windows"); `cargo test` description corrected (unit tests, manual smoke tests below).
17. **SSE notify cap ref**: `sse.rs` `cap_notify_text` was `opencode_result("ses_...")` (nonexistent tool) → `opencode_sessions(session_id="...")`.
18. **Tests**: 27 unit tests (tools: definitions surface, parse_model, matches_query, BM25 ranking, row formatting, slugify, latest_assistant_text, catalog filter; MCP: string/structured result encoding; registry: Status from_outcome, as_str, claim idempotency, reset re-arm, unregister). NotifyClaim drop/commit, wait-cap, and correlation regressions remain deferred quick wins.
19. **Catalog output**: `opencode_catalog` now returns plain-text rows, with `kind` omitted defaulting to `all`; agents and models have separate fixed-width sections and a shared 200-row cap.
20. **Catalog ranking**: Preserve strict AND substring eligibility, then rank eligible rows with dependency-free BM25 normalized to 0–50. Agents receive a 50-point preference bonus. BM25 was chosen over TF-IDF because it handles short catalog descriptions and document-length differences better without adding a crate.
21. **MCP string results**: Emit `Value::String` tool results verbatim in MCP text content; retain JSON encoding for structured results. This prevents plain-text catalog output from arriving with JSON string quotes.
22. **Internal package exclusion**: Exclude both `docs/internal/**` and local `tasks/**` from `cargo package`; CI checks both paths.
23. **Catalog cap edge**: Keep a section header whenever that section has matched entries, even when the shared cap gives it zero rows. This makes capped results explicit rather than implying the section had no matches.

Rejected / Deferred:
- **Per-turn claim scoping** (handoff "Per-turn claim scoping" followup): `notified` is per-session, not per-turn; concurrent `wait=true` + async on same session can cross. Documented in SPEC §8 "Accepted limitations" + `opencode_task` description; fix is per-turn generation counter — deferred to future release.
- **Reconnect churn** (handoff followup): `:` SSE keepalives swallowed, 60s read-timeout reconnects every 60s on quiet stream — harmless, deferred.
- **Metadata-backed rediscovery N+1**: would need per-message fetches — deferred, documented.

### Explicit Constraints

- **Must**: MIT license; git repo on `main`; lean deps (no MCP SDK, hand-rolled stdio per SPEC §4); pure in-memory registry (no disk job store, opencode server is source of truth per SPEC §3); ALL logs to stderr (stdout is protocol, §7.7); no new deps without justification.
- **Must**: SPEC §7 (11 robustness items) and §8 (origin-is-label invariant, live-ownership, durable tag) are non-negotiable contracts — reviews enforced them.
- **Must**: Use `luna-high` as adversarial reviewer; refine-loop with builder (this agent) + luna-high verifier; depth = real bugs + OSS-readiness, stop before pedantic.
- **Must not**: Ship `docs/internal/` or `tasks/` in `cargo package`; leak private session notes (fable/greybeard/Python prototype) into public SPEC/README.
- **User tone**: Wants concise, factual, no superlatives/praise/emojis (per system prompt). User said "continue" twice — wants autonomous progress.
- **Environment**: Darwin (macOS), `~/.cargo/bin` rust 1.93.0, opencode2 v0.0.0-beta-17898, crates `tokio`, `reqwest` (json+stream+rustls), `serde`, `serde_json`, `futures-util`, `eventsource-stream`. No MCP SDK. `zsh` loop var `path` clobbers `$PATH` — use `ep` etc. (handoff gotcha).

### Approved Outputs & Assets

- **Repo**: `git init -b main` at `/Users/ajeesh/projects/madaboutcode/opencode-mcp`, 3 commits (`d5fe3c1` init, `5ff4eab` pass-1, `ab98d01` pass-2), `user.email`/`user.name` set to `ajeesh@users.noreply.github.com`/`Ajeesh`.
- **OSS scaffold**: `.gitignore`, `LICENSE` (MIT 2026 Ajeesh), `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1), `.editorconfig`, `.gitattributes`, `.github/workflows/ci.yml`, `.github/ISSUE_TEMPLATE/bug_report.md`, `.github/ISSUE_TEMPLATE/feature_request.md`, `.github/PULL_REQUEST_TEMPLATE.md`, `Cargo.toml` metadata, `SPEC.md` (reconciled), `docs/internal/README.md` + `handoff-...md`, `src/tools.rs` catalog ranking and formatting.
- **CI**: `ubuntu-latest` + `macos-latest`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `cargo fmt --check` + `clippy -D warnings` + `cargo build --locked` + `cargo test --locked` + `cargo package --list` anti-leak for `docs/internal` and `tasks` + MSRV check.
- **Gates verified**: `cargo build --all-targets --locked` green, `cargo fmt --check` green, `cargo clippy --all-targets -D warnings` green, `cargo test --all-targets --locked` 27 passed, package excludes `docs/internal` and `tasks`, MCP handshake returns 4 tools, catalog all/agents smoke checks pass, `--version` prints `0.1.0`.

### Scope & Boundaries

- **Included**: Git init, OSS scaffold, SPEC §5 reconciliation, Cargo metadata, catalog text output with BM25 ranking, MCP plain-string result handling, 27 unit tests, pass-1 (11 findings) + pass-2 (11 findings) fixes, gates verification.
- **Excluded**: Publishing to crates.io / GitHub (no remote set); actual `claude mcp add` re-registration (user does); Windows transport; per-turn claim generation counter; metadata-scan rediscovery; reconnect jitter.
- **Deferred (DO NOT implement now)**: Per-turn claim scoping, reconnect churn jitter, metadata-backed rediscovery N+1, user-supplied stable tag for cross-CC-restart correlation (handoff open items). All documented in SPEC/Changelog.

### Learnings & Gotchas

- **Pass-1 `wait()` timeout edit failed silently**: `oldString` whitespace mismatch → `wait()` stayed on 30s; pass-2 caught it. Always verify edits applied via `grep` after.
- **`reqwest::Client::timeout` is total timeout**: Setting `Client::builder().timeout(30s)` kills long-lived SSE `/event` after 30s. Fix is `Option<Duration>` (None for SSE, Some for others). Per-read idle timeout in `sse.rs` is separate.
- **MCP `isError` placement**: Must be at result level (`{"content": [...], "isError": true}`), not inside content item. Easy to misplace when refactoring `handle_tools_call`.
- **`cargo fmt` rewraps `model`/`variant` lines**: `tools.rs` `cargo fmt` changes `let model = a.model.as_ref().map(|m| format!(...))` to multi-line — breaks `oldString` matching. Read file before editing.
- **`path` loop var in zsh clobbers `$PATH`**: Use `ep`/`other`.
- **CC callback surfacing nuance**: Callback delivered instantly but surfaces only when CC session is idle between turns — delayed/out-of-phase is normal, not a bug (handoff §2).
- **Free test model**: `opencode-go/ox-alpha-free` (`{"providerID":"opencode-go","id":"ox-alpha-free"}`); default `deepseek` returns 402 Insufficient Balance.
- **`opencode2 pair` rotates port/password per service instance**: Always read fresh at startup + `Client::repair` on 401/refused.
- **Message list newest-first, `type` not `role`**: `latest_assistant_text` picks by `max(time.created)`, filters `type=="text"`, ignores `reasoning`.
- **Subagent parent blocks on child**: Child `execution.succeeded` ~7s before parent's — async callback only when whole nested turn done, correct per correlation.
- **Reviewer independence**: Don't prime reviewer with previous findings/checklists; give scope + lens + "say clean if clean" + "stop before pedantic".
- **BM25 with strict AND**: A matched-term fraction score is constant after strict AND filtering; BM25 preserves narrow matching while ranking by term frequency and document length without a new dependency.
- **Dirty package contents**: `cargo package --allow-dirty` included untracked `tasks/` files; exclude private handoff/plan paths explicitly and check both `docs/internal` and `tasks`.
- **Shared catalog cap**: A section header must reflect matched entries, not just displayed rows; a capped-out section can have zero rows while still needing its header.

### Dead Ends

- None in this session — prior handoff notes the Python `~/.opencode-bridge/server.py` prototype was superseded by Rust HTTP+SSE design (no subprocess-per-task). Not retried.

### Quick Wins

- Add regression tests for NotifyClaim drop/commit semantics, MCP envelope single-wrap, wait-cap path, `opt_bool`/`opt_str_strict` rejection, and snapshot/restore — would have caught the double-wrap and timeout regressions earlier.
- Add `cargo package --locked` to CI (now done) and consider `cargo audit` / `cargo deny`.
- Fresh final `luna-high` verification is clean after fixing catalog ranking, provider handles, package exclusions, and the shared-cap section-header edge.
- README badges (build, version, license) once GitHub remote is set.
- Issue: `actions/swfriedberg/cargo-cache` typo — already fixed; watch for similar in forks.

### Open Items

1. **Set GitHub remote + push**: No remote configured. Create `github.com/madaboutcode/opencode-bridge` (or chosen org), `git remote add origin <url>`, `git push -u origin main`, set branch protection, verify CI green on GitHub.
2. **Publish decision**: `cargo publish` to crates.io? Needs `repository` + `license` already set; decide if `Cargo.lock` should be committed (currently is — correct for binary).
3. **Release tag `v0.1.0`**: `git tag v0.1.0 && git push --tags`, GitHub Release with CHANGELOG.md notes, attach `target/release/opencode-bridge` binary.
4. **Verify `claude mcp add -s user opencode-bridge -- ~/.local/bin/opencode-bridge` still works** after release build (binary at `~/.local/bin/opencode-bridge` symlinked to `target/release/opencode-bridge`).
5. **UNSPECIFIED — GitHub repo visibility**: User said "prepare for opensource release" but did not state public vs private initial push. Confirm before pushing.
6. **UNSPECIFIED — LICENSE copyright holder**: Currently `Copyright (c) 2026 Ajeesh` — confirm full legal name / org if different.
7. **UNSPECIFIED — `repository` URL in Cargo.toml**: Currently `https://github.com/madaboutcode/opencode-bridge` — confirm final org/repo name matches created remote.
8. **Deferred code fixes** (see Decisions→Rejected): per-turn claim generation counter, reconnect jitter, metadata-scan rediscovery — file as GitHub issues if desired, not blocking v0.1.0.

### Key Files

| File | Purpose |
|------|---------|
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/SPEC.md` | Functional spec — API facts (§1), CC callback (§2), Rust design (§3), MCP protocol (§4), tool schemas §5 (4 tools), testing §6, robustness §7 (11 items), correlation §8 — source of truth; reconciled this session |
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/src/main.rs` | Startup: `opencode2 pair` discovery, health check, notifier/origin/default_dir, spawn SSE + sweep, `mcp::serve_stdio`; now has `--version`/`--help` |
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/src/tools.rs` | 4 tool definitions + dispatch (`opencode_task`, `opencode_sessions`, `opencode_cancel`, `opencode_catalog`); `task()` with register/ snapshot+pre-claim+prompt+rollback, `wait_and_finish(pre_claim)`, `list_sessions`, BM25-ranked catalog output, 19 unit tests |
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/src/mcp.rs` | Hand-rolled JSON-RPC stdio: `serve_stdio`, `handle_request` (initialize negotiation, tools/list, tools/call single-wrap), `reply_opt`, writer task, plain string/structured result encoding, 2 tests |
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/src/opencode.rs` | HTTP client: `Creds`, `pair`, `Client` (30s default, `Option<Duration>` per-call, `None` for SSE), `wait` (Some(300s)), `events` (None), model/agent/session/message shapes |
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/src/registry.rs` | In-memory `Registry`: `register`, `unregister`, `snapshot`/`restore`, `reset_for_followup`, `claim_notification`/`claim_notification_guard` + `NotifyClaim` RAII, `set_result`, `running_session_ids`, `list`; 6 tests |
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/src/sse.rs` | SSE consumer: `run` (reconnect + backoff + reconcile), `periodic_sweep` (60s), `handle_event_line` (eventsource-stream), `complete_session`, `cap_notify_text` (now `opencode_sessions`) |
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/src/notify.rs` | AF_UNIX CC callback: `Notifier::from_env`, `notify` (best-effort, log-on-fail) |
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/src/state.rs` | `AppState { client, registry, notifier, origin, default_dir }` |
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/src/error.rs` | `type Result<T> = Result<T, Box<dyn Error+Send+Sync>>` |
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/Cargo.toml` | Crate metadata: `opencode-bridge 0.1.0`, `rust-version 1.75`, `exclude` for `docs/internal/**` and `tasks/**` |
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/README.md` | Public face: tools, install (`mkdir -p`), prereqs (Unix-only), env vars, how-it-works, robustness, limitations, dev commands, docs |
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/.github/workflows/ci.yml` | CI on `main` push/PR, matrix `ubuntu/macos`, `fmt`+`clippy`+`build`+`test`+`package --list`+MSRV |
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/docs/internal/handoff-2026-08-26.md` | Prior build-session handoff (moved from `tasks/`; private provenance, excluded from package) |
| `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/opencode-bridge.handoff.md` | **This file** — living handoff for OSS release prep |

### Environment Setup

```bash
# Verified commands to get into working state
cd /Users/ajeesh/projects/madaboutcode/opencode-mcp
cargo build                  # debug build — green
cargo test                   # 27 tests — green
cargo clippy -- -D warnings  # lint — green
cargo fmt --check            # formatting — green
cargo package --locked --allow-dirty --no-verify --list | grep -Eq 'docs/internal|tasks/' && echo LEAKED || echo "EXCLUDE OK"
./target/release/opencode-bridge --version  # prints 0.1.0
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | ./target/release/opencode-bridge 2>/tmp/bridge-stderr.log | head -1 | python3 -m json.tool | head -20
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"opencode_task","arguments":{"prompt":"hi"}}}' \
  | timeout 5 ./target/release/opencode-bridge 2>/tmp/bridge-stderr2.log | tail -1 | python3 -m json.tool
git log --oneline -5
git status --short
```

### How to Resume

1. Read this file + `SPEC.md` + `README.md` + `Cargo.toml`
2. Run Environment Setup commands above
3. Continue with **Open Items** (set remote, push, tag, verify `claude mcp add`)
4. Catalog output is complete and reviewed. Continue with **Open Items**: set the remote only after confirming GitHub visibility, copyright holder, and final repository URL.

---

## Section 2: Session Log

### Session 1 — 2026-08-26 (OSS release prep, refine-loop)

**Phase**: OSS Release Prep — COMPLETE (2 passes)

**Work done**:
- `git init -b main`, `.gitignore`, `LICENSE` (MIT), `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `.editorconfig`, `.gitattributes`, `Cargo.toml` metadata, `.github/workflows/ci.yml`, issue/PR templates — committed `d5fe3c1`.
- Rewrote `SPEC.md` §5 (7→4 tools), removed private refs, moved `tasks/handoff`→`docs/internal/`, fixed `tools.rs` 7→4 comment, `cargo build`/`fmt`/`clippy` green, handshake verified — committed.
- **Pass-1** (`luna-high`, 11 findings): CI action fix, 30s HTTP timeout, protocol negotiation, `reply_opt` for notifications, `opencode_result`→`opencode_sessions`, `unregister` phantom rollback, per-turn limitation doc, 16 tests, SPEC §6/§8/README/SECURITY updates — committed `5ff4eab`, gates green, handshake 4 tools.
- **Pass-2** (`luna-high`, 11 findings): Fixed double-wrap `tools/call` (single-wrap + `isError` at result level), `wait()` 30s→300s (pass-1 edit hadn't applied), SSE `/event` 30s→`None` (Option<Duration>), `snapshot`/`restore` for failed followups + pre-claim BEFORE `/prompt` for `wait=true` (closes race), strict arg validation (`opt_bool`/`opt_str_strict` + delivery enum), SPEC §8 metadata honesty, `--version`/`--help`, README token/Unix fixes, CI `package --list`+MSRV — committed `ab98d01`, gates green, handshake single-wrapped verified, `--version` works.

**Learnings**:
- `reqwest::Client::timeout` is total timeout — kills SSE if applied.
- `oldString` whitespace mismatches silently fail edits — verify via `grep` after.
- `isError` must be at MCP result level, not inside content.
- Reviewer independence is load-bearing — don't prime with prior findings.

**Dead ends**:
- None — Python prototype was prior session's dead end (superseded).

**Blockers surfaced**:
- None blocking — remaining work is set remote/push/tag (user decision on org/visibility).

---

<!-- NEXT SESSION: Append below this line -->

### Session 2 — 2026-08-26 (catalog text output and release audit)

**Phase**: OSS Release Prep + Catalog Ranking — COMPLETE

**Work done**:
- Completed the uncommitted `opencode_catalog` redesign: plain-text fixed-width rows, optional `kind=all`, agents-first sections, strict argument validation, shared 200-row cap, and explicit empty/truncation output.
- Replaced the ineffective matched-term fraction ranking with dependency-free BM25 relevance normalized to 0–50, preserving strict AND substring eligibility. Agent rows retain usable `providerID/id` handles.
- Fixed MCP string result encoding so catalog text is emitted without JSON string quotes while structured tool results remain JSON text.
- Expanded focused coverage across catalog ranking/formatting, MCP result encoding, and cap behavior; total suite is now 27 tests.
- Updated README, CHANGELOG, SPEC, Cargo package exclusions, CI leak checks, and the catalog implementation plan. `tasks/**` and `docs/internal/**` are excluded from packages.
- First final review found the shared-cap header edge; fixed it and received a clean follow-up `luna-high` review.

**Verification**:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo build --all-targets --locked`
- `cargo test --all-targets --locked` — 27 passed
- `cargo build --release`
- `cargo package --locked --allow-dirty --no-verify --list` — no `docs/internal` or `tasks` paths
- Live release binary: version, MCP handshake (4 tools), `opencode_catalog kind=agents`, `kind=all`, and strict `kind` validation

**Blockers surfaced**:
- No code blockers. GitHub remote/push/tagging remains blocked until repository visibility, copyright holder, and final repository URL are confirmed.
