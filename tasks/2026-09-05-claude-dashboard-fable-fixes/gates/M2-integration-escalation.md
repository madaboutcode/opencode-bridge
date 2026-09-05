# M2 Integration Escalation

## Finding

The M2 fan-in review found that T04's tool receipt fallback is emitted only as
an event-local projection. When a session has no retained `turn_started`, a
tool event emits `Running { turn_started: receipt }` but leaves all facts empty.
A following tracked `Resume` or `Compact` therefore preserves empty facts and
projects `Idle`, instead of preserving the prior Running snapshot as required
by T04 v3 and the M1 compaction workflow.

## Decision Requested

Adjudicate whether a tool event with no retained start basis must set
`turn_started = receipt` as its authoritative Running basis. This preserves the
observed M1 Running value across Resume/Compact and does not change ordinary
snapshot output; it does change the current test's interpretation that the
fallback is not retained. The alternative is to add another fact for a
fallback Running projection, which is strictly more state for the same fact.

T04 remains blocked pending advisor disposition, implementation correction,
fresh `luna-high` review, and a new fan-in review.

## Advisor Disposition

The advisor chose to set `turn_started = receipt` when a tool event has no
retained basis. This reuses the existing authoritative fact and preserves the
M1 Running projection across Resume/Compact. The alternative of a second
fallback fact and the alternative of weakening preservation were rejected.

T04 contract version 4 / Review Frame v3 records this decision. The
event-local-fallback test must be replaced with retained-basis and
Resume/Compact regression coverage before re-review.
