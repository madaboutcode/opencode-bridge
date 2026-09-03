# T01b Escalation Brief

## Area

T01b adoption of the uncommitted T01 evidence directory and its one-file
correction to `tasks/2026-09-03-claude-dashboard-support/deferred.md`.

## Calibration and History

Delivery profile version 1; T01b contract version 2; Review Frame version 2.
T01 failed its final bounded review and remained uncommitted. T01b's DeepSeek
correction edited only the S1 async deferral wording. The one fresh Luna review
then checked the T01b boundary and returned `not clean`; no T01b commit exists.

## Residual Findings

- **Privacy/output boundary:** Luna found literal CLI output in current T01
  evidence at `EVIDENCE.md:18` (`2.1.259`) and `EVIDENCE.md:65` (`[]`). No
  prompt, assistant text, transcript path value, error value, secret, or
  arbitrary payload was found. T01b's contract prohibits retained raw CLI
  output, while the original T01 evidence contract permits version, exit
  status, and metadata. The final Terra acceptance boundary also named
  assistant/CLI output broadly, so the treatment of these two allowed metadata
  values needs judgment.
- **Adoption provenance:** all eight spike files are untracked and have no git
  diff baseline. Luna cannot confirm from the repository record that they were
  staged unchanged for adoption, although no spike file was modified during
  T01b and DeepSeek reported that the files were unchanged.

## Evidence and Consequence

R1 and R3 remain clean. Isolation remains clean: temporary `HOME`,
`CLAUDE_CONFIG_DIR`, disposable CWD, no credentials, no global/project Claude
configuration, and no transcript access. The current blockers concern only the
T01b acceptance boundary and durability proof. T02 must not consume T01
evidence until both are resolved or explicitly reclassified.

## Caller Hypothesis

The re-cut exposed two specification/provenance ambiguities rather than a new
runtime defect: the final privacy wording may unintentionally classify
allowlisted version/empty-discovery metadata as raw CLI output, and adopting a
failed task's untracked artifacts lacks a pre-task snapshot. The former may
need a precise allowed-metadata exception; the latter may need a conductor
record or a new adoption task boundary rather than another content cleanup.

## Advisor Decision Requested

Decide whether the two literal values are permitted metadata or must be
redacted, and how to establish durable provenance for the untracked T01 spike
artifacts without violating the no-commit-on-failed-T01 history. Choose a
minimal correction/re-cut or declare T01/M1 unable to proceed. Do not authorize
T02 until the final evidence boundary and adoption durability are explicit.
