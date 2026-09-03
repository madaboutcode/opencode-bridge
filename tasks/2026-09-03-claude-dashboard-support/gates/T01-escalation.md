# T01 Escalation Brief

## Area

T01 evidence artifacts under
`tasks/spikes/2026-09-03-claude-dashboard-support/`, its gate report, and the
T01 entries in `deferred.md`.

## Calibration

Delivery profile version 1; T01 contract version 2; Review Frame version 2.
Two refine-loop passes completed. No task commit was made because the runner
escalated after pass 2.

## Residual Findings

- **R1 High:** the exact supported event set is not selected; the evidence says
  all documented events are configurable rather than selecting an allowlist.
- **R2 High:** the synchronous lifecycle claim is not reproducible as written;
  the landed probe records only `SessionStart`, while the claimed full sequence
  came from separate runs.
- **R3 Medium:** probes do not capture CLI exit status.
- **R4 Medium:** asynchronous timing lacks a common observer start/end marker,
  so ordering is not established.
- **R5 High, contested:** evidence contains a transcript-path template and an
  `authentication_failed` example value. The reviewer classifies both as
  metadata-only violations; the runner agrees on the transcript path but views
  the error type as permitted metadata, while also agreeing it is cheap to
  remove.

## Evidence and Consequence

The CLI version `2.1.259` is installed. Synchronous unauthenticated probes
observed `SessionStart`, `StopFailure`, and `SessionEnd`; no authenticated
scenario could run because API/OAuth credentials are absent and the real
`~/.claude` is off-limits. The artifacts are reversible, but T01 feeds the
allowlist, lifecycle, staleness, and identity decisions consumed by T02/T03.
R1/R2 and R5 therefore threaten downstream contract correctness and the
metadata-only release bar. R3/R4 are cheap evidence-completeness fixes. The
authenticated lifecycle, startup-gap, async-success, exit-path, and subagent
questions are already recorded as contract-sanctioned deferrals for T05.

## Caller Hypothesis

The loop did not converge because the task combines evidence collection with
decisions that require authenticated runtime behavior unavailable in the
isolated environment. The implementer also overclaimed separate unauthenticated
traces as one lifecycle and retained examples that violate the evidence
redaction rule. The fixable residuals should be corrected now; genuinely
credential-dependent claims should remain deferred without weakening the
authenticated T05 requirement.

## Advisor Decision Requested

For each residual, choose Correct now, Defer with trigger, Preserve foundation,
Reject, or Contract/Decomposition re-cut. Explicitly resolve the R5 contested
disposition and whether T01 may pass after one targeted correction pass. Preserve
the requirement that T05 performs the complete authenticated end-to-end flow.
