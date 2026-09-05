# M2 Design Review Checkpoint

**Status:** resolved; T04/T05 contracts sealed at version 2.

The independent design review found the decomposition sound but identified
contract-level gaps that must be resolved before implementation:

- T04 needs an explicit event-to-fact transition matrix. In particular,
  matching `PermissionDenied`, `ElicitationResult`, `PostToolUse`, and
  `PostToolUseFailure` must clear the active needs-you fact; unmatched ids must
  preserve it. User prompts, running tool events, Notifications, Stop,
  StopFailure, subagent Stop, and all SessionStart sources need explicit
  set/clear/preserve behavior.
- T04 must state the precedence and lifecycle of `turn_ended` so it cannot
  permanently pin a session in `NeedsYou` or erase the retained turn start.
- T05 needs a compatibility matrix for all fifteen variants, exact JSON tags
  and optional omission, newline and size-bound behavior, malformed fields,
  unknown kinds/versions, ignored extras, and duplicate/typed edge cases.
- T05 must keep raw JSON transient and map every serde failure to the existing
  category-only error behavior; the kind preflight boundary must be explicit.
- Both contracts must state that the serde model does not bypass hook-side R14
  extraction, truncation, and required-field validation.

**Disposition:** contract ambiguity, not an implementation failure. Revise the
M2 plan/design and draft contracts with these matrices before advisor sealing.
The user-selected bindings remain `luna-high` reviewer for T04 and fresh
`luna` reviewer for T05.

**Source:** independent `luna` software-design review and `luna-high` advisor
calibration on 2026-09-05.

## Resolution

The contracts were amended with the requested event/fact transition matrix,
matching-clear fallback, unmatched pending distinction, duplicate-key
last-wins behavior, mandatory raw-kind preflight, and the existing serde_json
128-versus-129 recursion boundary. `luna-high` then sealed both contracts and
authored their Review Frames. No source code changed during decomposition.

## Advisor Amendment

The fresh advisor adjudicated two implementation-discovered contract errors.
T04 now adds a private `idle_since` fact so the structural rewrite preserves
M1 Idle timestamps on no-op and Resume/Compact paths without storing
`AttentionState`. T05 now documents the locked serde_json behavior observed by
the pass-2 tests: 127 nested containers are accepted and 128 are rejected as
category-only `Malformed`. The parser policy is unchanged.

Both contracts are version 3 with Review Frame v2 and remain pending fresh
bound implementation review before M2 sign-off.

## Final Disposition

T04 was subsequently amended to version 4 / Review Frame v3 for retained tool
fallback bases. T04 and T05 final bound reviews passed, and the M2 fan-in review
passed across commits `b88e56e`, `74ecc97`, and `71841e5`. The advisor signed
off M2; the stale `claude/DESIGN.md` state-model paragraph is recorded as a
bounded documentation deferral.
