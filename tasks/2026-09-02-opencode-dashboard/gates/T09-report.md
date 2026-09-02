<!-- gates/T09-report.md. Written by the task's runner; it is the conductor's entire
     gate and must stand alone. -->

# T09 — gate report

**Conformance:** yes — reviewer's (`ask_opus`) explicit pass-1 verdict against
the contract's Acceptance section, all nine criteria individually confirmed:
`HarnessAdapter` trait with the opencode adapter as sole implementation and
zero wire-type leakage into the core-facing types (AC1); snapshot carries
every listed field with timestamps, not pre-rendered strings (AC2); project
identity resolver cross-compared line-for-line against the T01 spike, plus a
dedicated test proving the caching obligation (exactly one `git` spawn across
three resolutions of one session) (AC3); `SessionInfo` additive-only, 29
`opencode-bridge` tests unchanged (AC4); tool-call correlation and
action-line rendering proven with fixtures matching the SSE event catalog
(AC5); reconcile sweep proven independent of SSE health via a sweep-only test
(AC6); tombstone-on-vanish proven, including non-interference with other
sessions (AC7); no TUI dependency in `Cargo.toml` (AC8); build/test/clippy/fmt
clean workspace-wide, nothing outside the owns-list touched (AC9).

**Calibration:** delivery profile version 1 · contract version 2 · Review
Frame "as of" contract version 2 — confirmed by direct read of the contract
before spawning either agent, matched, no mismatch.

**Passes:**

- Pass 1 — implementer: `coder` (Agent-tool subagent). Built the boundary as
  a physical split: `crates/dashboard/src/{adapter.rs, snapshot.rs,
  project_identity.rs, lib.rs}` at the crate root (harness-agnostic), and
  `crates/dashboard/src/opencode/{mod.rs, reconcile.rs, action_line.rs,
  question.rs, session_state.rs}` (opencode-specific), plus additive
  `location`/`project_id`/`subpath`/`parent_id` fields on
  `opencode-client`'s `SessionInfo`. 34 new tests in `dashboard`, 29
  pre-existing `opencode-bridge` tests unchanged. `cargo
  build/test/clippy/fmt` all clean on first attempt — no fix cycle was
  needed.

  Reviewer: `ask_opus` (Agent-tool subagent), independent judgment (not
  shown the implementer's own report or its self-flagged deviations before
  forming its verdict). Confirmed all 9 acceptance criteria yes, and
  independently endorsed the implementer's two most interpretive calls: (a)
  `AttentionState::Idle` exists in the shared type but the opencode adapter
  never constructs it (T12's job, per `overview.md` R3's window-filter
  reclassification, which the adapter has no visibility into) — reviewer
  called this "the kind of boundary clarity that makes the design
  composable"; (b) `HarnessKind` as a string newtype rather than an enum, so
  a second harness never requires changing the core type. Reviewer raised
  two new low-severity findings, both self-dispositioned by the reviewer as
  defer-with-trigger, not correct-now (see Deferred below). No finding
  contested the delivery profile or the Review Frame.

  Runner's own check: read the full diff (`git status`/`git diff --stat`)
  independently — confirmed `crates/opencode-bridge/**` and `docs/specs/**`
  are untouched, and every non-owns-list file the diff does touch
  (`Cargo.lock`, `crates/opencode-client/src/lib.rs`'s one-line re-export
  addition, `crates/dashboard/src/main.rs`'s module-wiring edit) is a
  minimal, necessary consequence of the contract's own scope, not scope
  creep.

- No pass 2. Per the refine-loop's stopping rule ("pass 1 clean → done"):
  the reviewer's pass-1 verdict was conformance-yes on every criterion, with
  zero correct-now findings — both findings were already dispositioned by
  the reviewer itself as defer-with-trigger. Nothing needed fixing, so a
  verification pass had nothing to verify. Passes used: 1 of the 2-pass
  budget.

**Residuals:** none above the depth line.

**Challenges:** none — no finding from either agent contested the delivery
profile or the contract's Review Frame.

**Contested:** none.

**Deferred:** 4 items appended to `deferred.md` under a new "T09" heading —
2 from the reviewer (missing `FALLBACK-OK` citations on adapter-internal
fallback paths; `start_turn` not clearing `current_action` across a turn
boundary), 2 self-flagged by the implementer and independently judged real
and worth carrying forward by the runner (R6.5's edit-fallback-to-full-path
behavior is unreachable in the current snapshot shape — flagged specifically
for T11 to check before it gates; `GET /api/session` has no pagination loop,
folded under the existing 50+ session deferral). A fifth implementer-flagged
item (`turn_started` falling back to `last_updated` when a running session is
first discovered via sweep rather than SSE) is also in the new entry, judged
real but narrow. Three other implementer-flagged deviations were judged
resolved, not deferrals: the `parent_id` addition to `SessionInfo` (the
contract's own snapshot-shape list requires it in the core type with no
other wire source — reviewer independently confirmed this as correct under
AC4); action-line truncation left to render time (consistent with how
`layout.md` already treats nickname truncation, and reviewer did not flag
it); `final_assistant_text`/`last_user_prompt` fetched via REST on state
transition rather than accumulated from SSE deltas (a reasonable mechanism
choice within the contract's silence on how, not flagged by the reviewer).

**Files changed (owns-list, per the contract's Boundaries section):**
- `crates/dashboard/src/adapter.rs`, `snapshot.rs`, `project_identity.rs`,
  `lib.rs` (new — core boundary and model types)
- `crates/dashboard/src/opencode/mod.rs`, `reconcile.rs`, `action_line.rs`,
  `question.rs`, `session_state.rs` (new — the opencode adapter)
- `crates/dashboard/src/main.rs` (edited — module wiring only, still prints
  the "not yet implemented" placeholder; the real event loop is T12's)
- `crates/dashboard/Cargo.toml` (added `tokio`, `serde_json`; no TUI crate)
- `crates/opencode-client/src/opencode.rs` (additive `SessionInfo` fields:
  `location`, `project_id`, `subpath`, `parent_id`, all `#[serde(default)]`)
- `crates/opencode-client/src/lib.rs` (one-line re-export addition,
  `SessionLocation`, needed to expose the new field's type)
- `Cargo.lock` (registers the two new `dashboard` deps; no new fetches, both
  already resolved elsewhere in the workspace tree)

Nothing under `crates/opencode-bridge/**`, `docs/specs/**`, rendering/layout
code, naming/claim-map code, or the main event loop/terminal/keyboard-input
path was touched.
