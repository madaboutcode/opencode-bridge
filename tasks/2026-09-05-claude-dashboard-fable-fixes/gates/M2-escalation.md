# M2 Execution Escalation

## Area

M2 reviewed tasks T04 (turn-fact attention projection) and T05 (serde wire
conversion) on branch `claude-dashboard-fable-fixes`. Both contracts are
version 2, advisor-sealed, with T04 reviewer `luna-high` and T05 reviewer fresh
`luna`.

## Calibration

Delivery profile version 2. T04/T05 contract version 2, Review Frame v1.
Neither task began an implementation pass. No product disposition is being
contested.

## Residual Finding

The runner topology cannot start its required implementer and independent
reviewer because nested subagent spawning is unavailable at depth 1. T04's
runner stopped without code or verification. T05's runner also tried an
`opencode` CLI fallback, which rejected the existing config's unsupported
`subagent_depth` and `permissions` keys; it did not modify configuration.

## Views

- Runner/conductor view: no task can pass under the sealed runner prompt until
  the topology is changed or the session is resumed from a shallower context.
  A local review must not be substituted for T04's user-selected `luna-high`
  reviewer or T05's fresh reviewer.
- Contract view: no ambiguity or product defect was found. Both contracts are
  ready and their ownership surfaces are disjoint.
- Evidence view: T04 and T05 reports each show zero implementation passes and
  no commits. Baseline `cargo test -p dashboard` and clippy are green only for
  the unchanged M1 base and do not establish M2 conformance.

## Pass Count

Zero refine-loop passes for both tasks. No implementation agent started.

## Hypothesis

The failure is execution topology, not task scope: this conductor session is
already at the maximum nested-agent depth, so a runner cannot satisfy the
runner/implementer/reviewer role chain. The independent-review requirement is
still material for T04 and T05 and must survive any workaround.

## Decision Requested

Authorize either a resumed shallower conductor session, or a direct sequential
topology in which the conductor launches the implementer and then a fresh
reviewer while preserving the sealed contracts, writes the gate report, and
commits only after independent review. Do not weaken the Review Frames or
claim the baseline test run as task conformance.

## Resolution

The user authorized the conductor to act as the runner from this session.
Because nested spawning is unavailable, implementation and review will be
directly sequenced from the conductor: `luna` implementer first, then a fresh
independent reviewer (`luna-high` for T04, `luna` for T05). The conductor owns
the gate reports and task commits after the reviewer returns. The sealed
contracts and Review Frames remain unchanged, and no unsupported config keys
will be edited.
