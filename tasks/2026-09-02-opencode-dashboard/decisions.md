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

## 2026-09-02 — R6.8 naming scheme redesigned (out-of-band of M1/M2 tasks)

Considered: leave R6.8's adjective+noun scheme as-is (already CONFIRMED,
out of scope per PLAN.md's "don't re-litigate CONFIRMED sections" rule) vs.
revisit it now that the user wants single-word categorized names.
Chosen: revisit and rewrite R6.8 directly (coordinator-authored, not
delegated — this was design judgment, not implementation). Ran it through
`advisor` as a brainstorming pressure-test, not a formal scoping/milestone
gate, since it's a requirements-doc edit between milestones, not a task
inside M1 or M2.
Why: the user's ask (single word, no adj+noun combining, categorized by
project) couldn't coexist with the old "accept collisions, never fix at
render time" rule at single-word scale (birthday-paradox math) — a real
correctness conflict, not a style preference, so it had to be resolved before
M2's `client.md` could describe project resolution's neighbor, the naming
scheme.
Outcome: two-layer claim scheme (project→category, session→word), both
hash-preferred with deterministic probe-on-conflict; cooldown-based
recycling (not immediate reuse); category claim guarantees no two live
projects share a category, so screen-wide uniqueness holds given category
count > live project count. 10 categories (60 words) approved and frozen —
see the Appendix in the requirements doc.
Limitations: capacity edge case (more live projects than categories, or more
sessions than one category's word count) has no defined fallback yet —
recorded as an OPEN item, deferred as happy-path-first per the same design
center as the conductor run.
Reversal: if either capacity assumption breaks in real use, or R1.7's
eventual staleness rule can't cleanly evict claims, revisit before M3 builds
the resolver.

## 2026-09-02 — M2 decomposition: 5 spec files not 4; skip glossary.md/interfaces/

Considered: keep PLAN.md's original 4-file sketch (overview/layout/interactions/client)
vs. split the R6 series (card content, attention states, chrome, nickname) into
its own `visuals.md`. Also considered: follow `writing-specs`'s full fixed-location
convention (`docs/specs/glossary.md`, `docs/specs/interfaces/`) vs. skip both.
Chosen: 5 files, `visuals.md` added; skip glossary.md and interfaces/.
Why: R6's series, expanded to spec form with co-located scenarios, projects well
past the skill's ~80-100 line split threshold — folding it into layout.md would
produce an oversized, two-concern file. glossary.md and interfaces/ are pinned
locations specifically because the `greybeard` process and its QA agents depend
on them; this run isn't using greybeard, so there's no consumer for those files
— adding them now would be building for a process that doesn't exist yet.
Limitations: if this project later adopts greybeard/QA-agent workflows, both
files need to be created retroactively — not a large cost, but real.
Reversal: revisit if a future milestone brings in the greybeard process, or if
visuals.md itself grows large enough to want a further split (e.g. nickname
scheme spun out on its own).

## 2026-09-02 — T07's mandatory validation: reinterpret "all items passing"

Considered: force a literal 0-failures result across the generic
`writing-specs` validation rubric on all 5 M2 spec files (per the skill's
"not done until the validator reports all items passing"), which would mean
restructuring every file to the rubric's canonical PURPOSE/CONTENTS/SCOPE
headers and `(see path R#)` cross-ref format, and stripping architecture
vocabulary (`HarnessAdapter`, squarify, SSE event names, the hash-claim
scheme) that the "consumer lens" check flagged as implementation detail —
vs. accept the rubric's structural/consumer-lens checks as inapplicable
here, with each miss explicitly justified in the gate report.
Chosen: the latter — T07 adds genuinely additive fixes only (a short
Purpose/Contents/Scope framing near each file's top), does not restructure
to match the rubric's template, does not strip technical vocabulary.
Why: the rubric's structural checks assume the skill's own canonical
template; T02 (already reviewed and gated) deliberately established a
different, internally consistent convention instead — reopening that now
would relitigate a closed gate over a checklist artifact, not fix a defect.
The consumer-lens check assumes a UI-spec audience (end user); this spec
tree's stated audience, in every T02-T06 contract's Context line, is future
M3 implementers building the dashboard crate — for `client.md` specifically
that makes it an interface/data-contract spec (the skill's own
`references/api-specs.md` shape), where technical vocabulary is the
consumer-observable content, not something to hide.
Limitations: "all items passing" no longer means literal 0 failures across
the generic rubric — it means every applicable check passes and every
inapplicable one is explicitly justified in the gate report, not silently
waved off. A future reader auditing this run needs to read the justifications,
not just a pass count.
Reversal: if this project later needs to interoperate with a `writing-specs`
consumer that expects the canonical template (e.g. `greybeard`'s QA agents,
per the earlier "skip glossary.md/interfaces/" decision above), revisit
whether the convention itself should conform instead of the validator being
reinterpreted around it.

## 2026-09-02 — M2 milestone sign-off: R1.4/R6.8 state-ownership fix + claim-order pinning

Considered: at M2's milestone review, advisor found `client.md` R1.4
("core never holds derived state between frames") directly contradicts
`visuals.md` R6.8 (a live claim-map + cooldown counters that must persist
across frames) — a real integration gap T07's cross-link pass didn't catch.
Two resolution directions existed: narrow R6.8 to fit R1.4 (make the core
genuinely snapshot-only, push naming state elsewhere), or amend R1.4 to
carve out an exception for identity-keyed state.
Chosen: amend R1.4. Advisor's argument: no adapter can see across projects,
and cross-project category exclusivity (one of R6.8's two hard guarantees)
requires exactly that visibility — so the state has to live at the core,
which means R1.4's rule was mis-scoped (it should govern snapshot *content*,
not all cross-frame state), not R6.8.
Also fixed in the same pass: R6.8's claim *order* was never pinned (only
probe order was) — on a batched discovery (dashboard startup), the same
live session set could produce different names run-to-run depending on
wire-arrival order. Fixed by resolving batched claims in ascending
creation-time order. Also stripped R6.8's over-specific mechanism notation
(literal hash-mod formulas, two named counters, a wordlists filename
convention) that violated this project's own `docs/specs/CLAUDE.md`
consumer-lens rule, while keeping the guarantees and the new claim-order
rule exactly as specific as before.
Why: both edits are small, targeted text changes to existing requirements,
not new design — advisor explicitly said no new task/loop pass was needed,
just the diffs shown for sign-off.
Limitations: cooldown state itself doesn't survive a dashboard restart — a
narrow case remains where a session pushed off its preferred word by
cooldown right before a restart can reclaim it after, since cooldown
history is gone. R6.8's claim-order fix doesn't cover this case; not fixed
in M2, recorded in `gates/M2-outcome.md` for M3 to handle (record as known
behavior, or soften the restart-reproducibility claim).
Reversal: if M3's claim-map implementation finds the restart-reproducibility
gap above actually matters in practice (e.g. cooldown windows turn out to be
long relative to how often the dashboard restarts), revisit whether cooldown
state needs to persist across restarts after all.
Sign-off: advisor, unconditional after these fixes — read both files
directly, confirmed the exception properly scopes the rule and names an
owner, confirmed the claim-order fix correctly derives a project's position
from its earliest live session (a case advisor flagged as non-obvious and
got resolved correctly), and confirmed "hash" as a general term (not a
formula) is the right line to draw given the Reversal section's
birthday-paradox reasoning depends on the reader knowing the scheme is
content-derived and scattered.

## 2026-09-02 — delivery-profile.md retrofitted and approved; M3 role binding changed

Considered: M1's scoping predated (or skipped) the scoping stage's item 7,
`delivery-profile.md` — no such file existed going into M3. Options: skip it
and decompose M3 straight from the spec tree, or retrofit it now per the
scoping stage's own definition-of-ready. User chose to retrofit formally.
Chosen: advisor drafted `delivery-profile.md` from supplied facts (signed-off
spec tree, requirements doc, the Mosaic layout spike + BRIEF-v2, T01's
identity-spike evidence, and a fresh scan of every `[REVIEW:]`/OPEN marker).
User approved all four of advisor's amendments to the conductor's initial
v1-cut proposal:
1. Card content scales continuously with tile size (`layout.md` R5.3), not
   fixed at 3 lines — reverses the conductor's initial lean, which had the
   requirement dates backwards (R6.3 is 2026-09-01, R5.3 is CONFIRMED
   2026-09-02) and missed that R5.10's provenance note records fixed-3-line
   cards were already built and rejected by real-render evidence (blank
   space below the text in a larger proportional box). R6.3 is read as
   governing the compact regime's content/order, not a global line count —
   both requirements survive; `visuals.md`'s open tag closes.
2. Chrome (R6.2) and minimum tile sizes (R5.5) are already answered by the
   verified borderless Mosaic build and BRIEF-v2's regime table respectively
   — dropped from v1 work as new decisions, kept as confirmation passes only.
3. Session zoom (R7.1) is cut from v1 entirely, not shipped as a "confirmed
   subset" as the conductor first proposed — no prototype exists for it, it
   needs a second full-screen view and its own fetch/navigation/escape
   semantics, and it doesn't serve R7's own "at a glance" purpose. Enter is
   unbound or shows "not yet" in v1; deferred with trigger "user finds
   themselves needing detail the tile cannot carry."
4. R1.7 (staleness) splits: display treatment defers, but claim *release* on
   explicit session-gone does not — R6.8's claim-map depends on it, and a
   session vanishing without release would hold its name/category forever.
Also changed, same sitting: **M3's implementer/reviewer role binding**
switches from `coder`/`ask_opus` (M1-M2) to opencode-hosted agents —
implementer = opencode `deepseek` agent, reviewer = opencode `glm-5.3` agent,
both dispatched via `mcp__opencode-bridge__opencode_task` with the `agent`
param (never `model`), `wait: true` so the refine-loop stays synchronous
despite the bridge's async default. `runner` stays `coder`; `advisor` stays
Opus.
Why: user's explicit choice, both for the delivery-profile retrofit and for
trying the cheaper/faster opencode agents on real implementation work now
that the spike round they were originally proposed for was cut for being
too much process without a running product.
Limitations: the opencode implementer/reviewer pairing is **unproven on this
repo's Rust code** — user asked for a one-task trial (M3's first task, the
Cargo workspace migration) before treating it as M3's standing binding. If
the trial's gate report is weak, the binding reverts to `coder`/`ask_opus`
for the remainder of M3 without further discussion.
Reversal: if the trial task's refine-loop produces a weak gate report (poor
Rust quality, reviewer missing real defects, or the async-dispatch/`wait`
mechanics proving unreliable), revert to `coder`/`ask_opus` for M3's
remaining tasks. If a capacity/scale assumption in delivery-profile.md breaks
(R5.8's ~8-session design center, or the case-preserving-filesystem
assumption behind the R1.6 deferral), reopen this profile rather than
patching around it silently.
Sign-off: advisor drafted the profile and proposed the four amendments;
user approved all four plus the role-binding change. Scoping definition-
of-ready item 7 (delivery profile approved) and the roles portion of item 5
now both hold for M3.

## 2026-09-02 — T08 opencode implementer trial: failed, reverted to coder/ask_opus

Considered: whether `crof/deepseek-v4-flash` (opencode `deepseek` agent) is
viable as M3's implementer on a task the size of the Cargo workspace
migration.
Chosen: no — reverted to `coder`/`ask_opus` for T08 and, absent a specific
reason to retry on a smaller task, for the rest of M3.
Why: dispatched twice with an identical brief (fresh session both times).
Both attempts stalled with zero filesystem change — no `crates/` directory,
no `cargo build`/`test` run, `git status` unchanged outside pre-existing
untracked files — for ~58 minutes and ~26 minutes respectively, while
cost and reasoning-token counters kept climbing. Confirmed independently
via `opencode_sessions` polling both times (not a lookup mismatch). This
is a model-capability finding, not a dispatch/polling-plumbing problem —
the dispatch, poll, and cancel mechanics all worked correctly; the model
just never converged on producing filesystem-touching tool calls. Reviewer
(`glm-5.3`) was never reached — nothing existed to review either time.
Process note: T08's runner (`runner-t08`) deviated once from an explicit
instruction mid-incident — told to cancel and report back rather than
redispatch, it started a second identical dispatch on its own initiative
before reporting. Caught and corrected immediately (the runner cancelled
the unauthorized retry on request and reported cleanly afterward); flagged
to the user as a real instruction-following lapse, not silently absorbed.
Limitations: this only tests one opencode model (`deepseek-v4-flash`) on
one task shape (a large, judgment-heavy multi-file restructure). It says
nothing about smaller/more mechanical M3 tasks, nor about other opencode
agents (e.g. `deepseek-v4-pro`, `glm-5.3` itself as an implementer) — those
remain untested.
Reversal: if a future M3 task is small/mechanical enough that a stalled
attempt costs little, and there's a specific reason to believe an opencode
agent would do better there (e.g. a task that's mostly prose/config, not
multi-file structural judgment), it's fair to retry the experiment narrowly
— but as a new, explicit decision, not a default.
Sign-off: conductor decision, made under the pre-approved reversal trigger
already recorded above ("weak gate report → revert... without further
discussion") — this outcome (no gate report at all, twice) meets that bar
more clearly than the trigger's original wording anticipated. Reported to
advisor and the user; T08 proceeds now with `coder` as implementer.

## 2026-09-02 — T08 pass-2 completion misrouted to top-level session, not the runner

Considered: (a) accept runner-t08's gate report as written (pass 2 "did not
return", gate closed on pass-1 + runner's own re-verification only), (b)
correct the record once the actual pass-2 output surfaced.
Chosen: (b). The pass-2 reviewer (`ask_opus`, same agent, resumed via
`SendMessage`) did complete — its finished output landed in the top-level
coordinator's session, not runner-t08's, so runner-t08 genuinely never saw
it and wasn't wrong to treat it as non-responsive from where it sat. Verdict
received directly: fresh scan clean, all 9 acceptance criteria pass, fmt fix
confirmed as the only change, one deferred stale-comment note carried
forward, conformance yes. Runner-t08 asked to append a correction to
`gates/T08-report.md` rather than leave the "reviewer never returned" framing
standing, since that's not what happened.
Why: this is the same misrouting pattern the opencode-dispatch trial
surfaced earlier this run (a completion notification for a subagent spawned
by a sub-agent arrived at the top-level session instead), now confirmed for
a second, different transport — nested Agent-tool subagent completions via
`SendMessage`-resume, not just MCP dispatch callbacks. Two independent
transports, same failure shape: completion visibility follows the process
tree the harness tracks, not the logical spawner.
Limitations: only observed twice, both times with the top-level coordinator
as the wrong recipient; whether it's deterministic (always routes to the
root) or occasionally correct is unknown — no fix attempted, this is a
record of the behavior, not a mitigation.
Reversal: any runner in this run (or a future one) waiting on a nested
subagent's response should ping the conductor if the wait looks anomalously
long relative to a comparable prior turn (as runner-t08 correctly did here)
rather than conclude non-response on elapsed time alone — the conductor may
be holding the answer already. Revisit if this recurs a third time; at that
point it's not a one-off routing quirk, it's a property of the harness worth
raising outside this run.
Sign-off: conductor decision, record-accuracy correction — no scope or
binding change, T08's actual result (conformance yes, zero new pass-2
findings) is unchanged and was never in doubt once the real output was in
hand.

## 2026-09-02 — M3 remainder decomposed: T09-T12, straight pipeline

Considered: fan out any pair of T09-T12 (naming/claim-map and Mosaic
render both looked plausibly independent of each other) vs. a straight
pipeline with no fan-out.
Chosen: straight pipeline — T09 (`HarnessAdapter` boundary + core
session/project model + opencode adapter) → T10 (R6.8 naming/claim-map) →
T11 (Mosaic layout/render, promoted from the verified spike) → T12
(interactive shell: main loop, terminal lifecycle, window controls,
keyboard nav minus zoom). T09 bundles the boundary trait with its one
implementation rather than splitting them into separate tasks.
Why: T10 and T11 both consume types T09 defines (session/project identity,
creation time, tombstone signal, snapshot content fields); parallelizing
either against a guessed shape of those types risked the same kind of
integration gap the advisor caught between `client.md` R1.4 and
`visuals.md` R6.8 at M2 sign-off. On T09's internal shape: advisor's
review — the trait and its only implementation are co-designed by
necessity (you can't verify the boundary is right without an adapter that
implements it), so splitting would create an artificial seam and risk the
same integration-gap failure mode the pipeline choice is meant to avoid.
Limitations: this is a 4-task, single-threaded critical path through the
rest of M3 — no wall-clock parallelism available until T09 gates. If T09
runs long, nothing else in M3 can start.
Reversal: if a later resume finds T10's or T11's actual shape needs were
smaller/more stable than expected once T09 is built, a future run could
still choose to fan out remaining work differently — this decision governs
this run's four sealed contracts, not a permanent constraint on how
dashboard work must always be sequenced.
Sign-off: advisor, contingent on two contract fixes applied before
sealing — T09 (v2): named a dedicated last-updated timestamp field for
R3's window filter, distinct from the per-state elapsed-time basis. T12
(v2): added AC 9, tombstone-to-claim-release wiring (T09's tombstone →
T10's release), the one place those two signals meet at runtime. Both
fixes applied; advisor confirming Review Frames hold at v2.
