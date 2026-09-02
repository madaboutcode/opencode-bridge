<!-- Instantiated at scoping (retrofitted 2026-09-02, before M3 decomposition).
     Drafted by the advisor from conductor-supplied facts; the user approves
     material release posture. Referenced by path, never pasted into spawn
     prompts. -->

# Delivery Profile — opencode-dashboard

**Status** — user-approved · version: 1 · approved: 2026-09-02, by Ajeesh (all four amendments accepted as drafted)

**Source facts** — the signed-off M2 spec tree (`docs/specs/dashboard/{overview,client,layout,visuals,interactions}.md`); the requirements doc (`tasks/2026-09-01-opencode-dashboard.requirements.md`); the verified Mosaic layout spike and its build contract (`tmp/20260901-prototype-dashboard-layout/`, `BRIEF-v2.md`, render evidence in `renders/`); the T01 project-identity spike evidence (`tmp/2026-09-02-project-identity-spike/EVIDENCE.md`); a conductor scan of every `[REVIEW:]`/OPEN marker across the five spec files, 2026-09-02.

**Release context** — stage: initial build (v1, first running version; not hardening) · users and scale: one user, one local machine, personal development tool — a TUI watching that user's own concurrent coding-agent sessions across projects. Concurrency is **specified, not inferred**: `overview.md` R5.8 (CONFIRMED) sets the design center at ~2 sessions per project, ~4 projects, ~8 sessions total. Stress scale must not break badly, but is explicitly not the design target · purpose: prove the harness-adapter boundary, the Mosaic layout, and the naming scheme render and update correctly against a real opencode server, end to end.

**Supported workflows** — the ordinary paths this release must make dependable:

1. **Cargo workspace migration.** Today's single published crate (`opencode-bridge`) becomes a workspace with `crates/opencode-client`, `crates/dashboard`, and the existing MCP binary (R1/R1.1). The release pipeline and `Cargo.toml` `exclude` rules are in this workflow's blast radius, and the files it moves carry pre-existing uncommitted changes.
2. **The adapter boundary itself exists, and opencode goes through it.** Not merely "one adapter is enough for v1" — the core must not import opencode wire shapes directly (R1.3). This is the foundation whose absence would be disproportionately expensive to retrofit, and it is the thing most likely to be quietly skipped under time pressure because a direct wiring is shorter. One adapter, opencode, per R4/R6.4-R6.6.
3. **Session snapshot ingestion → Mosaic render.** Adapters push whole-session snapshots on one channel (R1.4); the core buckets by project identity (R1.6, spike-confirmed) and recomputes the two-level squarified layout every frame (R5, R5.1, R5.2).
4. **Tile content scales with tile size** — the continuous regime of `layout.md` R5.3 and BRIEF-v2's regime table, from colour-only through compact through extended. **See Amendment 1: this reverses the conductor's proposed "3 lines fixed."**
5. **Attention model renders correctly** — `running / needs-you / idle`, with the needs-you question sub-state driven by the opencode adapter's own heuristic (R6.7, R1.3).
6. **Naming scheme ships as specified** — two-layer claim scheme, both hard guarantees, creation-time-ordered claim resolution, cooldown (R6.8).
7. **Window controls** — `W` adjustment and the active-window filter (R3, R8).
8. **Keyboard navigation across tiles** — arrow/`j`/`k` movement and selection (R7.1 subset). **Zoom is not in this list — see Amendment 3.**

**Release bar** — a finding is release-critical if it breaks any supported workflow above at the stated scale; leaves visible state wrong under ordinary operation (a session shown running that ended, a session missing from its project, two identical names on screen); risks irreversible loss or corruption of the user's own work (the workspace migration is the only workflow with that exposure — it moves files carrying uncommitted changes and rewrites the release pipeline); or removes a foundation that is disproportionately expensive to retrofit. Exactly two foundations qualify: the adapter boundary (workflow 2) and identity-keyed claim-map ownership at the core (`client.md` R1.4's exception). Terminal restore on panic also sits at this bar — a TUI that leaves the terminal in raw mode on crash is a user-visible break, and R2 already requires the Drop/panic guard.

**Deferral posture** — credible but bounded, out of supported use, or belonging to later scale:

- **Second-harness adapter (R1.3-R1.8).** Not a deferral. The spec's own v1 boundary is one adapter; the genericity is already in scope as the boundary itself (workflow 2). Assumption: no second harness is wired in v1.
- **Case-sensitive path collision (R1.6).** T01 could not exercise it — the build machine is case-preserving. Assumption: one user, one machine, paths of consistent case. Trigger: a case-insensitive-but-not-preserving volume, or two project boxes appearing for one repo.
- **Per-tool action-line formatting beyond `shell`/`edit` (R6.6).** The spec already defers this itself; `running: <name>` is the specified fallback. Assumption: a generic fallback line is readable enough at v1.
- **50+ session overflow (R5.6, R8's `a` key).** Outside the confirmed design center (R5.8). Assumption: the user does not routinely run dozens of sessions. Consequence if wrong: tiles breach minimum size and the screen becomes unreadable — degraded, not corrupt. Trigger: the user reports the `a` view as unusable.
- **R6.8 capacity edge case** — more live projects than categories, or more live sessions than a category has words. Assumption: ~4 projects against 10 categories, ~2 sessions against ~10 words. Consequence: a duplicate name, i.e. the guarantee silently fails. Trigger: either count approaching its list length.
- **Question-badge heuristic phrase list (R6.7).** Ships with a minimal rule; refinement deferred. Assumption: false negatives (a question not badged) are tolerable; the session still shows as `needs-you`.
- **R1.7 staleness — display treatment only.** See Amendment 4: the *display* of staleness defers; *claim release* does not.
- **Session zoom / full-screen trace (R7.1).** See Amendment 3.

## Finding Disposition

- **Correct now** — evidenced defects against the release bar, plus unapproved implementation scope beyond the supported release.
- **Preserve foundation** — keep the minimum seam needed to avoid disproportionate retrofit cost; record the constraint in `decisions.md`, but do not build the future behavior.
- **Defer with trigger** — append a credible, bounded concern to `deferred.md` with scenario, consequence, assumption, and promotion trigger.
- **Reject** — do not turn scenario-free hypotheticals or alternative design preferences into known issues.

Frequency alone never decides disposition. A reviewer may challenge this profile with evidence that a real workflow or non-deferrable consequence was misdrawn. Preserve contested classifications in the gate report; do not silently downgrade them.

**Amendment** — the advisor may propose changes, but material changes to supported workflows, risk posture, or non-deferrables require user approval and a `decisions.md` entry naming affected tasks and deferrals.

---

## Amendments proposed in this draft (user approval required)

**Amendment 1 — the R6.3/R5.3 line-count conflict resolves toward continuous scaling, not fixed 3 lines.** The conductor's lean was "3-lines-wins, since `visuals.md` is more recent." The file is more recent; the requirement is not. R6.3 is dated 2026-09-01, R5.3 is CONFIRMED 2026-09-02. More decisively, R5.10's provenance note records that fixed-3-line cards were actually built and rejected by render evidence — inside a proportional box they left blank space below the text (`shots/C-real-wide.png`, pre-fix). Shipping fixed 3 lines would re-adopt a design that already failed the one test that was run against it. Reconciliation, which is not a coin flip: R6.3 specifies the **content and priority order of the compact regime** (nickname + title, status + elapsed, current action) — it is not a global line count. Both requirements survive with that reading; `visuals.md`'s `[REVIEW:]` tag can be closed.

**Amendment 2 — two proposed workflows are already answered and should not be re-opened as v1 work.** R6.2's chrome axis (the A/B/C bake-off) was settled by the verified Mosaic build, which is borderless: tile background carries state, the project accent sits on the name tag only (R5.11), and nickname placement follows as shared line 1. Don't spend a v1 decision re-picking it. Similarly, R5.5's "recheck minimum sizes against the 3-line card" is largely already answered by BRIEF-v2's regime table, which defines behavior at every `(width, height)` including the small cells. Keep a confirmation pass; drop the re-derivation.

**Amendment 3 — session zoom (R7.1) should be cut from v1, not shipped as a "confirmed subset."** It is the largest hidden cost in the proposed list and the only supported workflow with no prototype behind it: the Mosaic spike built no zoom view. It needs a second full-screen view, a per-session message fetch path that nothing else in v1 uses, and its own navigation and escape semantics — and `interactions.md` already carries a `[REVIEW:]` on what Enter even zooms. R7's stated purpose is seeing everything *at a glance*; zoom is the one item that doesn't serve it. Recommend: v1 ships navigation and selection, Enter is unbound or shows a "not yet" affordance, and zoom becomes a deferral with the trigger "the user finds themselves needing detail the tile cannot carry."

**Amendment 4 — R1.7 is mis-classified; its two halves have different postures.** The conductor proposed deferring R1.7 wholesale behind a "dim after N seconds" treatment. The *display* half is genuinely deferrable — v1's single adapter has a 60s REST reconcile sweep (R4) that makes silent staleness unlikely. But `visuals.md` R6.8's claim-map now depends on R1.7 for claim release, and a session that vanishes without a tombstone holds its word and its project's category forever. So v1 must define claim release on explicit session-gone (already specified) and must not ship a claim-map with no release path at all. Staleness-*triggered* release can defer; releasing on tombstone cannot. Stating it here so an implementer doesn't read "R1.7 deferred" as "claims never release."

**Note, not an amendment** — the concurrency figure the conductor flagged as inference is a confirmed requirement (R5.8). The profile above states it as fact.
