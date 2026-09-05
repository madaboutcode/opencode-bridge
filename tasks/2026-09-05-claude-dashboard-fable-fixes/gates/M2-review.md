# M2 Milestone Review

**Branch:** `claude-dashboard-fable-fixes`

## Task Commits

- T05 — `b88e56e` — serde-backed Claude wire conversion, exhaustive wire
  compatibility tests, and T05 gate report.
- T04 — `74ecc97` — turn facts, `idle_since`, projection, matrix coverage,
  permitted state-model comments, and initial T04 gate artifacts.
- T04 follow-up — `71841e5` — retained first-tool receipt in `turn_started`
  and Resume/Compact regression coverage after fan-in review.

## Gates

- T04 contract version 4 / Review Frame v3: final fresh `luna-high` review
  PASS; no findings.
- T05 contract version 3 / Review Frame v2: final fresh `luna` review PASS;
  no findings.
- M2 fan-in review across all three commits: fresh `luna-high` PASS; no
  findings.
- Final verification: `cargo test -p dashboard` 376 passed; clippy clean;
  `cargo fmt --all -- --check` passed; scoped `git diff --check` passed.

## Residuals

- Live Claude hook end-to-end proof remains explicitly deferred until the next
  phase and must use an isolated `claude --settings <scratch-path>` config.
- `crates/dashboard/src/claude/DESIGN.md` still describes the removed stored
  `attention` field. The fan-in reviewer classified this as documentation drift
  outside the T04 ownership surface and non-blocking for runtime conformance;
  it should be corrected in a bounded documentation follow-up before or with
  live-proof.

## Sign-Off Request

The implementation gates are green and both task commits are isolated from the
unrelated dirty mosaic/shell/opencode-client workstream. Advisor disposition is
requested for M2 milestone sign-off with the two bounded residuals above.

## Advisor Sign-Off

**FIT PASS.** The advisor accepted the three isolated task commits and all
final verification evidence. The stale `claude/DESIGN.md` paragraph is a
bounded documentation follow-up, not a task reopen or release blocker. Live
hook proof remains deferred to the next phase and must use an isolated Claude
settings path.
