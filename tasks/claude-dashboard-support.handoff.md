# claude-dashboard-support - Handoff

> Living document - Section 1 is current truth (rewrite each session), Section 2 is
> append-only history. Maintained via the session-continuity skill.

## Section 1: Current State

### Orientation

This run adds opt-in, metadata-only Claude Code monitoring to the dashboard
without transcript access, session control, or Claude configuration
mutation. T01c/T02/T03/T04 are committed, and M3 cross-task sign-off
(T02+T03+T04 plus three post-gate commits) is unconditionally closed. T05 —
the authenticated release-verification gate — is designed and sealed at
contract v2, not yet implemented. Six task/decision files are modified or
new on disk, uncommitted; no source code changed this session.

### Map - read in this order

| Priority | File / section | What to look for |
|----------|----------------|------------------|
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-support/contracts/T05-claude-release-verification.md` | Sealed v2 contract: `state.rs` owned outright for staleness/subagent identity; five other sealed files are conditional-access-for-defects-only; the failure branch; S6 closure-by-citation (unverified — see Open Items). |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-support/decisions.md` | Read from "2026-09-04 - Post-gate `04a7cf5` disposition" to the end — the M3 close-out, the two run-wide procedural rules, and the T05 seal are all there, each with Considered/Chosen/Why/Limitations/Reversal. |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-support/deferred.md` | S1/S2/S3/S4-S5 evidence deferrals T05 must close, now with 2026-09-04 evidence-status notes and two live findings folded in. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-support/gates/T04-report.md` | Has a 2026-09-04 post-gate correction note: the original "binds before adapters" claim was false pre-`babf167`. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-support/PLAN.md` | Status and Decomposition sections updated to reflect M3 close and T05 seal. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/crates/dashboard/src/claude/{command.rs,listener.rs,hook.rs,wire.rs,state.rs,mod.rs}` | Unchanged this session; `hook.rs`/`wire.rs` carry the `04a7cf5` overflow fix from before this session. |

### World-Facts & Tooling

- The opencode bridge (`mcp__opencode-bridge__*`, used for Terra/DeepSeek/Luna in prior sessions) has no Anthropic/Opus model in its catalog — only DeepSeek/GPT-5.6-family/GLM/Kimi/Qwen/MiniMax/etc. The `terra` agent tag there maps to `openai/gpt-5.6-terra`, not Opus. A request for "an Opus advisor on a different harness" cannot be satisfied by both clauses at once; asked the user, who chose Claude Opus via the Agent tool (same harness as the conductor, not the opencode bridge) over the two genuinely-separate-harness options.
- The `advisor` agent spawned this session has no memory of prior sessions' Terra and does not claim to be Terra; it works entirely from `decisions.md` (its memory) and the documents it's pointed at. Cross-session notifications from it arrive truncated at roughly 1400-1800 words; asking it to "send the rest" reliably continues from the cut point — this happened three times this session with no loss of content once reassembled.
- A `coder` agent delegated to add a spec-delta.md entry reported having done so; `git status`/`git diff --stat` showed the file untouched. Its factual investigation (the R15 chars-vs-bytes determination) was independently re-verified and correct — only the "I edited the file" claim was false. Caught by checking directly, not by trusting the completion report.
- `04a7cf5` fixed a real crash: a `cwd`/`session_id` built from quote/backslash characters at its byte-length bound escapes to roughly double length in JSON, pushing the serialized envelope past `MAX_ENVELOPE_BYTES` (8 KiB) despite passing individual field bounds, and used to hit an `assert!` panic in `to_wire()`/`serialize_envelope()`. Fixed by returning `Result` and adding `DropReason::OversizedEnvelope`/`DeliveryOutcome::EnvelopeTooLarge`. `MAX_HOOK_INPUT_BYTES` and `decode_envelope`'s `OutOfBounds` triggers are untouched — verified directly via `git diff fd83209 04a7cf5 -- hook.rs`.
- `feature_hook_subprocess_drops_escaped_envelope_overflow` (`crates/dashboard/tests/claude_runtime.rs:348`) already proves the real `dashboard claude-hook` subprocess exits 0 with no stdout on an oversized envelope. `DeliveryOutcome::EnvelopeTooLarge` is structurally unreachable from that path — `parse_hook_input` catches the oversized case via `DropReason::OversizedEnvelope` before `deliver()` is ever called — so no new test was needed for the M3 helper-level-coverage condition. Re-ran it directly: `test feature_hook_subprocess_drops_escaped_envelope_overflow ... ok`.
- The R15 spec wording "128/4096 characters" was always wrong; `hook.rs`'s `valid_session_id`/`valid_cwd` used `value.len()` (Rust `str::len()` = UTF-8 byte length, never char count) unchanged from the T04 gate commit `fd83209` through `04a7cf5` — confirmed via direct diff, not agent report. Purely editorial; recorded in `spec-delta.md`'s new "POST-T04" section.
- Test-count delta (35->38 ingress, 19->20 runtime) fully accounted: all four new tests trace to `04a7cf5` (`escaped_envelope_overflow_drops_before_any_ipc`, `identity_bounds_are_measured_in_utf8_bytes`, `escaped_envelope_overflow_is_dropped_without_serialization_panic`, `feature_hook_subprocess_drops_escaped_envelope_overflow`); grep-verified current counts match.
- `bd35c5b` verified directly via `git show --stat`/`--name-only`: touched `crates/dashboard/src/main.rs` (23 lines, the pre-existing icon-mode hunk), `mosaic/*.rs`, `opencode/mod.rs`, `opencode-client/src/opencode.rs`, `.gitignore`, and 70+ non-source docs/tmp/plan files. Never touched `crates/dashboard/src/claude/` or `docs/specs/dashboard/`.

### State & Provenance

- M3 is unconditionally signed by the `advisor` agent (standing in for Terra). Gated commits: T01c `401887e`, T02 `aeb8317`, T03 `e631129`, T04 decomposition `bdb8647`, T04 runtime `fd83209`. Post-gate commits disclosed and accepted on this sign-off: `04a7cf5`, `bd35c5b`, `babf167`. All four T05 evidence deferrals (S1-S5 numbering per `deferred.md`) remain open with promotion triggers unmet; the live run only partially informs S2.
- Two run-wide procedural rules now apply from 2026-09-04 forward, not just to T05: (1) any change to a sealed file outside its owning task's active gate needs a decision entry naming the contract touched, the rule it fixes against, and a test or stated reason none is needed — before or alongside the change; (2) every gate closure names its artifact and the exact confirming command, not a self-report alone.
- `contracts/T05-claude-release-verification.md` is sealed at v2 by the advisor, who pasted its own Review Frame text into the file per its explicit "no re-review needed" instruction. `state.rs` is owned outright by T05 for staleness-policy and subagent-identity logic only (T05's planned deliverable from T01's original deferral, not a defect discovery). `hook.rs`/`wire.rs`/`listener.rs`/`command.rs`/`mod.rs` are conditional access, bounded to fixing a defect against a rule those files' sealed contracts already state — any rule change is a contract amendment requiring the advisor's approval before implementation, not a decision entry after. T05 has an explicit failure branch: if evidence goes the wrong way (async hooks unreliable, no defensible staleness policy), that blocks release and gets recorded, not engineered around.
- The seal is conditional on one unverified point: S6 (socket/IPC) is closed in the contract "by citation" to T02's and T04's gate artifacts, on the advisor's belief that those gates substantively settled it — but no document explicitly confirms this. If reviewing those artifacts during T05 finds a real gap, that's a decomposition question back to the advisor, not something T05 resolves unilaterally.
- Implementation has not started against the T05 contract.

### Judgment - Recommended Next Moves

- Before T05's first implementation pass, explicitly verify the S6 closure-by-citation against `gates/T02-report.md` and `gates/T04-report.md` — this is the one item in the sealed contract the advisor itself flagged as unverified rather than settled.
- Run T05's evidence-gathering the same way the M3 live validation worked: tmux, temporary hook settings, isolated `HOME`/`CLAUDE_CONFIG_DIR` by default, with a fresh decision entry any time real credentials are used opaquely instead (the Session 3 credential-isolation amendment is scoped to that one run, not inherited automatically).
- Apply the two new run-wide rules literally on T05: disclose any sealed-file touch as it happens, and verify any delegated agent's "I edited X" claim against `git status`/`git diff` directly before trusting it — this session caught exactly one false claim of that kind.
- Ask before committing. The uncommitted state (`decisions.md`, `deferred.md`, `spec-delta.md`, `gates/T04-report.md`, `PLAN.md`, new `contracts/T05-claude-release-verification.md`) was never confirmed for commit this session — the user moved from "want me to commit?" straight to "do T05 decomposition and close this session" without answering. Do not assume yes.

### Dead Ends & Corrections

- T02 v5 initially failed review because it used a shared `/tmp` fallback, did not bind the full delivery deadline over filesystem metadata, had stale spec registries, malformed R15 structure, and lacked busy-listener/log-capture tests. T02 v6 corrected these; do not reintroduce the old fallback or requirements.
- T03 Luna's first provider request returned HTTP 404 before review; a fresh retry produced the clean verdict. Provider failure before output is not a review result.
- T04 Candidate A (all runtime logic in `main.rs`) and Candidate C (separate binary) were rejected; Candidate B (narrow `main.rs` composition plus `claude/command.rs`/`claude/listener.rs`) is sealed.
- Direct normal startup before `babf167` panicked because `UnixListener::bind` requires an entered Tokio runtime; fixed by `rt.enter()` around the bind, not a listener-ownership change.
- A delayed synchronous `SessionEnd` hook in `--print` mode was canceled by Claude at process shutdown (`Hook cancelled`) — a probe limitation, not evidence the interactive path removes sessions incorrectly. Use interactive idle-then-`/exit` instead.
- A plain cwd under `./tmp` resolves to the parent `opencode-mcp` git root, not its own project identity; a nested disposable git repository is needed for isolation. Not owned by any S-item or spec-delta; recorded as a standalone finding in `deferred.md`.
- A coder agent this session falsely reported editing `spec-delta.md`; the underlying investigation was correct, the "I did it" claim wasn't. Caught by `git status`, not by trusting the report.
- T05 contract v1 was not sealed on first submission: the advisor found a `state.rs` ownership contradiction, a missing kind-bound on conditional sealed-file access, no failure branch, and an unaccounted-for S6. All four were fixed in v2, which is sealed. Producing a contract is not the same as it being sound without an adversarial read.

### Do-Not-Touch

- Never inspect `~/.claude`, project `.claude`, credentials, or transcript JSONL. A user-requested black-box Claude invocation may use its own opaque auth/config internally, but the test harness must not read or retain those artifacts.
- `hook.rs`, `wire.rs`, `listener.rs`, `command.rs`, `mod.rs` are no longer flatly off-limits — T05's sealed contract grants conditional access, bounded to fixing a defect against a rule those files' contracts already state. Any broader change is a contract amendment requiring the advisor's prior approval, not a T05-owned decision.
- `state.rs` is owned by T05 outright, but only for staleness-policy and subagent-identity logic — no other change.
- Do not modify provider-neutral `adapter.rs`, `snapshot.rs`, `project_identity.rs`, shell files, OpenCode files, or prior T01c/T02/T03/T04 evidence/gates outside the specific post-gate corrections already made.
- Do not commit generated nested `target/` output or machine-local runtime artifacts; the root `.gitignore` covers nested Rust targets.

### Open Items - triaged

**Blocks next phase (resolve first):**

1. Verify S6's closure-by-citation against `gates/T02-report.md`/`gates/T04-report.md` before or during T05's first pass; if it's not actually closed, escalate to the advisor rather than writing new behavior into a sealed file to close it.

**Resolvable during the work:**

2. Convert each T05 evidence area into `T05-evidence/` records without copying raw hook payloads or transcript data.
3. Any platform-specific listener/socket behavior must be documented as observed or deferred, not generalized from macOS.

**UNSPECIFIED (ask, don't guess):**

4. Whether to commit the current uncommitted state (six files: `decisions.md`, `deferred.md`, `spec-delta.md`, `gates/T04-report.md`, `PLAN.md`, new `contracts/T05-claude-release-verification.md`) — asked once, not yet answered.
5. Whether the product should require synchronous hooks for reliable `SessionEnd`, or explicitly define an async-hook/staleness policy, is T05's S1/S4 evidence question to resolve, not something to guess ahead of the evidence.

### Working Commands

```bash
# Verify branch, unrelated dirt, and staged/unstaged state
git status --short --branch

# Full workspace tests, including the Claude runtime and ingress paths
cargo test --workspace

# Workspace quality gates
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo build -p dashboard

# Confirm the helper-level overflow test still passes
cargo test -p dashboard --test claude_runtime feature_hook_subprocess_drops_escaped_envelope_overflow -- --nocapture

# Runtime/tool availability used for live black-box checks
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

### Session 4 - 2026-09-04

**Phase**: M3 cross-task sign-off closure -> T05 decomposition and seal

**Work done**:

- Spawned an `advisor` agent (Claude Opus, Agent tool) to stand in for Terra and render M3 cross-task sign-off, after surfacing that the user's request ("Opus" + "different coding harness") named two incompatible things — the opencode bridge harness has no Opus model. User chose Claude Opus via the Agent tool.
- Advisor initially withheld M3: three post-gate commits (`04a7cf5`, `bd35c5b`, `babf167`) existed in git history but not in `decisions.md`, the run's authoritative log.
- Investigated all three with delegated `coder`/`clerk` agents plus direct verification: `04a7cf5` fixed a real envelope-overflow crash without contract-rule changes but touched sealed files undisclosed; `bd35c5b` never touched Claude/spec files; the live run's `HOME`/`CLAUDE_CONFIG_DIR` relaxation got explicit user approval. Wrote three decision entries and reordered the log chronologically.
- Advisor signed M3 conditionally on six items; closed all six (helper-level test coverage confirmed already existing, R15 spec wording confirmed editorial, test-count delta reconciled, two gate/deferral doc corrections). Caught and corrected one false "I edited this file" claim from a delegated coder agent by checking `git status` directly.
- Advisor signed M3 unconditionally and asked for two run-wide procedural rules (post-gate sealed-file disclosure; gate-closure artifact verification) plus a built-binary-only acceptance criterion, to be carried into T05. Recorded both rules as a run-wide decision entry.
- Drafted `contracts/T05-claude-release-verification.md` v1 and sent it for the advisor's decomposition Review Frame. Advisor withheld seal: found a `state.rs` ownership contradiction, missing kind-bound on conditional sealed-file access, no failure branch, and an unaccounted-for S6 evidence item. Applied all four corrections plus one S5 wording fix, bumped to v2, pasted the advisor's exact Review Frame text — sealed, no re-review required.
- Updated `PLAN.md`'s Status and Decomposition sections to reflect the M3 close and T05 seal.

**Learned**:

- The opencode bridge harness used for Terra/DeepSeek/Luna carries no Anthropic models; "terra" there is `openai/gpt-5.6-terra`. A genuinely separate-harness Opus advisor is not possible in this project's current tooling.
- Cross-session agent messages truncate around 1400-1800 words; asking for "the rest" reliably continues them with no content loss once reassembled — happened three times this session.
- Delegated agent self-reports of file edits are not reliable without a direct check; caught one false claim this session by running `git status` instead of trusting the completion message.
- Producing a decomposition contract is not the same as it being sound — the advisor found four substantive gaps in T05 v1 that a less adversarial read would have missed, including one (S6) that isn't fully closed even now, just cited as probably-closed.

**Blockers surfaced**:

- None blocking T05's start. One open verification task carried into T05 itself: confirm S6 is actually closed by the cited T02/T04 gate artifacts, not just assumed so.
- Six files (five modified, one new) are uncommitted; commit was asked about once and not yet answered by the user.

---

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
