# M1 Fresh-Eyes Milestone Review

## Verdict

**FIT: BLOCKED pending a documentation correction.** The assembled implementation is behaviorally coherent at the requested integration seams, and the dashboard test/clippy gates are green. However, `docs/specs/dashboard/claude.md` still contradicts the shipped `Notification` attention mapping, so the delivery profile's documented-truth bar is not met.

## Findings

### Blocking: Claude spec contradicts T01 notification behavior

`docs/specs/dashboard/claude.md:113-119` says that although `Notification` contains attention-worthy subtypes, the state mapping "conservatively does not force an attention change on every sub-type yet." The assembled `crates/dashboard/src/claude/state.rs:381-409` now deliberately maps `idle_prompt`, `permission_prompt`, and `agent_needs_input` to `NeedsYou`, with the specified `turn_started` asymmetry.

This is not merely an implementation note: the delivery profile requires docs/specs to describe changed attention mapping, and the spec's current wording tells a reader the opposite of the resulting behavior. Update that clause to describe the three mappings and preserve the adapter-owned scope. This is the only M1 blocker found.

## Seam Assessment

- **Shared text helper relocation:** `text.rs` contains provider-neutral `looks_like_question`, `collapse_newlines`, and `basename`. OpenCode imports the helpers from `crate::text`; Claude imports the question heuristic and its action-line module imports the rendering helpers. The shared module contains no provider tool vocabulary. The helper bodies remain compatible with the OpenCode callers.
- **Claude ingress to tile invariants:** `hook`/`wire` already deliver the required fields. `state.rs` renders Claude action lines from bounded `tool_input`, gracefully falls back on malformed/truncated input, updates action history only on `PreToolUse`, and leaves `PostToolUse`/failure unable to double-count. `Stop`, notification, permission/elicitation content, and stale-text clearing compose as intended through snapshot construction. The existing `current_action`-at-turn-end concern remains correctly deferred and does not violate this milestone's stated timing fix.
- **Shared `state.rs` representation:** T01's notification/stop changes, T02's tool/content changes, and T03's session-start truth table operate on the same `attention`, `turn_started`, `pending_tool_use_id`, action fields, and final text without conflicting assignments. In particular, subagent `Stop` selects `Idle` before the top-level question heuristic, and compact/resume preserves the existing `AttentionState` rather than recomputing it.
- **Specs and docs:** `client.md` R1.3 correctly describes the shared question heuristic, and `layout.md` R5.3 correctly describes permission/elicitation Question content. The Claude spec's R13 paragraph is stale as described in the blocking finding above. `snapshot.rs:110-121` also contains an outdated comment claiming the OpenCode adapter is the only adapter constructing `Idle`; this is source documentation rather than a functional contract, but should be corrected when the state-model docs are next touched.
- **Decomposition and ownership:** The M1 split held. T00 isolated the coupling-direction change; T01/T02/T03 remained serialized around the shared `state.rs` file and their changes are distinguishable. The provider-specific action dispatch stayed in Claude/OpenCode modules while neutral mechanics moved to `text.rs`. No unrelated mosaic/shell/opencode-client/docs work was considered part of M1.

## Residual and Deferred Concerns

- `current_action` is not cleared by turn-ending `Stop`; it is documented in `deferred.md` with the agreed live-proof/future-Stop-arm trigger and is not an M1 blocker.
- Live end-to-end hook/transcript proof remains explicitly out of scope.
- Claude tool argument field names remain an implementation assumption documented by T02; no integration evidence in these commits contradicts it.
- The `snapshot.rs` adapter-ownership comment should be updated as documentation follow-up, but it does not change the fit verdict independently of the Claude-spec blocker.

## Commits Reviewed

- `305f24c` — T00: Relocate shared text helpers out of opencode into `crate::text`
- `d16330a` — T01: Map Notification/Stop events to correct attention state
- `3d3bfa2` — T02: Render truthful Claude tile content
- `d1f41b7` — T03: preserve attention across compact and resume

## Verification

- `cargo test -p dashboard`: **pass**, 265 unit tests, 8 adapter integration tests, 67 ingress tests, 20 runtime tests, 0 doc tests; **360 passed, 0 failed**.
- `cargo clippy -p dashboard --all-targets`: **pass**, clean.
- `git diff --check 69addca..d1f41b7`: **pass**, no whitespace errors.

## Re-review

### Verdict

**FIT: PASS.** The prior documentation blocker is cleared. No new blocking
finding was found at the M1 integration seams or against the approved delivery
profile.

### Prior blocker disposition

- `docs/specs/dashboard/claude.md` R13 now explicitly documents all three
  attention-worthy notification mappings: `idle_prompt` to `NeedsYou` without
  the question flag, and `permission_prompt`/`agent_needs_input` to
  `NeedsYou` with the question flag. It also states that absent or unknown
  subtypes preserve existing attention. This agrees with
  `crates/dashboard/src/claude/state.rs:389-408` and the notification tests.
- `docs/specs/dashboard/client.md` now cross-references the fifteen-event R13
  matrix and bounded R14-R15 content instead of claiming a three-event adapter
  with empty content fields. It retains the `SessionEnd`/`Gone` contract and
  is consistent with the current adapter boundary.
- `tasks/.../spec-delta.md` and the append-only `decisions.md` entries record
  both corrections and their decomposition rationale. The correction is
  bounded to the M1-owned documentation and does not recut T01/T02.

### Seam assessment

- The shared `crate::text` module remains provider-neutral; Claude and
  OpenCode import the relocated helpers without Claude depending on OpenCode
  internals. Claude-specific tool dispatch remains in
  `claude/action_line.rs`.
- T01, T02, and T03 compose on the same state representation without
  conflicting assignments: notification and stop attention mapping, rendered
  action/current-history updates, permission/elicitation text clearing, and
  compact/resume preservation are all present in the integrated `state.rs`.
  In particular, subagent `Stop` takes the `Idle` branch before the top-level
  question heuristic, and `PostToolUse`/`PostToolUseFailure` do not update the
  action pair.
- Ownership remains within the approved decomposition. The current unrelated
  mosaic/shell/opencode-client work was excluded; only the uncommitted
  M1-owned Claude/client documentation and run records were considered.
- The accepted deferrals remain documented and unchanged in substance:
  turn-end `current_action` clearing, live end-to-end proof, the stale
  `snapshot.rs` adapter-ownership comment, and broad legacy spec-rubric
  cleanup are not M1 blockers under the profile.

### New observations

The module comment at `crates/dashboard/src/claude/state.rs:67` says “the two
sub-types” while naming three notification subtypes. The mappings immediately
below are explicit and correct, so this is a non-blocking wording nit, not a
consumer-facing contract contradiction. No other new finding crossed the
blocking line.

### Verification

- `cargo test -p dashboard`: **pass**, 265 unit tests, 8 Claude adapter
  integration tests, 67 ingress tests, 20 runtime tests, and 0 doc tests;
  **360 passed, 0 failed**.
- `cargo clippy -p dashboard --all-targets`: **pass**, clean.

## Advisor Sign-Off

**FIT PASS — M1 signed off on 2026-09-05.** The approved delivery profile is
satisfied: T00-T03 passed their contracts, both documented-truth blockers were
corrected and recorded in `spec-delta.md`, and the fresh targeted validation
and re-review are green. Retained deferrals are turn-end `current_action`
clearing, the stale `snapshot.rs` adapter-ownership comment, broad legacy
spec-rubric cleanup, and live end-to-end hook/transcript proof.

The module-comment wording nit ("the two sub-types" while naming three
Notification subtypes) is accepted as a documentation-only deferral, with
promotion when the state-model documentation is next touched, especially M2.
