# Spec Delta — claude-dashboard-fable-fixes

## MODIFIED

- **R13** (`docs/specs/dashboard/claude.md`): the Notification paragraph now
  states the adapter mapping for `idle_prompt`, `permission_prompt`, and
  `agent_needs_input`, and states that absent or unrecognized subtypes leave
  existing attention unchanged. Added a co-located Notification scenario.
  reason: the M1 fresh-eyes review found the prior statement contradicted the
  shipped attention behavior; advisor adjudication classified T01's spec
  exclusion as a decomposition flaw and required this milestone correction.

- **Claude hook adapter section** (`docs/specs/dashboard/client.md`): replaced
  the stale three-event/T01c description and empty-field claims with a
  cross-reference to the fifteen-event R13 matrix and bounded R14-R15 fields;
  retained the SessionEnd tombstone contract. reason: advisor adjudication
  found a release-critical cross-file documented-truth contradiction during
  the M1 correction review.
