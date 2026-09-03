# T01 - Isolated Claude hook and lifecycle evidence

**Contract version** - 2

**Context** - goal: close the real Claude evidence gates S1-S5 and produce the
  facts the ingress and adapter contracts depend on; who uses it: the T02/T03
  implementers and reviewers; scale: one selected-user workstation and a small
  number of disposable sessions; criticality: high because unsupported hook or
  identity assumptions contaminate every downstream task, but all spike output
  is reversible evidence.

**Delivery profile** - `tasks/2026-09-03-claude-dashboard-support/delivery-profile.md` version 1; task override: none.

**Boundaries** - owns: `tasks/spikes/2026-09-03-claude-dashboard-support/` and
  only its temporary evidence fixtures/scripts; must not touch production source,
  `docs/specs/dashboard/claude.md`, the source plan, any unrelated dirty file,
  the real `~/.claude`, project `.claude` settings, or persisted Claude
  transcripts.

**Conventions** - every Claude invocation uses fresh temporary `HOME` and
  `CLAUDE_CONFIG_DIR`, with a disposable settings fixture installed only there;
  do not read, write, or assert against global Claude configuration or
  transcript JSONL. Record event names, redacted field presence, opaque IDs,
  tool labels, notification types, timestamps, exit status, version, and timing
  only. Do not store prompt text, assistant text, tool input/output, transcript
  paths, secrets, or arbitrary payloads. A real CLI run is required; synthetic
  fixtures may supplement it but cannot replace it.

**Skills to read and apply** - `debugging`.

**Acceptance - done when** - `EVIDENCE.md` and supporting redacted schemas are
  present under the owned spike directory and record explicit decisions for:

- S1: supported Claude version floor, exact supported event set, payload field
  presence, and synchronous versus asynchronous command-hook behavior.
- S2: ordered traces for a successful real turn, tool activity, permission wait,
  failure/stop, and user exit, including a negative assertion that rejected
  content fields never enter the observer record.
- S3: foreground/background/startup-gap behavior and the limitations of
  `claude agents --json`.
- S4: ordinary exit, interrupt, terminal close, crash/sleep evidence where
  safely testable, plus a bounded stale-session recommendation.
- S5: parent/session identity, CWD changes, repository root/subfolder/symlink/
  worktree behavior, and whether subagent parent identity is representable.

At least one authenticated real Claude CLI scenario must invoke the actual
configured hook command against an isolated observer/helper while all
configuration remains isolated. This task does not claim the later adapter or
dashboard event path; T05 owns that integrated release gate. If the real CLI
scenario cannot run, stop with a blocked evidence report; do not claim
completion using fixtures alone. No raw sensitive value may appear in evidence.

**Gate** - report-only (refine-loop).

**Dependencies** - none.

## Review Frame

**As of** - contract version 2

**Context** - Evidence-only prerequisite for downstream contracts; high consequence, reversible output.

**Expectations** - Treat completion as a release-blocking decision record: authenticated isolated CLI evidence or an explicit blocked report. Preserve the metadata-only posture and T05 integration boundary.

**Depth** - Deep review of evidence sufficiency, decision traceability, and boundary discipline; no production design review.
