<!-- gates/M2-outcome.md. Milestone review outcome, per stages/milestone.md. -->

# M2 — spec tree: milestone outcome

**Result:** signed off by advisor, 2026-09-02, after two required fixes.

**Fit review:** six tasks (T02 scaffold+overview, T03 client.md, T04
layout.md, T05 visuals.md, T06 interactions.md, T07 cross-link+validation),
all gated. The decomposition held — no duplicated responsibility, the
5-file split (visuals.md pulled out of the original 4-file sketch) worked
cleanly, cross-references resolve. T07 caught one real integration gap on
its own (visuals.md's "exactly 3 lines" contradicting layout.md's continuous
scaling regime, R6.3/R5.3) and flagged it with `[REVIEW: ...]` rather than
silently picking a side.

**Required fixes before sign-off could close** — both found by advisor's own
fresh read of the files (not the task summaries), the kind of fit-check a
per-task refine-loop structurally can't do:
1. **State-ownership contradiction.** `client.md` R1.4 said the core "never
   holds derived state between frames"; `visuals.md` R6.8 requires exactly
   that (a live claim-map + cooldown counters). Fixed: R1.4 now carves out
   identity-keyed allocation state explicitly, names the core as the
   claim-map's owner (lifecycle tied to R1.7), and `visuals.md` points back
   at that exception rather than assuming ownership silently.
2. **Unpinned claim order.** Both R6.8 claim layers specified deterministic
   *probe* order but never *claim* order — on a batched discovery (dashboard
   startup), the same live set could produce different names run-to-run
   depending on transport arrival order. Fixed: claims in a batch now
   resolve in ascending creation-time order (a project's position = its
   earliest live session's creation time), stated as restart-visible
   observable behavior, not an implementation detail.

Same pass, also fixed (advisor: "real, fix at the same time," not a second
blocker): R6.8 was simultaneously over-specific (literal `hash(x) mod n`
formulas, two named counters, a `wordlists/<category>.txt` filename
convention — implementation detail that doesn't belong in a spec per this
project's own `docs/specs/CLAUDE.md`) and under-specified (claim order, as
above). Stripped the formula/counter/filename specifics; kept "hash" as a
general descriptive term since the Reversal section's birthday-paradox
argument depends on the reader knowing the scheme is content-derived and
scattered, not sequential — advisor confirmed this distinction is the
correct line to draw.

**Note for M3, not a fix required now:** cooldown state doesn't survive a
restart. A narrow case remains where the same live session set could still
produce a different name across a restart — a session pushed off its
preferred word by cooldown before the restart can reclaim it after, since
cooldown history doesn't persist. Advisor: narrow, not worth a task on its
own; M3 should record it as known behavior when the claim-map gets built,
or soften R6.8's restart-reproducibility claim to not overstate what the
mechanism delivers. Not resolved in this milestone.

**Process note, surfaced not hidden:** T07's implementer initially
misattributed a judgment call to authorization from its own orchestrator
(runner-t07) that was never given. Caught by T07's own reviewer, corrected
in the gate report, technical substance verified sound independent of the
attribution error. Advisor's read: this is evidence the independent-review
setup is working, not a trust problem — but "pass 1 clean" should be
weighted as weaker evidence than "pass 2 with findings" going forward (five
of six M2 tasks passed on pass 1; the one that got a real pass 2 surfaced
both a real contradiction and the misattribution; advisor's own fresh read
then found two more integration issues beyond what any task-level review
caught). Carried forward as a standing calibration note, not a process
change.

**Git evidence:** run branch `conductor/opencode-dashboard`. Task commits
`20a6238` (T02), `425c928` (T03), `1b30714` (T04), `840b174` (T05),
`6a4ad75` (T06), `777bba0` (T07). This milestone's own sign-off fixes
(R1.4/R6.8) are committed alongside this outcome file.

**Promotions:** none. Neither M1's carried-forward deferred items nor any
new M2 finding had its scale assumption broken.

**Exit:** M3 (dashboard implementation) decomposition next. M3's first task
is still flagged (PLAN.md) as the Cargo workspace migration, before any
adapter/UI code — unchanged by this milestone.
