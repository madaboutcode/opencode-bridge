# PLAN - claude-dashboard-support

## Status

Scoping signed off by Terra on 2026-09-03; M1 task gates are closed through committed T02; milestone sign-off is pending.
Source plan: `tasks/2026-09-03-claude-dashboard-support.plan.md`.

## Boundaries

This run adds Claude Code as an opt-in, read-only harness observed by the
dashboard. It accepts user-configured Claude lifecycle hooks through a local
Unix socket, strictly allowlists metadata, maps live lifecycle state into the
existing provider-neutral snapshot/tombstone boundary, and documents manual
configuration and removal.

The run must preserve existing OpenCode behavior, must not read Claude
transcripts, must not control Claude sessions, and must not write Claude
configuration or persistent monitoring state.

## Out Of Scope

- Transcript reading, tailing, parsing, summarization, or historical discovery.
- Session control, prompt submission, attach/resume, interrupt, or mutation.
- Automatic Claude hook installation or any write to `~/.claude` or project
  Claude settings.
- Public network monitoring, remote endpoints, and a persistent event journal.
- Treating `claude agents --json` as the authoritative foreground session set.
- Session zoom or transcript trace views.

## Project Ground Truth

- `CONTRIBUTING.md` requires lean dependencies, stderr-only logs, no disk state,
  and cargo build/test/clippy/format checks.
- Dashboard contracts live under `docs/specs/dashboard/`; the existing adapter
  boundary is in `crates/dashboard/src/adapter.rs` and snapshots in
  `crates/dashboard/src/snapshot.rs`.
- The source plan contains the evidence backlog, lifecycle decisions, privacy
  boundary, acceptance criteria, and expected file areas. Contracts extract
  only task-relevant rules from it.
- All Claude experiments use isolated temporary `HOME` and
  `CLAUDE_CONFIG_DIR`; the real Claude configuration remains untouched.
- Release verification must include at least one real Claude CLI flow through
  the configured hook, helper, Unix socket, adapter, and dashboard event path.
  Fixtures alone cannot satisfy end-to-end validation. Missing credentials or a
  missing CLI is a blocked evidence gate, not permission to claim validation.
- T01 proves real Claude hook invocation into an isolated observer/helper, T02
  proves the real Unix-socket ingress, and T05 proves the complete integrated
  hook-to-dashboard path after T03/T04 exist.

## Milestones

- **M1 - evidence and ingress contract:** close S1-S7, choose the supported
  lifecycle set and bounded stale policy, define the allowlisted IPC seam, and
  write the Claude-specific privacy/configuration spec.
- **M2 - adapter:** implement Claude lifecycle state, project identity,
  provider-neutral snapshots, tombstones, and unit/IPC feature tests.
- **M3 - runtime and release verification:** wire opt-in startup and the hook
  subcommand, document installation/removal/validation, then run isolated CLI,
  workspace, clippy, format, privacy, and rollback gates.

## Run Config - Roles

| Role | Binding | Responsibility |
|---|---|---|
| advisor | `terra`, one persistent session | Delivery profile, Review Frames, scoping/milestone sign-off, escalations |
| runner | this conductor session | Runs the review-loop protocol directly, records gate reports, performs bookkeeping and mechanical verification |
| implementer | `deepseek-flash` | Evidence work and production implementation |
| reviewer | `luna-high` | Independent refine-loop review, adversarial QA/privacy coverage, no self-fixing |
| conductor | this session | Judgment, contracts, stage transitions, bookkeeping, commits, milestone fit |

Luna High is never the implementer. The reviewer may write tests that expose
  defects, but must not weaken or repair those tests. The conductor/runner does
  not replace Luna's independent review; it only coordinates the passes and
  records the gate.

## Git Policy

- Run branch: `conductor/claude-dashboard-support`.
- The branch was created from the committed dashboard state on
  `conductor/opencode-dashboard`, not `main`, because this feature depends on
  that completed dashboard and the current worktree already contains unrelated
  dirty changes. Those changes are carried untouched and are not owned by any
  task in this run.
- Reviewed tasks are committed by the runner at loop-pass: contract owns-list,
  gate report, and `deferred.md` only; never `git add -A` or `git add -u`.
- Bare tasks and milestone artifacts are committed by the conductor at their
  gates. No force-push, history rewrite, or direct changes to `main`.
- Any pre-existing dirty file is outside task owns-lists unless explicitly
  re-scoped and approved before a task starts.

## Decomposition

### M1 - evidence and ingress contract

- T01: isolated Claude lifecycle and identity evidence (S1-S5). Reviewed;
  failed after its final bounded review; no commit.
- T01b: adopt corrected T01 evidence unchanged, correct the unsupported async
  timing statement in the four-entry T01 deferral record, and re-verify the
  complete T01 evidence boundary. Reviewed; depends on T01 and is the M1 re-cut
  gate.
- T01c: normalize the remaining raw empty-discovery representation, establish
  the current evidence set as an explicit content-hash baseline, and adopt it
  durably without claiming historical unchanged lineage. Reviewed; depends on
  T01b and is the final M1 re-cut gate. The conductor performs the one bounded
  correction; Luna performs the one verification.
- T02: local IPC, privacy boundary, and Claude-specific specification (S6-S7).
  Reviewed; failed its first review; v6 correction verified clean and committed
  as `aeb8317`.
- T03: Claude adapter/state integration. Design, implementation plan, and
  contract v1 sealed by Terra; ready for one implementation pass and one fresh
  Luna verification. Depends on T01c and T02; T04 remains blocked until T03 is
  clean and committed.

Planned later tasks, to be decomposed after each milestone review:

- T03: Claude adapter and feature verification through IPC.
- T04: runtime wiring, hook command, and user documentation.
- T05: release regression, rollback verification, and the authenticated real
  Claude CLI flow through hook, helper, Unix socket, adapter, and dashboard
  event path.
