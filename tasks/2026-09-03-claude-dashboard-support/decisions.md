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

## 2026-09-03 - T02 v6 gate closure

Considered: advance directly to T03 after the T02 implementation report, or require the sealed v6 correction and independent review.
Chosen: T02 is clean and committed as `aeb8317` after one bounded DeepSeek correction and one fresh Luna verification. M1 now pauses for cross-task milestone sign-off before T03 decomposition.
Why: Luna verified the user-scoped socket, end-to-end delivery deadline, three six-file registries, R15 structure, real busy-listener outcome, captured-log privacy, and preservation of T01c/T05 boundaries.
Limitations: macOS promptly refuses saturated Unix connections, so the test proves exact `ListenerUnavailable` mapping and bounded completion; Linux blocking behavior remains platform runtime coverage. Authenticated full-path evidence remains T05.
Reversal: return to M1 escalation if milestone review finds a cross-task mismatch; do not spawn T03 before sign-off.

## 2026-09-03 - T03 adapter decomposition

Considered: put wire decoding, lifecycle state, and adapter orchestration in one module; let T04's listener own lifecycle mapping; or split the Claude module into wire, state, and adapter layers.
Chosen: T03 uses `wire.rs` for versioned envelope decoding, `state.rs` for pure metadata-only transitions and snapshots, and `mod.rs` for the `HarnessAdapter` channel loop. It registers the module in `lib.rs`, proves the real socket-to-adapter feature path, and updates directly affected adapter/overview documentation.
Why: this keeps Claude protocol volatility and lifecycle policy out of the provider-neutral core and prevents T04 runtime wiring from knowing event semantics. The split is the smallest boundary that gives pure state tests and a narrow listener integration point.
Limitations: only T01c's three observed events are supported; five-minute staleness, successful-turn events, async viability, startup gaps, exit paths, and subagent identity remain provisional/deferred to T05.
Reversal: return to Decomposition if the typed channel or decoder requires changes to shared snapshots, the T02 hook, or runtime startup.

## 2026-09-03 - T03 decomposition sign-off

Terra reviewed the T03 design, implementation plan, and contract and signed off Candidate B. T03 is released for one DeepSeek implementation pass and one fresh Luna verification. Its boundary is adapter registration, typed wire decoding, pure Claude state transitions, feature tests, and directly affected documentation; T04 owns listener/startup and T05 owns authenticated E2E and final staleness policy.

## 2026-09-03 - T03 gate closure

T03 is clean after one bounded DeepSeek implementation pass and one fresh Luna verification. The gate covers the typed v1 decoder, three observed lifecycle mappings, provider-neutral snapshots/tombstone, project identity fallback, real socket-to-adapter feature path, and the T04/T05 boundaries. The first Luna request failed before review with HTTP 404 and is not counted. Commit T03 with its implementation, tests, directly affected docs, and gate report; keep T04 blocked until the commit is verified.

## 2026-09-03 - T04 runtime decomposition

Considered: put listener/command logic in `main.rs`, add a separate binary, or isolate the runtime pieces in `claude/command.rs` and `claude/listener.rs` with narrow main composition. Chosen: Candidate B, preserving the existing `dashboard claude-hook` command and keeping T02 parsing/delivery plus T03 decoding/channel boundaries authoritative. The listener binds before adapters, bounds each local connection, and fails closed without affecting OpenCode. Limitation: `main.rs` already has unrelated dirty icon-mode changes; Terra must explicitly approve only isolated T04 hunks before implementation. Reversal: re-cut if startup wiring cannot be added without touching that hunk or shared/runtime boundaries.

## 2026-09-03 - T04 decomposition sign-off

Terra reviewed the T04 design, implementation plan, and contract and sealed the v1 Review Frame. The narrow `main.rs` re-scope is approved for isolated command-dispatch and startup/shutdown composition additions only; the pre-existing icon-mode hunk remains verbatim and unstaged. T04 is released for one DeepSeek implementation pass, mandatory separate spec validation, and one fresh Luna verification. T05 retains authenticated E2E and final staleness ownership.

## 2026-09-03 - T04 spec-validation baseline disposition

The independent T04 spec validator found T04-modification validity clean and identified five strict-rubric exceptions that predate T04: four intentional adapter-internal contract items in `client.md` R6.4-R6.6/boundary prose, and the stale "five-file map" reference in `client.md` plus identical references in non-owned `layout.md`, `visuals.md`, and `interactions.md`. T04 corrects its owned `client.md` reference and records the change in `spec-delta.md`; it does not expand scope into the three non-owned sibling specs or rewrite the established adapter contract. The remaining baseline exceptions must be disclosed to Luna and the gate report, not called T04 regressions.

## 2026-09-04 - Post-gate `04a7cf5` disposition: recorded acceptance

Considered: reopen T02's gate for a full correction/review pass, or record
acceptance of `04a7cf5` ("Fix Claude envelope overflow handling") as an
in-bounds fix.
Chosen: recorded acceptance, not a reopened gate. `04a7cf5` replaced an
`assert!` panic in `ClaudeIpcEnvelope::to_wire()`/`serialize_envelope()` with
a `Result`-returning drop path (`DropReason::OversizedEnvelope`,
`DeliveryOutcome::EnvelopeTooLarge`). The panic was reachable: a `cwd` or
`session_id` built from quote/backslash characters at its byte bound escapes
to roughly double length in JSON, pushing the serialized envelope past
`MAX_ENVELOPE_BYTES` (8 KiB) despite passing the individual field bounds —
a crash of the hook helper process on a crafted value, reproduced by the new
test `escaped_envelope_overflow_is_dropped_without_serialization_panic`.
Why: T02's contract already required oversized values be dropped without
affecting Claude; the panic violated that existing rule rather than the fix
changing it. No rule value moved — 8 KiB holds, `MAX_HOOK_INPUT_BYTES` and
`decode_envelope`'s `OutOfBounds` triggers are untouched. The new public
surface (two enum variants, a `Result`-wrap on two functions) is
compiler-enforced blast radius inside the ingress boundary, not a widened
contract.
Limitations: this was a real procedural break — every prior touch to
`hook.rs` got a decision entry, a contract version bump, and a fresh review;
this one got none, and went undisclosed until this entry. Three conditions
remain open before T05 decomposition: (a) a helper-level test proving
`EnvelopeTooLarge` exits 0 with no stdout — not yet proven at the
`dashboard claude-hook` command boundary; (b) confirm the R15 spec change
("128 characters" -> "128 UTF-8 bytes") is editorial, not a moved bound, and
record it in `spec-delta.md`; (c) account for the 35->38 ingress / 19->20
runtime test-count delta against gate reports (only one new test is named
here).
Reversal: if (a) surfaces a helper-level failure mode, or (b) finds the byte
bound actually moved, reopen T02's gate rather than treating this as closed.

## 2026-09-04 - Dirty worktree sweep into `bd35c5b`

Considered: leave the pre-existing dirty worktree (icon-mode `main.rs` hunk,
mosaic UI, OpenCode client, `tmp/` prototypes, historical T01/T01b/T02
escalation and gate artifacts, handoff docs, layout/brainstorm/plan files)
untouched and unstaged as prior gates required, or commit it as one sweep
per the user's explicit "commit it all" direction.
Chosen: `bd35c5b` ("Complete dashboard support work") committed that sweep.
Verified by direct `git show --stat`/`--name-only`: it touched
`crates/dashboard/src/main.rs` (23 lines — consistent with the pre-existing
icon-mode hunk that T04's contract required be preserved verbatim and
unstaged), `mosaic/{ladder,palette,render,view}.rs`, `opencode/mod.rs`,
`opencode-client/src/opencode.rs`, `.gitignore`, and 70+ non-code files
(`tmp/` prototypes, `docs/internal/*`, task plans, handoff docs, and
historical T01/T01b/T02 escalation/gate reports for tasks that predate this
run's committed history). It did not touch any file under
`crates/dashboard/src/claude/` or `docs/specs/dashboard/`.
Why: the delivery profile's "unrelated dirty worktree changes must remain
untouched" constraint governs modification, not whether previously-approved
dirty work may be committed once the user explicitly directs it. No Claude
runtime/ingress/adapter source was touched, so the T04 owns-list boundary is
intact; the icon-mode hunk landing in this commit — not `fd83209` — is
consistent with the sealed Review Frame's requirement that it stay verbatim
and outside T04's approved hunks.
Limitations: this is a bookkeeping entry recording a fact that already
happened, not a new authorization. Future tasks must not treat `bd35c5b` as
license to bundle unrelated changes into a task commit without the same
explicit direction.
Reversal: none; this is historical record.

## 2026-09-04 - Live-validation credential isolation amendment

Considered: hold the live Claude Haiku validation to the original isolated
`HOME`/`CLAUDE_CONFIG_DIR` requirement, or relax it because an isolated `HOME`
has no credentials to authenticate with.
Chosen: for the Session 3 live run, isolation is real credentials used
opaquely — `--settings <temporary hooks JSON> --setting-sources project`
against the real environment, with `~/.claude`, project `.claude`,
credentials, and transcript JSONL never read or retained by the test
harness. The user approved this relaxation explicitly on 2026-09-04.
Why: the original decision's own reversal clause required explicit user
approval to relax isolated `HOME`/`CLAUDE_CONFIG_DIR`, and the Session 3
handoff recorded the relaxation in prose (Do-Not-Touch section) without an
amending decision entry. The advisor withheld M3 sign-off partly on this gap
and the run needs the wording T05 inherits on record here, not only in a
handoff that gets rewritten each session.
Limitations: this amendment covers only the Session 3 live run. Any T05
authenticated evidence gate must restate or re-approve this posture for its
own scope; it is not a blanket relaxation for all future Claude CLI testing.
Reversal: none stated beyond re-approval per future scope; if a future run
needs broader credential access, return to the user for explicit approval
again.

## 2026-09-04 - M3 cross-task sign-off

Considered: hold M3 open pending disclosure of the three post-gate commits
(`04a7cf5`, `bd35c5b`, `babf167`), or sign off once each is disclosed and
dispositioned.
Chosen: advisor (standing in for Terra in this session, per the advisor
skill) signed M3, conditional on six items closing before T05 decomposition
opens: (a) a helper-level test proving `EnvelopeTooLarge` exits 0/no stdout;
(b) confirm the R15 chars->bytes change is editorial and record it in
`spec-delta.md`; (c) account for the 35->38 ingress / 19->20 runtime test
delta; (d) a correction note on `gates/T04-report.md` for the pre-`babf167`
startup-ordering claim; (e) a `deferred.md` S2 evidence-status update
recording the two live findings (`--print` `SessionEnd` cancellation at
shutdown; `./tmp` cwd resolving to parent-repo project identity); (f) this
log staying in chronological order (done via this entry's placement).
Why: T02+T03+T04 form a coherent milestone — owns-lists disjoint and
layered, only the three contracted events used, no lifecycle mapping in
runtime code, T04's `main.rs` re-scope stayed inside its approved hunks, and
`babf167` is startup composition the T04 Review Frame approved by name. All
four T05 evidence deferrals (S1, S2, S3, S4/S5) remain open with their
promotion triggers unmet; the live run only partially informs S2
(`SessionStart`/`SessionEnd` observed; `UserPromptSubmit`, `PreToolUse`,
`PostToolUse`, `Notification`, `Stop` were not, since the test hook wired
only three events).
Limitations: gated commits remain T01c `401887e`, T02 `aeb8317`, T03
`e631129`, T04 decomposition `bdb8647`, T04 runtime `fd83209`. Post-gate
commits accepted on this sign-off: `04a7cf5`, `bd35c5b`, `babf167`. T05
decomposition should include an explicit rule for post-gate fixes to sealed
files (decision entry naming the sealed contract touched, the rule it fixes
against, and a test or stated reason none is needed) and at least one
acceptance criterion satisfiable only by the built binary on the real path —
the startup panic that `babf167` fixed passed 60+ unit/integration tests
across three gates because none of them exercised the composed binary.
Reversal: return to M3 escalation if items (a)-(e) surface a real defect
rather than confirming the accepted disposition.

## 2026-09-04 - Run-wide post-gate and gate-closure verification rules

Considered: leave post-gate sealed-file fixes and gate-closure evidence to
ad-hoc handling per task, or adopt explicit rules from the M3 review.
Chosen: two rules apply to every remaining task in this run, starting with
T05. (1) Any change to a sealed file outside its owning task's active gate
requires a decision entry naming the sealed contract touched, the rule the
change fixes against, and either a test or a stated reason none is needed —
before or immediately after the change, not discovered later by a milestone
review. (2) Every gate closure names the artifact it claims and the exact
command that confirms it exists/passes; a report of work done is not
sufficient on its own.
Why: `04a7cf5` was a substantively sound fix that went undisclosed for a full
milestone because no rule required disclosure of post-gate sealed-file
changes. Separately, during M3 closeout a delegated coder agent reported
editing `spec-delta.md` when `git status` showed no such change — caught
only because the conductor checked directly rather than trusting the
report. This run's existing gate closures (Luna's CLEAN verdicts, Clerk's
spec validations) are not known to be false, but nothing in the process to
date required independent confirmation that a reported artifact exists.
Limitations: rule (2) does not retroactively re-verify T01c-T04's gates; it
governs T05 forward. Rule (1) does not require reopening a sealed gate for
every sealed-file touch — a stated reason "no test needed" is a valid
disposition, not an automatic escalation.
Reversal: none; both rules stand unless a future milestone review finds
them producing false-positive escalations disproportionate to the risk they
catch.

## 2026-09-04 - T05 decomposition and seal

Considered: split T05 into separate tasks per evidence area (S1-S6), or
keep it as one task staged evidence-first, implementation-only-where-
justified.
Chosen: one task. `contracts/T05-claude-release-verification.md` v2, sealed
by the advisor. S1-S5 evidence areas plus S6 closure-by-citation, a named
failure branch per the delivery profile's existing blocking rule, release
regression/rollback, and final release sign-off. `state.rs` is owned
outright for staleness/subagent-identity logic (T05's planned deliverable
from T01's original deferral, not a defect discovery); `hook.rs`/`wire.rs`/
`listener.rs`/`command.rs`/`mod.rs` are conditional access under the
2026-09-04 post-gate-fix rule, bounded to fixing a defect against a rule
those contracts already state — any rule change is a contract amendment
requiring advisor approval before implementation, not a decision entry
after.
Why: S1-S5 are largely surfaced by the same authenticated session runs
rather than needing separate infrastructure, so splitting would multiply
gate overhead without proportional benefit. The advisor withheld seal on v1
for four reasons, all corrected in v2: a `state.rs` ownership contradiction
between the owns-list and two evidence bullets; conditional sealed-file
access needed a kind-bound (defect-against-existing-rule only, not any
change under process compliance); the release gate had no failure branch,
so evidence going the wrong way had no path except forcing a "done"
verdict; and S6 (delivery profile's socket/IPC evidence item) was
unaccounted for anywhere in the contract.
Limitations: the seal is conditional — if S6 turns out not to be closed by
the cited T02/T04 gate artifacts, that is a decomposition question
returning to the advisor before implementation, not something T05 resolves
unilaterally. Implementation has not started against this contract.
Reversal: return to the advisor if evidence during T05 forces a rule change
in a sealed file rather than a defect fix, or if S6's citation-closure
turns out incomplete.
