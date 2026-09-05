# T04 Gate Report

**Status:** blocked / escalated

The required T04 loop could not start because nested subagent spawning is
unavailable at the current session depth (`Subagent depth limit reached (1)`).
Both required roles were therefore unavailable:

- implementer: `luna`
- fresh independent reviewer: `luna-high`

Per the T04 execution contract, no local review was substituted for the
independent reviewer. No implementation changes were made, no verification
gates were run, and no commit was created.

**Conformance:** not evaluated.

**Passes used:** none.

**Deferred:** none. This is an execution blocker, not a product deferral.

**Residuals:** T04 implementation, independent review, `cargo test -p
dashboard`, and `cargo clippy -p dashboard --all-targets` remain outstanding.

**Escalation:** rerun T04 from a session where nested `luna` and fresh
`luna-high` subagents can be spawned.

## Direct Runner Pass

The user-authorized direct runner completed the T04 implementation and
launched a fresh independent `luna-high` review. The implementation removed
the separately stored `AttentionState`, added the turn-fact projection, and
updated only the intended state-model comments and state tests. No commit was
created.

**Review verdict:** FAIL.

The reviewer found a P1 representation contradiction: the sealed fact set and
projection rule cannot preserve an existing `Idle` timestamp on tracked
`Resume`/`Compact`, unmatched permission/elicitation results, and no-op
notifications, although the M1 contract requires those paths to preserve the
prior attention byte-for-byte. The reviewer also found missing executable
coverage for several sealed matrix rows. Full dashboard tests and clippy
passed, but they do not establish T04 conformance.

The decision is escalated in `gates/T04-contract-escalation.md`. No T04
commit is permitted until the contract/design tension is resolved and the
missing matrix tests are added.

## Advisor-Amended Direct Runner Passes

The advisor adjudicated the representation conflict in favor of preserving M1
behavior. T04 contract version 3 / Review Frame v2 adds a private
`idle_since` fact, which preserves existing Idle timestamps on no-op and
Resume/Compact paths without storing a mutable public `AttentionState`.

T04 pass 2 implemented that fact and added the missing SessionStart, unmatched
pending, subagent, and Idle-preservation coverage. Pass 3 added the absent
`PostToolUseFailure` and observable pending-retention assertions identified by
the conditional review. No production logic changed in pass 3.

**Final independent reviewer:** fresh `luna-high` — PASS, no findings.

**Verification:**

- focused state tests: 49 passed
- adapter integration tests: 8 passed
- `cargo test -p dashboard`: 376 passed
- `cargo clippy -p dashboard --all-targets`: clean
- `git diff --check`: clean

**Conformance:** yes. T04 is ready to commit.

## Fan-In Follow-Up

The first M2 fan-in review found that an event-local tool receipt fallback was
lost across tracked `Resume`/`Compact`, causing a first-tool Running snapshot
to become Idle. The advisor rejected both an extra fallback fact and weakening
M1 preservation, and chose retaining the receipt in the existing
`turn_started` fact. T04 contract version 4 / Review Frame v3 records that
amendment.

Pass 4 retained the receipt for `PreToolUse`, `PostToolUse`, and
`PostToolUseFailure`, replaced the contradictory non-retention test, and added
Resume/Compact preservation coverage.

**Follow-up independent reviewer:** fresh `luna-high` — PASS, no findings.

**Verification:**

- focused state tests: 49 passed
- focused adapter tests: 3 passed
- `cargo test -p dashboard`: 376 passed
- `cargo clippy -p dashboard --all-targets`: clean
- `cargo fmt --all -- --check`: passed
- `git diff --check`: passed

**Conformance:** yes. T04 follow-up is ready to commit; M2 fan-in must be
rerun before milestone sign-off.
