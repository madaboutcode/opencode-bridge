<!-- gates/T01-report.md -->

# T01 — gate report

**Conformance:** yes — reviewer's explicit verdict: all five acceptance items plus 3a met, diff confined to the contract's Boundaries. Verified directly against the diff, not the implementer's summary: `Notification` mapping (idle_prompt → `NeedsYou{question:false}` + `turn_started = None`; permission_prompt/agent_needs_input → `NeedsYou{question:true}`, `turn_started` untouched; anything else unchanged); subagent `Stop` → `Idle`; top-level `Stop` → `looks_like_question(last_assistant_message)` imported from T00's `crate::text` module (checked `text.rs` directly, not just the `use` line — not a reimplementation); item 3a narrowed to the named clause in `client.md` R1.3, forward-looking clause byte-identical/untouched; `decisions.md` entry closes T00's deferred.md trigger. `git diff` against `hook.rs`, `wire.rs`, `docs/specs/dashboard/claude.md` empty; `StopFailure` untouched; no other `process()` arm touched.

**Calibration:** delivery profile version 2 · contract version 2 (Review Frame v2, refreshed for item 3a)

**Passes:**
1. Reviewer read the diff cold (no findings supplied at spawn), ran `cargo test -p dashboard` and `cargo clippy -p dashboard --all-targets` independently rather than trusting the implementer's claimed numbers: 338 passed (333 baseline + 5 new), 0 failed; clippy clean (touched state.rs to bust cache, reran to confirm). Specifically verified the two flagged risk points: item 1's asymmetry preserved as-written (not flattened into symmetry — confirmed via tests that drive a second event afterward and check which timestamp `turn_started` carries); item 3 does not leak into item 2's path (direct test: subagent `Stop` ending in "?" still resolves to `Idle`, not `NeedsYou`). No findings above the depth line. No manufactured findings — reviewer had explicit permission to report "clean."

**Residuals:** none.

**Challenges:** none — reviewer did not contest the profile or Review Frame.

**Contested:** none.

**Deferred:** none new. (T00's deferred.md entry on `client.md` R1.3 is closed by this task's item 3a; see `decisions.md`'s "T01 item 3a" entry.)

**Rejected:** none.
