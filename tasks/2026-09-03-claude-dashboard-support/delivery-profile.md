# Delivery Profile - claude-dashboard-support

## Status

**User-approved** · version: 1 · approved: 2026-09-03 by user

## Supplied Facts

- The dashboard already has a provider-neutral adapter, snapshot, tombstone,
  and canonical project-identity boundary for OpenCode.
- This run adds Claude Code as a second opt-in, read-only harness.
- Ingress is a user-configured Claude command hook. It forwards a strict,
  bounded metadata envelope to a user-scoped Unix socket.
- Prompt and assistant text, tool arguments/results, transcript paths/files,
  secrets, and unknown fields must not cross the hook or adapter boundary.
- Only live hook-observed sessions are in scope. No transcript scanning,
  history replay, persistent event journal, automatic configuration writes, or
  Claude session control is permitted.
- Existing OpenCode behavior and unrelated dirty worktree changes are outside
  this run and must remain untouched.
- S1-S7 must produce isolated evidence for hook payloads, lifecycle,
  discoverability/startup gap, staleness, identity/subagents, IPC failures,
  and privacy before production implementation.
- The run has three milestones: evidence and ingress contract (M1), adapter
  (M2), and runtime/release verification (M3). The run branch and commit
  policy are named in `PLAN.md`.

## Proposed Delivery Posture

- Deliver a local-only, manually configured, opt-in Claude monitoring
  capability alongside unchanged OpenCode monitoring.
- Treat the first release as a guarded capability: it is inactive unless a
  user explicitly installs documented hooks, and it has no discovery or replay
  promise for sessions that were not observed by those hooks.
- Support it for selected users as a guarded experimental capability, not as a
  generally supported feature, and never enable it by default. M3 must record
  passing evidence for every acceptance criterion before exposure.
- The release claim is metadata-only observability of live sessions. It makes
  no claim of complete Claude-session inventory, transcript visibility,
  durable history, or delivery during dashboard downtime.
- Any failure of the privacy boundary, non-blocking hook behavior, stale-session
  policy, or OpenCode regression gate blocks exposure in runtime startup. The
  rollback is to disable/remove the Claude adapter and hook subcommand without
  changing user Claude settings.

## Evidence-Gated Decisions Before Release

- S1 selects the supported Claude version floor, final event allowlist, and
  asynchronous-hook viability.
- S2 selects evidence-backed lifecycle and attention mappings.
- S3 confirms the stated startup-gap limitation or requires a separately
  approved scope change.
- S4 selects a bounded stale-session policy and its user-visible treatment.
- S5 decides whether subagents have stable representable parent identity.
- S6 selects socket location, permissions, bounds, framing/version handling,
  and exact best-effort helper behavior.
- S7 approves the final exposure and logging allowlists through negative
  privacy evidence.

## Approved User Decision

Approved: **an experimental/guarded, manual opt-in capability for selected
users, with no completeness guarantee for sessions missed while the dashboard
is unavailable.**
