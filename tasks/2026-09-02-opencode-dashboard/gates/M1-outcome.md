<!-- gates/M1-outcome.md. Milestone review outcome, per stages/milestone.md. -->

# M1 — project-identity spike: milestone outcome

**Result:** signed off by advisor, 2026-09-02, after one required fix.

**Fit review:** M1 had exactly one task (T01); no cross-task integration to
check. The review was whether T01's evidence actually closes M1's stated
question. It does: R1.6 confirmed as written against all 9 checks (4 wire, 5
canonicalizer), one clause (case normalization) left explicitly open rather
than silently passed.

**Git evidence:** run branch `conductor/opencode-dashboard`. T01 commit
`9aa690e` ("T01: project-identity resolution spike"), gated pass 2, no
residuals — verified against `gates/T01-report.md` and `git log`/`git show`
before sign-off was requested.

**Required fix before sign-off could close:** the milestone's actual product —
the R1.6 edit in `tasks/2026-09-01-opencode-dashboard.requirements.md` — had
never been committed (it predates this run and was never in any commit).
`decisions.md`, `contracts/`, and `advisor-brief.md` were in the same
untracked state. PLAN.md's git policy didn't name project docs as milestone
artifacts, which is why it fell through; fixed in PLAN.md's Git policy
section.

**Notes carried into M2 (not blockers, not promoted to tasks):**
1. Case-normalization is one fixture away from closing outright (resolve via
   a deliberately wrong-cased path) — worth doing early in M2 rather than
   leaving it open indefinitely. Requirements doc's OPEN item corrected to
   say so.
2. `client.md` (M2 spec) must state that git-toplevel resolution is a
   subprocess spawn and needs per-session caching, not re-resolution per
   snapshot/redraw — added as an implementation note under R1.6.
3. `fs::canonicalize`'s existence precondition (deleted/unmounted directory)
   is a real gap, happy-path-deferred per PLAN.md — added to `deferred.md`,
   not a new task.

**Housekeeping:** two machine-specific symlink fixtures
(`fixtures/symlink-to-repo-root`, `fixtures/test-symlink-to-repo-root`) were
untracked from T01's commit and added to the spike's `.gitignore` — they were
throwaway test artifacts with absolute local paths, not meant for history.

**Promotions:** none. Neither deferred item's scale assumption has broken.

**Exit:** M2 (spec tree) decomposition next.
