<!-- gates/M3-outcome.md. Milestone review outcome, per stages/milestone.md. -->

# M3 — dashboard implementation: milestone outcome

**Result:** fit review complete, fit holds. Advisor sign-off pending.

**Fit review:** five tasks (T08 workspace migration, T09 adapter boundary,
T10 naming/claim-map, T11 Mosaic promotion, T12 interactive shell), all
gated, all pass 1 clean except T08 (2 passes, licensed fmt fix). A fresh
reviewer with no prior involvement in any task traced every inter-task seam
by reading the actual code, not the gate reports' claims:

- T09→T10: `naming/claim_map.rs` consumes T09's real `ProjectId`/
  `SessionId`/`Timestamp` types directly, no redefinition, no glue layer.
- T09/T10→T11: `mosaic/view.rs` reads T09's snapshot fields and T10's naming
  output correctly; the elapsed-time-computed-at-render-time rule holds
  (proven by a test rendering the same snapshot at two `now` values); both
  of T11's own self-flagged cross-task items (R6.5 basename limitation,
  subagent nickname fallback) confirmed correctly scoped.
- T09/T10/T11→T12 (the integration seam): all three decisions.md-recorded
  cross-task fixes confirmed *built*, not just decided — tombstone-to-release
  wiring (`shell/live.rs:76-79`, proven by test), subagent claim wiring
  (`shell/live.rs:67-69`, proven by test — a subagent fixture gets a real
  claimed name, not the id fallback), and R3 active/idle reclassification
  closing T09's own deferred gap (`shell/reclassify.rs:25-40`, computed from
  `last_updated` against the window, never from an opencode-native field).
- Module wiring (`main.rs`, `lib.rs`) follows the intended startup order
  (connect → adapter → shell) with no orphaned module.
- No duplicated responsibility found (elapsed time, attention-state
  construction, and claim logic are each single-owned).
- No deferred item was left unpromoted that should have been — the four
  cross-task deferrals raised during execution (T09's R3 gap, T09's R6.5
  gap, T11's subagent-nickname gap, T12's SIGTERM gap) each resolved either
  by a later task's wiring or by a correctly-scoped deferral.
- Spot-checked delivery profile workflows 2 (adapter boundary), 3 (snapshot
  → render), and 6 (naming scheme) directly against the code — all three
  hold as built, not just as claimed.

**One finding from the fit review, appended to `deferred.md`** (not a task
defect, a milestone-level observation): `NamingClaimMap`'s
`#[derive(Default)]` (T10, `naming/claim_map.rs:177`) produces an empty
category list that panics on first claim — no real code path reaches it
today (the only construction site, `LiveState::new()`, correctly uses
`NamingClaimMap::new()`, and its doc comment already flags this), but the
derive itself is a footgun for any future construction site. Deferred with
trigger: any new code path constructing `NamingClaimMap` should remove or
fix the derive first.

**Verdict on decomposition:** the T09→T10→T11→T12 pipeline (no fan-out) was
the right cut — see `decisions.md`'s "M3 remainder decomposed" entry for the
reasoning at decomposition time. Built, it held: each task owns exactly one
concern (data/adapter, naming, layout/render, wiring), and the one genuinely
ambiguous call — whether R3's active/idle reclassification belongs in the
adapter (T09) or the shell (T12) — landed correctly in T12, since the
keyboard-adjustable window `W` is a core/shell concern the adapter has no
visibility into (documented in `snapshot.rs`).

**Git evidence:** run branch `conductor/opencode-dashboard`. Task commits
`ac6962b` (T08, + correction `aa87c2f`), `d4e0432` (T09), `7c43858` (T10),
`4cb8a3a` (T11), `2c00bdd` (T12). Decomposition-fix commits `3d437f0` (T09-T12
sealed) and `2206e7d` (T12 v3, subagent claim wiring). This outcome file and
its `deferred.md` addition are committed alongside milestone sign-off.

**Promotions:** none. No deferred item's recorded assumption broke, and
none is load-bearing for work this run still has ahead of it.

**Open item carried to the user, not a gate blocker:** T12's AC8 manual
smoke test was recorded (exact steps) but not executed end-to-end on a real
terminal — the implementer's sandbox had no TTY, so only the connect/pairing
half was exercised live (paired successfully, then failed cleanly with no
TTY available, no panic, no hang). The interactive half (keys, terminal
restore, live rendering against a real opencode server) still needs a human
running it on a real terminal before this ships.

**Exit:** M3 was PLAN.md's last named milestone. If the advisor signs off,
this run has no further milestone to decompose — remaining work is the
carried-forward `deferred.md` backlog (14 items) and the manual smoke test
above, not new M-numbered work.
