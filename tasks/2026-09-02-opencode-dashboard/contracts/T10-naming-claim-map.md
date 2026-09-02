# T10 — Naming and claim-map (R6.8)

**Contract version** — 1

**Context** — goal: implement `visuals.md` R6.8's two-layer naming claim
scheme (project→category, session→word) as a pure, testable module — no I/O,
no TUI · who uses it: T11's render layer reads the assigned nickname per
session and category per project; T09's/T12's tombstone path calls this
task's release function when a session or project goes gone · scale: 10
categories/60 words frozen (Appendix, `visuals.md`), `overview.md` R5.8
design center (~4 live projects, ~2 sessions/project) · criticality:
moderate-high — two guarantees are stated as "hard, not just
low-probability" and this is one of the delivery profile's 8 supported
workflows, but the module itself is pure logic: reversible, fully unit
testable, no network/filesystem/render surface.

**Delivery profile** — `../delivery-profile.md` version 1 · task override:
none.

**Boundaries**

- **Owns:** a new module under `crates/dashboard/src/` for the claim-map
  (project→category, session→word, cooldown bookkeeping, claim-order
  resolution) and the frozen word lists, sourced verbatim from `visuals.md`'s
  Appendix (10 categories, ≤10 chars/word, no word repeated across
  categories — copy exactly, this is frozen data, not a draft).
- **Must not touch:** T09's adapter/model types (read-only consumer of
  `SessionIdentity`/`ProjectIdentity`/creation-time — do not redefine them
  here), T11's render code (doesn't exist yet), `docs/specs/**` (the word
  lists are copied from there, not edited there).

**The scheme (R6.8, restated for implementation)**

- **Project → category.** Each currently-live project (≥1 live session)
  claims exactly one category, exclusively, for as long as it has a live
  session. Preferred category is derived deterministically from project
  identity (same identity always prefers the same category). A conflict
  (preferred category already claimed by a different live project) resolves
  to another unclaimed category by a fixed rule — never randomly. Released
  when the project has no live sessions left.
- **Session → word.** Within its project's claimed category, each session
  claims exactly one word. Preferred word is derived deterministically from
  the session-identity tuple (`client.md` R1.5), scattered across the whole
  category list rather than always starting from word 0. A conflict (word
  held by a live sibling in the same project, or in cooldown) resolves
  deterministically to the next word that is both free and off cooldown.
- **Claim order is pinned, not incidental.** Batched claims (most commonly at
  dashboard startup) resolve in ascending order of actual creation time — a
  project's position is its earliest live session's creation time, a
  session's position is its own creation time — never wire-arrival order
  (REST pagination, SSE delivery order). This is what makes the same live
  set produce the same names across two restarts.
- **Cooldown (recycling).** When a session ends, its word does not return to
  the available pool immediately — enough *other* distinct words in that
  category must be claimed first. The exact cooldown rule/count is not
  pinned by the spec; pick one, document it in the gate report, and make it
  testable (e.g. "N other distinct words claimed since" for some documented
  N). This is a real implementation judgment call, not a spec gap to guess
  around silently.
- **Guarantees, both hard:** no two sessions in the same live project ever
  show the same name; no two live projects ever show the same name. Both
  hold only under two capacity assumptions, neither checked at runtime: live
  projects never exceed curated categories; no word is duplicated across
  category files. Both are the word-list curator's responsibility (i.e.
  already satisfied by the frozen Appendix data this task copies verbatim).

**Coupling to T09's tombstone (R1.7 coupling, not yet resolved on the
staleness side)** — both claim layers must release their claim when T09
signals a session or project is gone (the tombstone path T09 built, not a
staleness-threshold rule — that part is still `[REVIEW: OPEN]` and out of
scope here too). Wire the release call to T09's tombstone signal; do not
invent a staleness threshold to trigger it early.

**Capacity edge case (`[REVIEW: OPEN]` in the spec)** — what happens when a
capacity assumption breaks (more live projects than categories, or more live
sessions in one project than that category has words) is explicitly
undecided. A numeric-suffix fallback is spec-acceptable but not required;
what *is* required is that this task never panics or silently violates a
hard guarantee when capacity is exceeded — degrade visibly (e.g. a
duplicate-suffix name) rather than crash. Document the chosen behavior; this
stays a deferred item per the delivery profile, not a task blocker.

**Conventions** — `cargo build/test/clippy/fmt` per `CONTRIBUTING.md`, same
commands as T09. Pure-logic module: prefer deterministic unit tests over
anything requiring wall-clock time or real I/O.

**Skills to read and apply** — `code-quality`, `writing-unit-tests` (this
module's whole value is in its guarantees — the tests are the spec's proof,
not an afterthought).

**Acceptance — done when:**

1. Both claim layers implemented exactly as specified above, using T09's
   `SessionIdentity`/`ProjectIdentity`/creation-time types as input — no
   redefinition of those types.
2. A test proves claim order is creation-time-ordered, not
   input/arrival-ordered: feed the same live set in two different arrival
   orders, assert identical resulting claims both times.
3. A test proves both hard guarantees at the design-center scale (R5.8: ~4
   projects, ~2 sessions each) and at a stress case up to the capacity
   assumptions' boundary (10 projects / 10 categories; a category filled to
   its word count).
4. A test proves cooldown: a word freed by an ended session is not
   immediately reclaimed by a new session that prefers it; it becomes
   reclaimable once the documented cooldown condition is met.
5. A test proves release on tombstone: a project/session marked gone via
   T09's tombstone signal frees its category/word for reuse.
6. A test proves the capacity edge case degrades (documented fallback, no
   panic) rather than crashing or silently breaking a hard guarantee.
7. The 10-category/60-word Appendix is reproduced verbatim; a test asserts
   no word repeats across categories.
8. `cargo build/test/clippy/fmt` clean workspace-wide; nothing outside this
   contract's owns-list touched.

**Gate** — report-only (refine-loop).

**Dependencies** — T09 (session/project identity types, creation time,
tombstone signal).

## Review Frame

*Authored by the advisor. Governs disposition and review budget — never what
the reviewer may look at or discover. It cannot suppress credible severe
evidence.*

**As of** — contract version 1

**Context** — Pure logic module, fully testable. Implements the naming scheme's
two hard guarantees — both stated as non-probabilistic.

**Expectations** — Claim-order creation-time-ordered, not arrival-ordered
(AC 2). Both guarantees hold at design-center and capacity boundary (AC 3).
Tombstone release frees claims (AC 5). Overflow degrades visibly, never panics
or breaks a guarantee silently (AC 6).

**Depth** — Guarantees and edge cases are the whole value; review closely.
Cooldown constant and word-list format are implementation choices — out of
budget. 2 passes.
