# T04 Contract Escalation

## Finding

The fresh `luna-high` T04 review found that the sealed contract requires both:

- no separately mutable `AttentionState`, with projection `Idle { last_update: r }`
  whenever no `turn_ended` or `turn_started` fact exists; and
- byte-for-byte preservation of an existing `Idle` attention value on tracked
  `Resume`/`Compact`, unmatched permission/elicitation results, and no-op
  notifications.

The original M1 implementation retained the previous `AttentionState`, so its
`Idle` timestamp was preserved on those paths. The three sealed T04 facts cannot
represent that timestamp: `turn_started` would project `Running`,
`turn_ended` would project `NeedsYou`, and `pending_tool_use_id` is unrelated.

## Review Evidence

The reviewer also found incomplete executable coverage for tracked `Fork`,
`Clear`, absent-source reset, `Resume`, `Compact`, unmatched
`ElicitationResult`/`PostToolUse`, `PostToolUseFailure` pending paths, and the
top-level-pending/unrelated-subagent-tool cases. Full dashboard tests and
clippy pass, but T04 conformance is not established.

## Decision Requested

Choose whether to amend T04 contract/design v2 to add an `idle_since` fact that
is updated only when the projected state enters Idle and preserved on no-op
paths, or to explicitly revise the byte-for-byte M1 preservation requirement
and its acceptance tests. The first option preserves shipped behavior and the
selected structural goal without storing a public `AttentionState`; it needs
an advisor/user-approved contract amendment before implementation.

No T04 corrective implementation or commit is authorized until this decision
is recorded. T05 remains independently in pass-2 rework.

## Advisor Disposition

The fresh advisor chose the behavior-preserving option: amend T04 with a
private `idle_since` fact, retain the M1 byte-for-byte preservation rule, and
expand the executable matrix coverage. The alternative of recomputing an Idle
timestamp from each new receipt was rejected as a behavior regression.

T04 contract version 3 and Review Frame v2 now record this disposition. The
implementation remains blocked until the amended matrix tests are added and a
fresh bound `luna-high` review re-seals the task.
