# T01b - Correct deferred async evidence wording

**Contract version** - 2

**Context** - goal: repair the remaining unsupported R4 statement in T01's
  run-level deferral record and verify that the final T01 evidence boundary is
  coherent; who uses it: T02/T03 implementers and the T05 release gate; scale:
  one selected-user workstation and the four existing T05 deferrals;
  criticality: high because downstream lifecycle decisions must not consume an
  unsupported timing claim, while the correction is a small reversible record
  change.

**Delivery profile** - `tasks/2026-09-03-claude-dashboard-support/delivery-profile.md` version 1; task override: none.

**Boundaries** - owns: the current T01 evidence directory
  `tasks/spikes/2026-09-03-claude-dashboard-support/` and
  `tasks/2026-09-03-claude-dashboard-support/deferred.md`; the spike files are
  adopted unchanged and only `deferred.md` may be edited. Must not touch the
  T01 contract, T01 gate report, other gate reports, production source,
  `PLAN.md`, `decisions.md`, user Claude configuration, project `.claude`,
  transcript files, or unrelated dirty worktree files.

**Conventions** - preserve exactly four T01 deferral entries and each entry's
  scenario, consequence, deferral assumption, and promotion trigger. Replace
  only unsupported claims that async hooks did not execute before CLI exit with
  wording that says async timing/order was not measured and successful-session
  async viability remains indeterminate and deferred to T05. The record must
  not claim authenticated evidence, cross-trace ordering, or any raw prompt,
  assistant/CLI output, transcript path/value, error value, secret, or arbitrary
  payload. Do not access global or project Claude configuration or transcripts.

**Skills to read and apply** - `debugging`.

**Acceptance - done when** - `deferred.md` contains exactly the four existing
  T01 credential-dependent T05 deferrals, with their promotion triggers intact;
  the async deferral makes no CLI-relative ordering claim; no other deferral
  claims unsupported lifecycle/timing evidence; and a complete text sweep
  confirms the adopted T01 spike plus `deferred.md` contains no prohibited
  sensitive values or raw output examples. The adopted spike files are staged
  unchanged so the corrected T01 evidence is durable in the T01b commit; only
  `deferred.md` may differ from its pre-task content.

**Gate** - report-only (refine-loop).

**Dependencies** - T01 failed; this re-cut consumes its current artifacts.

## Review Frame

**As of** - contract version 2

**Context** - One-file wording repair with unchanged adoption of corrected evidence; high consequence because it unlocks downstream consumption.

**Expectations** - Make adopted spikes durable only with a clean boundary. Preserve four T05 deferrals and remove the CLI-relative async claim; introduce no runtime or production claim.

**Depth** - Deep review of adoption provenance and durability, plus surgical deferred-wording review; exclude lifecycle reinvestigation and runtime design.
