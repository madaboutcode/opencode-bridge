# T01c - Adopt and normalize current T01 evidence

**Contract version** - 1

**Context** - goal: make the current reviewed T01 evidence durable while
  removing the one raw empty-discovery serialization that Luna flagged; who
  uses it: T02/T03 implementers and the T05 release gate; scale: one
  selected-user workstation and the current nine-file evidence set; criticality:
  high because downstream decisions need a trustworthy, metadata-only evidence
  baseline, while the correction is bounded and reversible.

**Delivery profile** - `tasks/2026-09-03-claude-dashboard-support/delivery-profile.md` version 1; task override: Terra-approved conductor correction plus one fresh Luna verification; no new DeepSeek correction pass.

**Boundaries** - owns: the current T01 spike set under
  `tasks/spikes/2026-09-03-claude-dashboard-support/`,
  `tasks/2026-09-03-claude-dashboard-support/deferred.md`, and the new
  `tasks/spikes/2026-09-03-claude-dashboard-support/adoption-manifest.sha256`;
  the conductor may edit only the normalized lines in `EVIDENCE.md` and create
  the manifest, while all other adopted files must remain byte-for-byte as
  observed at task start. Must not touch T01/T01b contracts or historical gate
  reports, production source, `PLAN.md`, `decisions.md`, user Claude
  configuration, project `.claude`, transcript files, or unrelated dirty
  worktree files.

**Conventions** - the manifest lists the eight current spike files plus
  `deferred.md`, excluding the manifest itself, with SHA-256 hashes and stable
  repository-relative paths. It establishes T01c's current baseline; it must
  not claim to prove the files were unchanged during failed T01/T01b. Preserve
  exactly four T01 deferrals and their promotion triggers. Retain
  `2.1.259` only as explicitly labeled normalized version metadata. Replace the
  literal empty discovery serialization `[]` with a structured statement that
  no sessions were discovered. No raw CLI output, prompt/assistant text,
  transcript path/value, error-value example, secret, or arbitrary payload may
  remain in current evidence. Do not access global or project Claude
  configuration or transcript JSONL.

**Skills to read and apply** - `debugging`.

**Acceptance - done when** - the two targeted `EVIDENCE.md` normalizations are
  present; the existing eight spike files and four deferrals otherwise remain
  intact; `adoption-manifest.sha256` hashes exactly those eight spike files plus
  `deferred.md` and verifies successfully; the manifest and evidence make no
  historical unchanged-lineage claim; the complete current spike-plus-deferred
  sweep contains no raw CLI output or prohibited sensitive values; and no
  global/project Claude configuration or transcript is accessed. The
  authenticated scenario remains blocked and T05's integrated E2E requirement
  remains unchanged.

**Gate** - report-only (direct Luna verification; conductor writes the report).

**Dependencies** - T01b failed; consumes its current artifacts.

## Review Frame

**As of** - contract version 1

**Context** - Final M1 adoption gate converts reviewed current evidence into a durable baseline without asserting failed-task lineage.

**Expectations** - Limit changes to two evidence normalizations and manifest creation; preserve four T05 deferrals, isolation, and metadata-only posture. Introduce no runtime, authenticated, or integration claim.

**Depth** - Deep review of manifest-to-commit identity and narrow normalization; exclude evidence recollection and product design.
