# T01 Escalation Brief - Second Review

## Area

T01 evidence artifacts under
`tasks/spikes/2026-09-03-claude-dashboard-support/` and the final gate report.

## Calibration and History

Delivery profile version 1; T01 contract version 2; Review Frame version 2.
The original refine-loop used two passes and escalated. Terra authorized one
targeted correction/review pass. DeepSeek completed the correction and a direct
Luna High review verified only R1-R5. The targeted review is not clean; no T01
commit was made.

## Current Verdict

- **R1: Pass.** EVIDENCE.md now states a conservative observed allowlist of
  `SessionStart`, `StopFailure`, and `SessionEnd`, with other events unverified.
- **R2: Fail, High.** EVIDENCE.md lines 20-23 still claim all three lifecycle
  events were observed with synchronous hooks, contradicting the separated
  synchronous/async traces at lines 37-45. The combined lifecycle remains
  overclaimed.
- **R3: Pass.** The five probe scripts capture CLI exit status with
  `PIPESTATUS[0]` and the evidence records the observed status.
- **R4: Fail, High.** The async claim relies on `test-async-behavior.sh`, which
  lacks common CLI boundary markers. The only common-marker probe,
  `test-comprehensive.sh`, generates invalid JSON because its sync and async
  settings contain trailing commas. Its timing evidence cannot establish the
  claimed ordering.
- **R5: Fail, High.** Every probe still contains literal prompt text
  `claude --print "Hello"`; EVIDENCE.md retains the error example `rate limit`;
  and the probes emit up to five lines of CLI stdout/stderr through
  `2>&1 | head -5`. These violate the metadata-only evidence boundary.

## Below-Line Notes

Luna also found that the version attribution names a script that does not run
`claude --version`, the schema cites unidentified previous runs, and the old
`authentication_failed` wording remains only in the historical escalation
record. No transcript path value was found in current spike files, and global
Claude configuration, project `.claude`, transcripts, and credentials were not
accessed.

## Evidence and Consequence

The targeted pass did not close three high-severity contract defects. R2 can
contaminate the event allowlist/lifecycle decisions consumed by T02/T03. R4
leaves async behavior unsupported by the claimed test. R5 directly violates
the approved metadata-only posture and the explicit T01 prohibition on storing
prompt/output content. All are in-scope, reversible evidence corrections, but
the one authorized correction pass is exhausted.

## Caller Hypothesis

The first correction focused on the previously identified lines but did not
perform a complete owns-list sweep or validate every modified probe as valid
JSON. The remaining failures are therefore incomplete correction and
verification, not a new product-scope uncertainty. Repeating the same open
loop would risk another partial cleanup; any further work needs a narrower
brief with a fresh implementer/reviewer execution or an explicit conductor-led
artifact correction decision.

## Advisor Decision Requested

Decide whether to authorize a second, narrowly scoped correction with a fresh
DeepSeek/Luna top-level execution, direct conductor correction followed by
Luna verification, or a contract/decomposition change. Do not permit T02 to
consume the contradictory or leaking evidence. Preserve the four existing
credential-dependent T05 deferrals and the authenticated integrated E2E gate.
