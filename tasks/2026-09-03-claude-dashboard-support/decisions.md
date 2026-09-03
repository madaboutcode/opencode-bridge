# Decisions - claude-dashboard-support

Per-run append-only decision log. Source decisions remain in the feature plan
unless this run records a new or amended judgment.

## 2026-09-03 - Role binding

Considered: separate QA role versus reviewer-owned QA; separate runner model versus a mechanical runner.
Chosen: Terra is the persistent advisor; DeepSeek Flash implements; Luna High reviews and authors adversarial coverage; Clerk runs the refine loop and mechanical commands; the conductor owns judgment and bookkeeping.
Why: the user explicitly rejected a separate QA role, while the run still needs an independent refine-loop runner distinct from the implementer.
Limitations: Clerk is a procedural runner, not a second design authority; Luna's review must include privacy, IPC, and acceptance-test coverage.
Reversal: change the topology if the runner cannot execute the refine-loop or if review reports repeatedly miss release-critical boundary defects.

## 2026-09-03 - Delivery posture

Considered: internal-only, selected-user experimental support, or generally supported release.
Chosen: selected users receive a guarded experimental, local-only, manually configured, opt-in, metadata-only live-monitoring capability; no completeness guarantee exists during dashboard downtime.
Why: this matches the source plan's evidence-first posture while avoiding an unsupported general-release claim before lifecycle, privacy, IPC, and regression evidence exists.
Limitations: the exact Claude version floor, event allowlist, staleness policy, and subagent behavior remain evidence-gated by S1-S7.
Reversal: reopen if the evidence requires a material change to supported workflows, privacy posture, or support commitment.

## 2026-09-03 - Scoping sign-off

Considered: begin decomposition now or hold for more lifecycle/version/IPC evidence.
Chosen: begin M1 decomposition; S1-S7 remain evidence-gated tasks, not blockers to starting the run.
Why: Terra confirmed all seven definition-of-ready items: boundaries, exclusions, project ground truth, milestones, role bindings, branch/git policy, and user-approved profile.
Limitations: production exposure remains blocked until the evidence and release gates pass.
Reversal: return to scoping if M1 evidence forces a material change to boundaries, supported workflows, or delivery posture.
