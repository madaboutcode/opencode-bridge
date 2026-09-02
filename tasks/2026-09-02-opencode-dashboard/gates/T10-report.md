<!-- gates/T10-report.md. Written by the task's runner; it is the conductor's entire
     gate and must stand alone. -->

# T10 — gate report

**Conformance:** yes — reviewer's (`ask_opus`) explicit pass-1 verdict against
the contract's Acceptance section, all eight criteria individually confirmed:
both claim layers built on T09's real `ProjectId`/`SessionId`/`Timestamp`
types with no redefinition (AC1); a test proves claim order is
creation-time-ordered by feeding non-monotonic creation times in two
genuinely different arrival orders and asserting identical resulting claims
(AC2); both hard guarantees proven at design-center scale (4 projects/2
sessions) and at the capacity boundary (10 projects/10 categories, a category
filled to its word count) (AC3); cooldown proven via a documented N=2 rule —
a freed word enters a cooldown table and only clears once 2 other distinct
words in the category are claimed (AC4); release on T09's tombstone signal
implemented and proven — freeing the last live session in a project frees
its category, and the freed category is claimable by another project (AC5);
capacity overflow proven to degrade rather than panic — an 11th live project
shares its preferred category instead of failing, a session past its
category's word count gets a numeric-suffixed name, and per-project
uniqueness still holds under the suffix path (AC6); the 10-category/word
Appendix reproduced verbatim (105 words total, not 60 — see note below) with
a test asserting no cross-category repeats (AC7); build/test/clippy/fmt clean
workspace-wide, nothing outside the owns-list touched (AC8).

**Calibration:** delivery profile version 1 · contract version 1 · Review
Frame "as of" contract version 1 — confirmed by direct read of the contract
before spawning either agent, matched, no mismatch.

**Passes:**

- Pass 1 — implementer: `coder` (Agent-tool subagent). Built
  `crates/dashboard/src/naming/{mod.rs, wordlist.rs, claim_map.rs}` plus
  module wiring in `crates/dashboard/src/lib.rs` (`pub mod naming;` and
  re-exports). 15 new tests in `naming::`, all 34 pre-existing T09 tests and
  29 `opencode-bridge` tests unaffected (49 total in `dashboard`). `cargo
  build/test/clippy/fmt` all clean on first attempt — no fix cycle was
  needed.

  Cooldown rule chosen by the implementer: N=2 — a freed word is reclaimable
  once 2 other distinct words in the same category have been newly claimed
  since it was freed. Documented reasoning: categories run as small as 10
  words at design-center churn (~2 sessions/project), so a higher N risked
  locking up a small category, while N=2 still prevents a word jumping
  straight back to a new session.

  Capacity-overflow degrade chosen by the implementer: category overflow (an
  11th live project, no free category) shares the preferred category
  (`shared: true`) rather than failing — word claims stay category-scoped
  under sharing, so only the cross-project guarantee (the one the spec
  already says can't hold past the capacity assumption) is affected. Word
  overflow (more sessions in a project than its category has words, or a
  transient state where every remaining word is in cooldown) gets a
  numeric-suffixed name (`"Apollo-2"`, smallest unused suffix), which keeps
  the per-project uniqueness guarantee intact even under overflow. The
  implementer extended this same suffix path to cover a cooldown-lockout
  case (every remaining word transiently in cooldown, not true capacity
  overflow) beyond what AC4/AC6 strictly required, judged necessary to keep
  "never panic, never silently break a guarantee" airtight under churn — the
  runner accepts this as within the AC6 intent, not scope creep, since it's
  the same degrade mechanism applied to an adjacent impossible-without-it
  case.

  Reviewer: `ask_opus` (Agent-tool subagent), independent judgment. Verified
  the AC2 test data is genuinely non-monotonic and reordered by the sort (not
  incidentally correct from pre-sorted input); verified both hard guarantees
  via a shared assertion helper run at both scales, including that the
  capacity-boundary test correctly discovers each project's actual assigned
  category (accounting for hash-based assignment) before filling it;
  verified the cooldown mechanism is a real two-step lifecycle, not a no-op;
  verified `release_session`'s doc comment names `adapter::SessionEvent::Gone`
  as its intended trigger and the type matches T09's real signal (wiring the
  call itself is T12's job, correctly out of T10's scope); verified the word
  list programmatically against the spec Appendix, character-for-character,
  all 105 words in order. One low-severity finding, self-dispositioned by
  the reviewer as style/below-the-depth-line (see Deferred below). No finding
  contested the delivery profile or the Review Frame; reviewer explicitly
  noted the Review Frame's depth calibration ("guarantees and edge cases are
  the whole value") matched what it found — the interesting questions were
  all in claim-resolution and release-path correctness, and those held.

  Runner's own check: confirmed via `git status --short` that only
  `crates/dashboard/src/naming/{mod.rs,wordlist.rs,claim_map.rs}` (new) and
  `crates/dashboard/src/lib.rs` (module wiring) changed — nothing under
  T09's adapter/model files, T11's (nonexistent) render code, or
  `docs/specs/**` was touched.

- No pass 2. Per the refine-loop's stopping rule ("pass 1 clean → done"): the
  reviewer's pass-1 verdict was conformance-yes on every criterion, with zero
  correct-now findings — the one finding was already dispositioned by the
  reviewer as below the depth line. Nothing needed fixing, so a verification
  pass had nothing to verify. Passes used: 1 of the 2-pass budget.

**Residuals:** none above the depth line.

**Challenges:** none — no finding from either agent contested the delivery
profile or the contract's Review Frame. The reviewer flagged (not a
challenge, just a factual note) that the contract's context line says "60
words" while the actual Appendix has 105 — the contract's own instruction
("sourced verbatim from `visuals.md`'s Appendix") is what was followed, and
the implementation matches the real Appendix, so no discrepancy in what was
built.

**Contested:** none.

**Deferred:** 2 items appended to `deferred.md` under a new "T10" heading —
(1) a missing `FALLBACK-OK` citation on `claim_map.rs:377`'s `unwrap_or(0)`
in `release_session`, found by the reviewer and judged real-but-below-the-line
(self-heals rather than panics on an invariant that should be provably
impossible to break); (2) an informational note updating the delivery
profile's R6.8 capacity-edge-case deferral — the profile assumed the
consequence would be "a duplicate name, i.e. the guarantee silently fails,"
but T10's actual behavior degrades visibly instead (category sharing or a
numeric-suffixed name), so a future task hitting that deferral's trigger
should expect that behavior, not a raw silent duplicate.

**Files changed (owns-list, per the contract's Boundaries section):**
- `crates/dashboard/src/naming/mod.rs` (new — module root, re-exports)
- `crates/dashboard/src/naming/wordlist.rs` (new — frozen 10-category/105-word
  Appendix, copied verbatim from `docs/specs/dashboard/visuals.md`)
- `crates/dashboard/src/naming/claim_map.rs` (new — `NamingClaimMap`:
  project→category and session→word claim logic, cooldown, tombstone
  release, capacity-overflow degrade)
- `crates/dashboard/src/lib.rs` (edited — `pub mod naming;` and re-exports
  only; also updated the top doc comment to mark T10 as landed)

Nothing under `crates/dashboard/src/{adapter.rs,snapshot.rs,project_identity.rs,opencode/**}`
(T09's types, read-only consumed), T11's render code (doesn't exist yet), or
`docs/specs/**` was touched.
