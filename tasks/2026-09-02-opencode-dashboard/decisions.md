# Decisions — opencode-dashboard conductor run

## 2026-09-02 — Scoping sign-off, M1 scoped to the project-identity spike

Considered: scoping the whole run (spike + spec-writing + implementation) in
one pass, vs. scoping M1 concretely and drafting M2/M3 at a coarse level only.
Chosen: coarse M2/M3, concrete M1 — decomposed in detail once each is reached.
Why: specs for M2 don't exist yet and M3 depends on M2's output; scoping them
in detail now would be discovery dressed as decomposition. Limitations: M3 was
initially scoped without seeing that it requires a workspace migration
(`crates/dashboard` + `crates/opencode-client`) that collides with this repo's
existing published-crate structure and release CI — advisor caught this at
scoping review; PLAN.md's M3 bullet now names it explicitly as M3's own first
task. Reversal: if M2's specs turn out to need something M3's boundary can't
absorb without re-migrating twice, re-open M3's scope before decomposing it.

Sign-off: advisor, after two required contract fixes (T01 probe-session
inertness; worktree-fixture `.git/worktrees/` handling) and two soft fixes
(git-policy M3 qualifier; T01 check #4 conditional-acceptance wording) — all
four applied to PLAN.md / T01's contract before T01's runner spawns.

## 2026-09-02 — Baseline exemption for root's pre-existing red `cargo fmt --check`

Considered: block the run until the pre-existing uncommitted work (unrelated to
this run) is committed/stashed and the root package is green, vs. record a
baseline and gate on "no new failures."
Chosen: baseline + no-new-failures gate for root; full green required (no
exemption) for any new spike crate under `tmp/`.
Why: the failing files (`src/log.rs`, `src/tools.rs`) are someone else's
in-progress work this run has no mandate to touch or fix; root has no
`[workspace]`, so `tmp/` spikes are structurally isolated from that red anyway
— there's no real risk being papered over.
Limitations: if a later M3 task needs to touch `src/log.rs`/`src/tools.rs`
(plausible — the workspace migration moves them), the baseline exemption no
longer applies to that task; it inherits normal green-bar rules for whatever it
touches.
Reversal: if the pre-existing work gets committed or stashed before M3, drop
the baseline note — root's bar is genuinely green at that point.

## 2026-09-02 — M1 milestone sign-off; no deferred items promoted

Considered: promote the `SessionInfo` deserialization gap or the
case-normalization gap into a new M1/M1.5 task now, vs. carry both forward as
deferred with corrected framing.
Chosen: carry forward, no promotion — see `gates/M1-outcome.md` for the full
review. Both gaps' scale assumptions (M3 doesn't exist as a task yet; no
case-insensitive-filesystem environment is in play) still hold.
Why: promoting without a broken scale assumption is tidiness, not a real
decision — the milestone stage explicitly warns against this.
Limitations: the case-normalization gap is now flagged (by advisor) as
resolvable with one more fixture, cheaply, in M2 — carried forward as a
strengthened OPEN item rather than a task, since it doesn't block anything by
itself.
Reversal: if M2's spec work or M3's implementation actually needs either gap
closed to proceed, promote it into a real task at that point instead of
deferring further.
Sign-off: advisor, contingent on committing the milestone's actual product
(the R1.6 edit in the requirements doc, previously untracked) plus
`decisions.md`/`contracts/`/`advisor-brief.md`, all landed in the same
sign-off commit as this entry. PLAN.md's git policy corrected to name project
docs explicitly as milestone artifacts, so this doesn't recur at M2/M3.
