# claude-dashboard-support - Handoff

> Living document - Section 1 is current truth (rewrite each session), Section 2 is
> append-only history. Maintained via the session-continuity skill.

## Section 1: Current State

### Orientation

This run adds opt-in, metadata-only Claude Code monitoring to the dashboard
without transcript access, session control, or Claude configuration mutation.
M1 evidence/ingress, M2 adapter, and M3 T04 runtime are clean and committed.
T04 is `fd83209`; the next action is Terra's M3 cross-task sign-off before a
separate T05 decomposition.

### Map - read in this order

| Priority | File / section | What to look for |
|----------|----------------|------------------|
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-support/contracts/T04-claude-runtime.md` | Sealed T04 v1 boundary, including the explicitly approved isolated `main.rs` re-scope. |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-t04-runtime.plan.md` | T04 data flow, bounds, test strategy, owns-list, and checkpoints. |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-t04-runtime.design.md` | Candidate B runtime decomposition and component contracts. |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tmp/orchestrator/2026-09-03-claude-dashboard-support/STATE.md` | Current conductor state and next gate. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-support/gates/T01c-report.md` | Durable evidence baseline and four T05 deferrals. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-support/gates/T02-report.md` | User-scoped ingress, privacy, and macOS listener outcome evidence. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-support/gates/T03-report.md` | Typed decoder, adapter, feature-path, and provider-neutral boundary gate. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-03-claude-dashboard-support/deferred.md` | Four credential-dependent T05 promotions; do not promote them in T04. |

### World-Facts & Tooling

- The run branch is `conductor/claude-dashboard-support`; gated commits are T01c `401887e`, T02 `aeb8317`, T03 `e631129`, T04 decomposition `bdb8647`, and T04 runtime `fd83209`.
- T02's targeted ingress test passed 35 tests across three repeat runs; its clippy, format, registry, and diff checks are recorded in `gates/T02-report.md`.
- T03's targeted adapter test passed 8 tests, Claude library tests passed 46, all dashboard targets passed 206, and workspace check/clippy/format passed; details are in `gates/T03-report.md`.
- T04's runtime test passed 19 tests across three runs; ingress passed 35, adapter passed 8, Claude library passed 49, all dashboard targets, clippy, format, workspace check, and diff checks passed. Details are in `gates/T04-report.md`.
- The first T03 Luna request failed with provider HTTP 404 before producing a review; a fresh Luna High request then completed cleanly. The failed request is not a review pass.
- macOS promptly refuses saturated Unix connections; T02 maps that to `ListenerUnavailable` and proves bounded completion rather than claiming Linux blocking behavior.
- DeepSeek Flash is the implementation worker; Terra is the persistent design/milestone advisor; Luna High is the independent gate reviewer; a separate `clerk` spec validator is mandatory for T04's modified `claude.md`.
- Verified commands include `git status --short --branch`, `git diff --check`, `cargo test -p dashboard --test claude_ingress`, `cargo test -p dashboard --test claude_adapter`, `cargo test -p dashboard --lib claude::`, `cargo test -p dashboard --all-targets`, `cargo clippy -p dashboard --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `cargo check --workspace`.

### State & Provenance

- Terra explicitly signed off M1 and M2 and sealed the T04 v1 Review Frame. The T04 frame approves only isolated command-dispatch and listener startup/shutdown additions in `main.rs`; the pre-existing icon-mode dirty hunk must remain verbatim and unstaged.
- T02 owns path resolution, parsing, serialization, delivery deadline, and privacy filtering. T03 owns wire decoding, Claude lifecycle mapping, snapshots, tombstones, and adapter state. T04 must compose those APIs, not duplicate them.
- T04 is the runtime/release wiring phase: `dashboard claude-hook`, user-scoped Unix listener, bounded intake, startup ordering, cleanup, manual docs, and runtime tests. The durable contract owns the complete acceptance boundary.
- T05 alone owns authenticated Claude CLI evidence, full hook-to-dashboard E2E, successful-turn/exit-path/subagent evidence, and final stale-session policy. The four deferrals remain active.
- T04's first Luna review found three runtime defects; DeepSeek corrected them, and fresh Luna High verification returned CLEAN. T04 is committed as `fd83209`.

### Judgment - Recommended Next Moves

- Request Terra's M3 cross-task sign-off for the committed T04 handoff.
- Do not start T05 until its own decomposition, contract, and authenticated boundary are explicitly approved.
- Preserve the four credential-dependent deferrals and do not claim authenticated or complete dashboard behavior from T04 fixtures.

### Dead Ends & Corrections

- T02 v5 initially failed review because it used a shared `/tmp` fallback, did not bind the full delivery deadline over filesystem metadata, had stale spec registries, malformed R15 structure, and lacked busy-listener/log-capture tests. T02 v6 corrected these and is recorded in `gates/T02-report.md`; do not reintroduce the old fallback or requirements.
- T03 Luna's first provider request returned HTTP 404 before review. Retrying with a fresh Luna High session produced the clean verdict; do not count provider failure as a review or treat it as an implementation defect.
- T04 Candidate A (all runtime logic in `main.rs`) and Candidate C (separate binary) were rejected. Candidate B (`claude/command.rs` plus `claude/listener.rs` with narrow main composition) is the sealed choice because it keeps protocol/runtime mechanics testable and preserves `dashboard claude-hook`.
- The existing `main.rs` icon-mode changes are not part of T04. Do not run a formatter that rewrites that dirty hunk, stage it, or use broad staging commands.
- Combined large patches previously failed when one expected state line was stale or one added line lacked the patch prefix. Smaller file-oriented patches succeeded; use that approach for gate bookkeeping.

### Do-Not-Touch

- Never access `~/.claude`, project `.claude`, credentials, or transcript JSONL; T04 is documentation/runtime work only and must remain isolated.
- Do not modify or stage `crates/dashboard/src/claude/hook.rs`, `state.rs`, or `wire.rs`; those are T02/T03 authorities.
- Do not modify provider-neutral `adapter.rs`, `snapshot.rs`, `project_identity.rs`, shell files, OpenCode files, or T01c/T02/T03 evidence/gates.
- Preserve unrelated dirty files, especially `crates/dashboard/src/main.rs`'s icon-mode hunk, mosaic files, `opencode/mod.rs`, and `crates/opencode-client/src/opencode.rs`.
- Do not start T05 or claim authenticated/full-path validation from T04 fixtures.

### Open Items - triaged

**Blocks next phase (resolve first):**

1. Terra's M3 cross-task sign-off is required before T05 decomposition.
2. T05 must retain ownership of authenticated full hook-to-dashboard E2E, final staleness, and the four credential-dependent promotions.

**Resolvable during the work:**

3. Any spec-validator wording failure can be corrected in the owned spec/spec-delta and validated again; no requirement meaning should be guessed.
4. Any platform-specific listener behavior must be documented as observed or deferred, not generalized from macOS.

**UNSPECIFIED (ask, don't guess):**

5. None currently. Future authenticated lifecycle, async, exit-path, subagent, and final staleness questions are explicitly T05-owned rather than unspecified T04 decisions.

### Working Commands

```bash
# Verify branch, unrelated dirt, and staged/unstaged state
git status --short --branch

# Check whitespace without rewriting files
git diff --check

# T04 gate commands required by the sealed contract
cargo test -p dashboard --test claude_runtime
cargo test -p dashboard --test claude_ingress
cargo test -p dashboard --test claude_adapter
cargo test -p dashboard --lib claude::
cargo test -p dashboard --all-targets
cargo clippy -p dashboard --all-targets -- -D warnings
cargo fmt --all -- --check
cargo check --workspace
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
