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

## 2026-09-03 - Real Claude end-to-end validation

Considered: validate the integration only with synthetic hook fixtures or require a real Claude CLI flow.
Chosen: at least one authenticated real Claude CLI scenario must drive the configured hook, helper, Unix socket, Claude adapter, and dashboard event path end-to-end; all Claude configuration must remain in temporary isolated `HOME` and `CLAUDE_CONFIG_DIR` directories.
Why: the user requires validation of the actual integration seam, and fixtures cannot prove Claude's hook invocation contract or runtime wiring.
Limitations: the test must redact content and may record only allowlisted metadata; missing CLI or credentials blocks the evidence gate rather than being papered over with fixtures.
Reversal: none for this run; relaxing this requirement would require explicit user approval.

## 2026-09-03 - End-to-end gate placement

Considered: require the complete Claude-to-dashboard flow in the pre-implementation evidence task, or place it after adapter and runtime wiring exist.
Chosen: T01 must exercise the real Claude CLI and configured hook against an isolated observer/helper; T02 must exercise the real Unix socket and privacy boundary; T05 must exercise the complete authenticated hook -> helper -> Unix socket -> adapter -> dashboard event path after T03 and T04.
Why: Terra found that T01's no-production-source owns-list cannot prove an adapter/event path that later tasks create. The release requirement remains unchanged; only its checkpoint is corrected.
Limitations: T01/T02 cannot claim integrated dashboard validation. T05 remains blocked if a real Claude CLI or credentials are unavailable.
Reversal: reopen the decomposition if T03/T04 cannot expose a deterministic test seam for observing the provider-neutral event path.

## 2026-09-03 - T01 escalation disposition

Considered: ship T01's partial evidence, defer all residuals, or authorize one targeted correction/review pass.
Chosen: Terra authorized one targeted pass. Correct now: R1 exact conservative event allowlist, R2 separate observed traces, R3 CLI exit-status capture, R4 common timing markers, and R5 removal of the transcript-path template and `authentication_failed` value. Keep credential-dependent lifecycle, discovery, async-success, exit-path, and subagent evidence deferred to T05.
Why: R1/R2 affect downstream contract decisions and R5 violates the task's explicit evidence boundary; R3/R4 are cheap completeness fixes. The defects are evidence-record corrections, not a contract or decomposition failure.
Limitations: T01 may close as a blocked-but-complete evidence gate only after the targeted pass. Production exposure remains blocked until T05 provides authenticated integrated evidence.
Reversal: if the authorized pass finds a new above-line issue or cannot close R1-R5 within scope, return to escalation rather than starting T02.

## 2026-09-03 - Final bounded T01 correction

Considered: abandon T01, consume its partial evidence, or authorize one materially bounded final correction with fresh agents.
Chosen: Terra authorized one fresh top-level DeepSeek Flash correction and one fresh Luna High verification. R2 must retain only separately observed traces with no cross-trace ordering claim; R4 must retract unsupported async ordering/viability claims and retire invalid common-marker evidence; R5 must remove all literal prompt text, `rate limit` examples, and CLI stdout/stderr forwarding from every owned probe and artifact. R1/R3 and the four T05 deferrals remain intact.
Why: these are direct violations of T01 v2's closed metadata-only evidence boundary or unsupported evidence claims. A narrow fresh pass changes the failed execution shape without reopening the whole loop.
Limitations: this is the final bounded pass; if its acceptance boundary is not clean, T01 fails and M1 must be re-cut. T05 still owns authenticated lifecycle and complete integrated E2E evidence.
Reversal: do not start T02 if Luna finds any boundary violation; return to Decomposition/M1 re-cut as Terra directed.

## 2026-09-03 - Final correction execution recovery

Considered: repeat the previous long correction prompt, continue the stale CLI wrappers, or change execution shape.
Chosen: terminate stale reviewer wrappers and make one compact, direct top-level DeepSeek edit-only attempt followed by one fresh direct Luna verification. No nested `opencode run` wrapper, no background supervision claim, and no additional open-ended pass.
Why: the prior direct DeepSeek session ended after a preamble with no file changes, while an inherited Luna CLI wrapper remained parked without useful output. The execution failure is operational, not evidence that the bounded corrections are complete.
Limitations: this remains Terra's final bounded correction authorization; if the compact attempt does not produce a clean owns-list, T01 fails and M1 must be re-cut.
Reversal: stop and escalate rather than retrying if this final execution attempt fails or Luna finds any above-line boundary violation.

## 2026-09-03 - T01 failure and M1 re-cut

Considered: accept T01 with its R4 wording, run another T01 correction, or re-cut M1 around the remaining run-level record defect.
Chosen: T01 fails and remains uncommitted. M1 is re-cut with T01b, a reviewed one-file cleanup that removes the unsupported async-before-CLI-exit claim from `deferred.md`, preserves exactly the four credential-dependent T05 deferrals, and re-verifies the complete T01 boundary. T02 depends on T01b.
Why: Luna found R4 still above the line after Terra's final bounded pass. The claim is unsupported by the probe's missing CLI boundary marker and violates the final acceptance boundary; Terra explicitly directed an M1 re-cut rather than another open-ended pass.
Limitations: the authenticated scenario and all other credential-dependent evidence remain deferred to T05; no production scope changes.
Reversal: if T01b cannot close the exact R4 wording without changing the four deferrals or the approved profile, return to Terra for another decomposition decision.

## 2026-09-03 - T01b evidence adoption boundary

Considered: let T01b commit only its one-file deferral correction or adopt the uncommitted corrected T01 evidence artifacts from failed T01.
Chosen: T01b owns the current T01 spike directory and `deferred.md`, edits only `deferred.md`, and commits the spike artifacts unchanged after a clean boundary verification.
Why: T01 failed without a commit, but its corrected evidence must become durable before T02 can consume it. The ownership change preserves the evidence while keeping the re-cut's implementation scope to one wording correction.
Limitations: T01's failed report remains historical; T01b's report is the gate for adopting the evidence. No production code or authenticated claims are added.
Reversal: re-cut again if the adopted spike files differ during T01b or the four deferrals cannot remain intact.

## 2026-09-03 - Runner binding correction

Considered: retain Clerk as a nested-agent runner using `opencode run`, or have the conductor run the review-loop protocol directly.
Chosen: the conductor session is also the runner. DeepSeek Flash remains the implementer and Luna High remains the independent reviewer; no nested `opencode run` process is used for new tasks.
Why: the environment blocks subagent nesting at depth 1, and the CLI workaround made process supervision and status reporting unreliable. Direct top-level launches preserve the intended model bindings and make each process observable.
Limitations: conductor and runner duties share one session, so the conductor must remain artifact-blind on ordinary passed tasks and rely on Luna's independent report; escalation and milestone fit remain the only diff-reading exceptions.
Reversal: restore a separate runner only if the environment provides reliable nested-agent supervision without the CLI workaround.

## 2026-09-03 - T01b failure and T01c re-cut

Considered: accept T01b's adoption, redact all permitted metadata, or re-cut the evidence adoption with explicit provenance.
Chosen: T01b fails because its v2 boundary could not prove unchanged adoption of untracked T01 artifacts and its strict wording classified permitted metadata ambiguously. T01c will normalize only the raw empty-discovery serialization, retain the Claude version as labeled metadata, generate a content-hash manifest for the current spike set plus `deferred.md`, and commit that exact current baseline without claiming historical lineage. T02 depends on T01c.
Why: Terra ruled the `2.1.259` version metadata permitted, `[]` raw serialization must be normalized, and untracked adoption cannot prove “unchanged.” A manifest establishes durable current provenance without pretending to reconstruct failed-T01 history.
Limitations: T01 remains failed and uncommitted; T01c is the final M1 re-cut gate and uses one conductor correction plus one fresh Luna verification. Authenticated behavior and full E2E remain T05 scope.
Reversal: if T01c's manifest, normalization, or four deferrals fail review, stop and re-cut M1 again rather than starting T02.

## 2026-09-03 - T02 dependency refresh

Considered: spawn T02 using its stale T01 dependency or reseal the contract against the T01c adoption gate.
Chosen: T02 is version 2 and depends on committed T01c; it consumes the current hash-verified evidence baseline and four deferrals, while failed T01/T01b artifacts are historical records only.
Why: T01c is the actual durable M1 prerequisite. A stale contract dependency would let T02 consume an uncommitted failed-task state and violate the shared-contract correctness rule.
Limitations: T02's ingress implementation remains independent of adapter/runtime work; T05 still owns authenticated integrated evidence.
Reversal: return to Decomposition if T01c's gate or evidence baseline changes materially.

## 2026-09-03 - T02 executable test seam

Considered: execute T02 with tests only inside an unreferenced `hook.rs`, or add a Cargo integration test that imports the hook module.
Chosen: T02 v3 owns `crates/dashboard/tests/claude_ingress.rs` in addition to `hook.rs` and the Claude spec. The integration test must compile the hook module and execute real Unix-socket tests before T03 adds the library module wiring.
Why: tests inside an unreferenced source file would not run under Cargo, weakening the real-socket and privacy acceptance gate.
Limitations: T02 still does not own `lib.rs`, `mod.rs`, or runtime wiring; T05 remains the complete authenticated integration gate.
Reversal: return to Decomposition if the integration test cannot import the ingress module without changing the shared/runtime boundary.

## 2026-09-03 - T02 spec-tree convention update

Considered: create `docs/specs/dashboard/claude.md` while leaving the project convention at five files, or update the convention as part of T02.
Chosen: T02 v4 owns the required `docs/specs/CLAUDE.md` update registering `claude.md` as the sixth dashboard spec file. The ingress integration test and Claude spec remain T02-owned; adapter/runtime files remain out of scope.
Why: the feature plan explicitly requires a Claude-specific spec, while the current convention says five files and “no more.” Creating it without updating the convention would knowingly violate project ground truth.
Limitations: this is a documentation-convention expansion, not permission to create additional spec files; T02 must retain the one-scenario-per-requirement and consumer-lens rules.
Reversal: return to Decomposition if the convention update reveals an existing spec-tree conflict that cannot be resolved without changing unrelated specs.

## 2026-09-03 - T02 spec-index update

Considered: leave `docs/specs/README.md` with the old five-file index or include the index in T02's convention correction.
Chosen: T02 v5 owns `docs/specs/README.md` and must register `dashboard/claude.md` in both the prose index and the file-map table, alongside the existing `docs/specs/CLAUDE.md` update.
Why: the index is part of the documented spec-tree entry point and currently contradicts the sixth-file convention; leaving it stale would make the feature undiscoverable and the project documentation internally inconsistent.
Limitations: T02 still creates only one new spec file and does not alter unrelated requirements or implementation files.
Reversal: return to Decomposition if the index exposes another unrelated documentation conflict.

## 2026-09-03 - T02 overview registry ownership

Considered: leave the stale five-file registry in `docs/specs/dashboard/overview.md` outside T02, or include it in the spec-tree correction.
Chosen: T02 v6 owns `overview.md` in addition to the convention and root index; its one bounded correction must register `claude.md` consistently in all three registries.
Why: Terra ruled `overview.md` is a spec-tree registry, not unrelated documentation. Leaving it stale would make the six-file convention internally contradictory.
Limitations: the added ownership is documentation-only; no adapter, library, runtime, global Claude configuration, or transcript work enters T02.
Reversal: re-cut again if the three registries cannot be made consistent without changing unrelated requirements.
