# Claude Dashboard Fable Fixes — Handoff

> Living document — Section 1 is current truth (rewrite each session); Section 2 is
> append-only history. Maintained via the session-continuity skill.

## Section 1: Current State

### Orientation
M1 behavior fixes and M2 structural rewrites for the Claude dashboard are complete on
branch `claude-dashboard-fable-fixes`. M2 is advisor-signed off as FIT PASS. The next
separate phase is live Claude hook proof, after the bounded documentation follow-up.

### Map — read in this order
| Priority | File / section | What to look for |
|----------|----------------|------------------|
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tmp/orchestrator/2026-09-05-claude-dashboard-fable-fixes/STATE.md` | Durable current ledger, commits, residuals, and next phase. |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-05-claude-dashboard-fable-fixes/gates/M2-review.md` | M2 task commits, final verification, advisor sign-off, and residual disposition. |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-05-claude-dashboard-fable-fixes/decisions.md` | Append-only decisions, including `idle_since`, retained tool fallback, and M2 sign-off. |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-05-claude-dashboard-fable-fixes/deferred.md` | Active deferred behavior/documentation items and promotion triggers. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-05-claude-dashboard-fable-fixes/contracts/T04-turn-facts.md` | T04 v4 contract and Review Frame v3. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-05-claude-dashboard-fable-fixes/contracts/T05-serde-wire.md` | T05 v3 contract and Review Frame v2; actual recursion boundary is 127/128. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-05-claude-dashboard-fable-fixes/gates/T04-report.md` | T04 failed/conditional passes, corrections, and final PASS evidence. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-05-claude-dashboard-fable-fixes/gates/T05-report.md` | T05 failed/conditional passes, corrections, and final PASS evidence. |

### World-Facts & Tooling
- `cargo test -p dashboard` — final fan-in verification passed 376 tests across unit and integration suites.
- `cargo clippy -p dashboard --all-targets -- -D warnings` — passed; `cargo fmt --all -- --check` and scoped `git diff --check` also passed.
- Nested agent spawning failed at depth 1 with `Subagent depth limit reached (1)`; the user explicitly authorized direct runner mode, so implementers and fresh reviewers were launched directly and sequentially as required.
- The original persistent advisor was not reachable after session reset; fresh judgment-only `luna-high` advisors were used for the T04/T05 contract escalations and final M2 sign-off.
- Locked serde_json behavior accepts 127 nested containers and rejects 128; parser configuration was not changed.

### State & Provenance
- Final implementation commits: `b88e56e` (T05), `74ecc97` (T04), `71841e5` (T04 fallback follow-up).
- Run records and M2 sign-off commits: `2325f3a`, `bfde84c`.
- User-authorized topology decision and advisor contract adjudications are recorded in `decisions.md`; do not re-litigate them without new evidence.
- M2 fan-in passed across all three implementation commits. The advisor accepted stale `claude/DESIGN.md` wording as bounded documentation drift, not a T04 reopen.
- User constraint remains: live proof must use only isolated `claude --settings <scratch-path>` configuration; never touch the real `.claude/settings.json`.

### Judgment — Recommended Next Moves
- Correct the stale state-model paragraph in `crates/dashboard/src/claude/DESIGN.md` before or alongside live proof; it was deliberately outside the passed T04 ownership surface.
- Treat live hook registration and real-session proof as a new scoped phase with its own plan/review; do not expand M2 retroactively.
- Read `delivery-profile.md` and `deferred.md` before changing scope or promoting a residual.

### Dead Ends & Corrections
- Nested runner/implementer/reviewer spawning was unavailable at depth 1; direct runner mode was explicitly authorized instead of weakening independent review.
- The initial T04 three-fact design could not preserve an existing Idle timestamp; corrected by adding private `idle_since`, not by weakening M1 behavior or storing public `AttentionState`.
- A first tool receipt was initially projection-only and was lost across Resume/Compact; corrected by retaining it in `turn_started`.
- T05 initially retained a dead hand-written serializer and lacked edge-case tests; removed the mapper and added duplicate-key, recursion, optional-combination, null, and full-envelope equality coverage.
- The T05 contract originally claimed 128/129 recursion; corrected to the locked dependency's 127/128 behavior without changing parser policy.

### Do-Not-Touch
- Unrelated dirty worktree changes in mosaic, shell, `opencode-client`, and related docs/examples are user work; do not stage, revert, or fold them into follow-up commits.
- Do not touch the real `.claude/settings.json` during live proof.
- Do not reopen committed T04/T05 implementation unless a new test or live proof contradicts the signed-off contracts.

### Open Items — triaged
**Blocks next phase (resolve first):**
1. Correct `crates/dashboard/src/claude/DESIGN.md`'s stale stored-`attention` description before or alongside live proof.

**Resolvable during the work:**
2. Existing deferred `current_action` turn-end clearing, broad legacy spec-rubric cleanup, and the Claude module's “two sub-types” wording remain bounded in `deferred.md`.

**UNSPECIFIED (ask, don't guess):**
None.

### Working Commands
```bash
# Run the dashboard unit/integration suite
cargo test -p dashboard

# Run dashboard lint gates
cargo clippy -p dashboard --all-targets -- -D warnings

# Verify formatting
cargo fmt --all -- --check

# Verify whitespace on scoped changes
git diff --check
```

---

## Section 2: Session Log

### Session 1 — 2026-09-05

**Phase**: M2 structural rewrites → advisor-signed-off FIT PASS

**Work done**:
- Resolved the execution-topology escalation by user-authorized direct runner mode.
- Implemented and independently reviewed T04 turn facts/projection and T05 serde wire conversion.
- Corrected two contract mismatches through fresh advisor adjudication: `idle_since` for M1 Idle preservation and 127/128 serde_json recursion behavior.
- Added follow-up fixes for retained tool fallback across Resume/Compact and exhaustive wire optional/null/full-envelope evidence.
- Ran M2 fan-in review, committed implementation and documentation artifacts, and preserved unrelated worktree changes.

**Learned**:
- A projection-only fallback is insufficient when a lifecycle event promises prior projection preservation; the authoritative fact must retain the basis.
- Test coverage claims must include omission, explicit null, typed equality, and the complete envelope, not only variant-level event values.

**Blockers surfaced**:
- Live Claude hook proof remains intentionally deferred.
- Stale `claude/DESIGN.md` state-model wording is a bounded documentation follow-up.

---

<!-- NEXT SESSION: Append below this line -->
