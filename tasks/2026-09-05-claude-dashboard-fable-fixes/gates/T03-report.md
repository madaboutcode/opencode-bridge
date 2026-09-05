<!-- gates/T03-report.md -->

# T03 — gate report

**Conformance:** yes — the T03-owned `SessionStart` arm now preserves an
already-tracked session's attention for `Compact` and `Resume`, resets it for
`Startup`, `Clear`, absent, and other sources, and initializes every untracked
session through the existing `ensure_tracked` path. The module doc comment
records the lifecycle rule. The diff is confined to `state.rs`'s module
documentation, import, and `SessionStart` arm; unrelated worktree changes
remain unstaged.

**Calibration:** delivery profile version 2 · contract version 1 · report-only
refine-loop gate.

**Passes:**
1. Implemented the smallest truth-table change: tracked status is captured
   before unconditional `ensure_tracked`, and only tracked `Resume`/`Compact`
   skip the Idle reset.
2. Independent local review of the changed areas found no above-line issues;
   the requested Luna subprocess review could not be spawned because this
   session is already at the runtime's maximum nested-agent depth.
3. `cargo test -p dashboard` passed: 265 unit tests, 8 adapter integration
   tests, 67 ingress tests, and 20 runtime tests, plus 0 doc tests.
4. `cargo clippy -p dashboard --all-targets` completed cleanly.

**Residuals:** none identified within T03's contract boundaries.

**Challenges:** none.

**Contested:** none.

**Deferred:** none new; the existing `deferred.md` entries are unrelated to
T03 and were left unchanged.

**Rejected:** none.
