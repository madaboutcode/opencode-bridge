# claude-dashboard-support - Handoff

> Living document - Section 1 is current truth (rewrite each session), Section 2 is
> append-only history. Maintained via the session-continuity skill.

## Section 1: Current State

### Orientation

This run adds opt-in, metadata-only Claude Code monitoring to the dashboard
without transcript access, session control, or Claude configuration mutation.
T01c/T02/T03/T04, envelope remediation, and the live-startup correction are
committed. A real Claude Haiku interactive lifecycle now passes through the
dashboard; broader T05 evidence and Terra's M3 cross-task sign-off remain open.

### Map - read in this order

| Priority | File / section | What to look for |
|----------|----------------|------------------|
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/docs/specs/dashboard/claude.md` | R11-R17: manual opt-in, exact allowlist, privacy, bounded socket delivery, and the completeness boundary. |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/crates/dashboard/src/main.rs` | Normal startup enters Tokio before `ClaudeListener::bind`; this was the live validation fix in `babf167`. |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-support/contracts/T04-claude-runtime.md` | Sealed T04 runtime boundary and ownership; do not expand it into T05 by inference. |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-support/deferred.md` | Remaining T05 evidence and final staleness questions. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/crates/dashboard/src/claude/{command.rs,listener.rs,hook.rs,wire.rs,state.rs,mod.rs}` | The helper, listener, privacy boundary, decoder, adapter state, and shared event channel. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-support/gates/{T01c,T02,T03,T04}-report.md` | Prior gate evidence; live evidence below is newer than the old credential-unavailable note. |

### World-Facts & Tooling

- The branch is `conductor/claude-dashboard-support`; current implementation commits are T01c `401887e`, T02 `aeb8317`, T03 `e631129`, T04 decomposition `bdb8647`, T04 runtime `fd83209`, envelope remediation `04a7cf5`, all-worktree commit `bd35c5b`, and live-startup fix `babf167`.
- `cargo test --workspace` passed after the startup fix: 168 library tests, 8 adapter tests, 38 ingress tests, 20 runtime tests, and 29 bridge tests, with no failures.
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `cargo build -p dashboard` passed after the startup fix.
- `claude --version` reports Claude Code `2.1.259`; `tmux -V` reports `tmux 3.4`; `opencode2 service status` reports the local service at `http://127.0.0.1:49374`.
- Before `babf167`, normal dashboard startup panicked at `listener.rs:204` with `there is no reactor running, must be called from the context of a Tokio 1.x runtime` because synchronous `main` called `UnixListener::bind` before entering `rt`. Entering `rt` around the bind fixed the actual runtime path.
- A real interactive Claude session using model `claude-haiku-4-5-20251001` and temporary command-hook settings caused the dashboard count to move from `27 projects / 252 sessions` to `28 projects / 253 sessions` on `SessionStart`; after the prompt returned `READY`, `/exit` caused `SessionEnd` and the count returned exactly to `27 / 252`.
- The first one-turn `--print --no-session-persistence` probe also delivered `SessionStart`, but a deliberately delayed `SessionEnd` command was canceled by Claude with the exact message `SessionEnd hook [...] failed: Hook cancelled`; use an interactive session when proving live retention and clean removal.
- The live test used `--settings <temporary hooks JSON> --setting-sources project`, an explicit inherited `DASHBOARD_CLAUDE_SOCKET`, and synchronous command hooks for `SessionStart`, `StopFailure`, and `SessionEnd`; no Claude settings file, transcript, or credential value was inspected or retained.

### State & Provenance

- Terra explicitly signed off M1 and M2 and sealed the T04 v1 Review Frame. The live startup correction is a narrow runtime-context fix in `main.rs`; it does not change the T04 protocol boundary.
- T02 owns path resolution, parsing, serialization, delivery deadline, and privacy filtering. T03 owns wire decoding, Claude lifecycle mapping, snapshots, tombstones, and adapter state. T04 must compose those APIs, not duplicate them.
- T04 is the runtime/release wiring phase: `dashboard claude-hook`, user-scoped Unix listener, bounded intake, startup ordering, cleanup, manual docs, and runtime tests. The durable contract owns the complete acceptance boundary.
- The user explicitly directed: "run a claude haiku session with your own custom hooks config (temporarily - maybe in a new tmp project in the ./tmp dir) - and validate that the events are flowing - otherwise how are you to test it - use tmux session to capture etc". That directive is satisfied for one real interactive lifecycle, not silently promoted to complete T05 coverage.
- The live run supersedes only the old claim that authenticated full-path evidence was unavailable: `SessionStart` and `SessionEnd` are now observed through Claude -> hook command -> Unix socket -> listener -> adapter -> dashboard. T05 still owns successful-session async behavior, startup-gap/foreground discovery, exit-path reliability, subagent identity, broader event evidence, and final staleness policy.
- The prior uncommitted icon/OpenCode/prototype work was intentionally included in `bd35c5b` after the user said "commit it all"; it is no longer an unstaged boundary, but future Claude work should not rework it without an explicit scope.

### Judgment - Recommended Next Moves

- Treat the tmux run as concrete live integration evidence, then ask Terra for M3 cross-task sign-off before opening T05; the evidence is strong enough to shape T05, not to waive its ownership gate.
- For another live run, start the dashboard in a tmux pane with a wrapper that keeps the pane open, use an interactive Claude session, observe the dashboard while Claude is idle, then send `/exit`; do not rely on a delayed `--print` hook because Claude may cancel it at process shutdown.
- If project identity matters, put the temporary Claude cwd in a nested disposable git repository or outside this repository; a plain directory under `./tmp` resolves to the parent `opencode-mcp` git root.
- Continue treating hook payloads, Claude configuration, credentials, and transcript JSONL as opaque. Use only temporary settings passed with `--settings` for black-box validation when explicitly requested.

### Dead Ends & Corrections

- T02 v5 initially failed review because it used a shared `/tmp` fallback, did not bind the full delivery deadline over filesystem metadata, had stale spec registries, malformed R15 structure, and lacked busy-listener/log-capture tests. T02 v6 corrected these and is recorded in `gates/T02-report.md`; do not reintroduce the old fallback or requirements.
- T03 Luna's first provider request returned HTTP 404 before review. Retrying with a fresh Luna High session produced the clean verdict; do not count provider failure as a review or treat it as an implementation defect.
- T04 Candidate A (all runtime logic in `main.rs`) and Candidate C (separate binary) were rejected. Candidate B (`claude/command.rs` plus `claude/listener.rs` with narrow main composition) is the sealed choice because it keeps protocol/runtime mechanics testable and preserves `dashboard claude-hook`.
- Direct normal startup before `babf167` was not a valid integration result: it panicked because `UnixListener::bind` requires an entered Tokio runtime. The fix is `rt.enter()` around the bind, not a change to listener ownership or protocol.
- The first tmux capture used `"$SESSION:claude"`, which shell-expanded incorrectly and produced `can't find pane`; use `"${SESSION}:claude"` for a tmux target.
- A delayed synchronous `SessionEnd` hook in `--print` mode was canceled at process shutdown and left the live dashboard session present. That was a probe limitation and a Claude CLI lifecycle observation, not evidence that the interactive path removes sessions incorrectly.
- Combined large patches previously failed when one expected state line was stale or one added line lacked the patch prefix. Smaller file-oriented patches succeeded; use that approach for gate bookkeeping.

### Do-Not-Touch

- Never inspect `~/.claude`, project `.claude`, credentials, or transcript JSONL. A user-requested black-box Claude invocation may use its own opaque auth/config internally, but the test harness must not read or retain those artifacts.
- Do not modify or stage `crates/dashboard/src/claude/hook.rs`, `state.rs`, or `wire.rs`; those are T02/T03 authorities.
- Do not modify provider-neutral `adapter.rs`, `snapshot.rs`, `project_identity.rs`, shell files, OpenCode files, or prior T01c/T02/T03 evidence/gates for a Claude task.
- Do not claim complete Claude behavior from this single live lifecycle. Keep the T05 ownership boundary and its remaining evidence areas explicit.
- Do not commit generated nested `target/` output or machine-local runtime artifacts; the root `.gitignore` now covers nested Rust targets.

### Open Items - triaged

**Blocks next phase (resolve first):**

1. Terra's M3 cross-task sign-off is required before T05 decomposition.
2. T05 must retain ownership of async/successful-turn behavior, startup-gap/foreground discovery, exit-path reliability, subagent identity, final staleness, and the remaining evidence promotions.

**Resolvable during the work:**

3. Convert the live interactive evidence into the next T05 gate artifact without copying raw hook payloads or transcript data.
4. Any platform-specific listener behavior must be documented as observed or deferred, not generalized from macOS.

**UNSPECIFIED (ask, don't guess):**

5. Whether the product should require synchronous hooks for reliable `SessionEnd`, or explicitly define an async-hook/staleness policy, remains a T05 product/runtime decision.

### Working Commands

```bash
# Verify branch, unrelated dirt, and staged/unstaged state
git status --short --branch

# Full workspace tests, including the Claude runtime and ingress paths
cargo test --workspace

# Workspace quality gates verified after the live-startup fix
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo build -p dashboard

# Runtime/tool availability used for the live black-box check
claude --version
tmux -V
opencode2 service status
```

---

## Section 2: Session Log

### Session 1 - 2026-09-03

**Phase**: M1 evidence/ingress -> M2 adapter -> M3 T04 decomposition and implementation

**Work done**:

- Adopted T01c's hash-verified metadata evidence baseline as `401887e`.
- Corrected and committed T02 v6 ingress/privacy work as `aeb8317` after one bounded correction and one clean Luna review.
- Designed, reviewed, implemented, independently reviewed, and committed T03 typed Claude adapter/state integration as `e631129`.
- Obtained Terra's M2 sign-off that T02 and T03 form a coherent handoff.
- Created and committed T04 Candidate B design, implementation plan, and contract as `bdb8647`; Terra sealed the frame and approved isolated `main.rs` runtime hunks.
- Started the single DeepSeek T04 implementation pass; it is still active at handoff creation.

**Learned**:

- The task must preserve a dirty worktree carrying unrelated dashboard UI/OpenCode work; exact owns-lists and explicit staging are essential.
- Shared `/tmp` fallback, incomplete deadlines, stale registries, and weak socket tests were concrete T02 review failures and are now corrected in the durable T02 gate.
- T03's first Luna provider request failed at HTTP 404, but a fresh retry gave a clean independent verdict; provider failure before output is not a review result.
- T04 cannot safely put all logic in `main.rs`; isolated command/listener modules are the approved runtime boundary.

**Blockers surfaced**:

- Authenticated Claude credentials are unavailable and global Claude paths are off-limits; four evidence promotions and full E2E remain deferred to T05.
- T04 implementation, mandatory spec validation, fresh Luna review, and gate commit are unfinished at this handoff.

---

### Session 2 - 2026-09-03

**Phase**: M3 T04 runtime gate closure

**Work done**:

- Recorded the narrow disposition for the four pre-existing spec-rubric baseline exceptions and corrected the owned `client.md` six-file reference.
- Ran the mandatory Clerk validation; T04-specific spec modifications and `spec-delta.md` passed.
- Completed a first Luna review, corrected helper lifetime, replacement-safe cleanup, and exact-boundary multi-frame handling, then completed the required fresh Luna verification cleanly.
- Wrote `gates/T04-report.md` and committed T04 as `fd83209` with only the approved `main.rs` runtime hunks; the pre-existing icon-mode hunk remains unstaged.

**Current boundary**:

- T05 remains deferred. Authenticated lifecycle, async viability, exit-path reliability, subagent identity, final staleness, and complete full-path evidence are not established by T04.
- Unrelated dashboard, OpenCode, documentation, and temporary worktree changes remain untouched.

---

<!-- NEXT SESSION: Rewrite Section 1 to current truth and append a new session entry below. -->

### Session 3 - 2026-09-04

**Phase**: M3 T04 post-gate live integration verification -> passed with a startup correction and T05 boundary retained

**Work done**:

- Responded to the explicit request to run a real Claude Haiku session with temporary custom hooks, use tmux, and validate actual event flow rather than relying only on fixtures.
- Built `dashboard`, created a disposable hook settings JSON outside `.claude`, created a nested disposable git project under `./tmp`, and ran the dashboard plus Claude in tmux. The test hook command was the absolute `dashboard claude-hook` binary for `SessionStart`, `StopFailure`, and `SessionEnd`, with `DASHBOARD_CLAUDE_SOCKET` inherited by the helper.
- The first normal dashboard launch exposed a panic before Claude started. Entered the Tokio runtime around `ClaudeListener::bind` in `crates/dashboard/src/main.rs` and committed the correction as `babf167`.
- Reran with Claude Code `2.1.259`, model `claude-haiku-4-5-20251001`: after trust confirmation, `SessionStart` increased the live dashboard from `27 projects / 252 sessions` to `28 projects / 253 sessions`; Haiku returned `READY`; `/exit` delivered `SessionEnd` and restored the dashboard to exactly `27 / 252`.
- Removed the temporary project, hook settings, socket, and tmux session. The worktree was clean after the implementation commit; this handoff update is the only new document change now.

**Learned**:

- The T04 fixture path was not sufficient to establish integration confidence; the real CLI lifecycle now proves the interactive synchronous path end to end without retaining raw payloads.
- Claude `--print --no-session-persistence` can cancel a deliberately delayed `SessionEnd` hook at process shutdown (`Hook cancelled`), so interactive idle-then-`/exit` is the reliable observation method for session retention/removal.
- A plain cwd under `./tmp` resolves to the parent repository's project identity; a nested disposable git repository isolates the live test project.
- The dashboard's UI is populated by many existing OpenCode sessions, so the exact count delta, not the visible tile name, was the useful black-box assertion.

**Blockers surfaced**:

- No new code blocker remains after `babf167`; Terra's M3 cross-task sign-off still gates T05 decomposition.
- T05 remains responsible for async successful-turn behavior, startup-gap/foreground discovery, exit-path reliability, subagent identity, broader event coverage, and final staleness policy; this run proves only the real interactive synchronous `SessionStart`/`SessionEnd` lifecycle.

---
