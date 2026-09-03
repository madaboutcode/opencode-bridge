# Implementation Plan: Conductor Review Calibration

## Purpose

Give Conductor runs a durable, Advisor-authored delivery posture and a compact
task-local Review Frame so reviewed work prioritizes release-critical behavior
without turning speculative hardening into blockers. Keep Refine Loop generic and
preserve independent reviewer discovery and dissent.

## File Tree

```text
$HOME/.claude/skills/conductor/
  SKILL.md
  templates/delivery-profile.md (new)
  templates/advisor-brief.md
  templates/contract.md
  templates/runner-brief.md
  templates/report.md
  stages/scoping.md
  stages/decomposition.md
  stages/execution.md
  stages/escalation.md
  stages/milestone.md
$HOME/.claude/skills/refine-loop/
  SKILL.md
```

## Orchestration

```text
Conductor supplies facts -> Advisor drafts delivery profile -> user approves
-> stable reviewed-task contract -> Advisor authors <=90-word Review Frame
-> runner/implementer/reviewer read profile + contract/frame
-> report preserves disposition and dissent
-> Advisor adjudicates escalations/milestones against approved posture + evidence
```

## Boundaries

IN SCOPE:
- Add a per-run delivery-profile template and lifecycle.
- Add the compact Review Frame to reviewed task contracts.
- Propagate both artifacts through Advisor, runner, review, escalation, and milestone paths.
- Strengthen report/deferred disposition evidence.
- Remove Refine Loop's global initial-development calibration.

OUT OF SCOPE:
- No changes to the generic Advisor persona.
- No second known-issues file.
- No exact domain-specific review checklists.
- No automated tooling or scripts for validating profiles.

MUST NOT CHANGE:
- Refine Loop's two-pass cap and persistent per-task agents.
- Conductor's report-blind gate and commit protocol.
- Standalone Refine Loop's caller-selected depth.

MUST FOLLOW:
- Common policy is referenced by path, not pasted into spawn prompts.
- Review Frames prime disposition and depth, never suspected findings.
- Review Frames are at most 90 words and may collapse to one line.
- Material delivery-posture expansion remains user-approved.
- Reviewer dissent survives triage.

## Pre-Decisions

Decision: Common-context location
Choice: One self-contained per-run `delivery-profile.md` instantiated from a reusable template.
Rationale: One stable role-facing source survives skill evolution and session resume.

Decision: Task-local artifact
Choice: A Review Frame section in the existing contract, not a new file.
Rationale: It is a small task-specific delta and must remain coupled to contract version.

Decision: Advisor invocation frequency
Choice: Every reviewed task receives a frame; opportunistic batching must not delay ready tasks.
Rationale: Uniformity avoids another error-prone “which task needs calibration?” decision.

## Testing Strategy

- Feature verification: trace a reviewed task through scoping, decomposition, runner spawn,
  report, escalation, milestone, and Advisor resume; every role must receive the profile/frame.
- Search for contradictory global `80%`/`happy path` defaults in Refine Loop.
- Check Review Frame schema for the 90-word cap, no-hypothesis boundary, and contest path.
- Check existing commit and two-pass protocols remain present.

## Verification Checkpoints

| After | Verify By | Fail Action |
|---|---|---|
| Templates updated | Read templates as one contract chain | Fix missing references before stages |
| Stages updated | Trace the full run lifecycle | Fix any implicit handoff |
| Refine Loop updated | Search calibration and pass-cap language | Restore generic behavior/protocol |
| Full change | Cross-file grep and diff review | Resolve contradictions before completion |

## Acceptance Criteria

- [x] Advisor drafts the run profile from supplied facts; user approves material posture.
- [x] Advisor receives approved profile on resume, with decisions and deferrals; its initial spawn drafts the profile from supplied facts.
- [x] Every reviewed task has an Advisor-authored Review Frame of at most 90 words.
- [x] Review Frame uses Context, Expectations, and Depth and cannot prime suspected defects.
- [x] Runner, implementer, reviewer, escalation, and milestone paths reference common calibration.
- [x] Refine Loop is caller-calibrated and remains usable for exhaustive standalone review.
- [x] Reports preserve profile/frame challenges and contested disposition.
- [x] Existing two-pass, report-blind, and commit protocols remain intact.

## Tasks

### Task 1: Add calibration artifacts
- **Files:** conductor templates and `SKILL.md`
- **Depends on:** none
- **Verify:** profile and Review Frame contracts are internally coherent

### Task 2: Wire Conductor lifecycle
- **Files:** conductor stage files
- **Depends on:** Task 1
- **Verify:** lifecycle trace reaches every role and resume path

### Task 3: Neutralize Refine Loop calibration
- **Files:** refine-loop `SKILL.md`
- **Depends on:** Task 1
- **Verify:** embedded caller calibration and standalone depth both remain explicit

### Task 4: Integrity review
- **Files:** all changed skill files
- **Depends on:** Tasks 1-3
- **Verify:** searches and diff satisfy all acceptance criteria
