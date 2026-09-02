# T08 — Cargo workspace migration

**Contract version** — 1

**Context** — goal: turn today's single published crate (`opencode-bridge`)
into a Cargo workspace with a shared client library and two independent
binaries, per `docs/specs/dashboard/overview.md` R1/R1.1 — no behavior change
to the existing MCP binary, no dashboard logic written yet · who uses it:
every later M3 task builds on this shape; the existing MCP binary's users
(crates.io / GitHub release consumers) must see no functional change · scale:
one repo, ~10 source files, one CI workflow · criticality: **high** — this is
the one task in the run with irreversible-loss exposure (it moves files) and
release-pipeline blast radius (rewrites the tagged-release workflow of a
published crate). Treat it as such: no shortcuts on the acceptance checks
below.

**Delivery profile** — `../delivery-profile.md` version 1 · task override:
none. Per the profile's release bar, the adapter-boundary and workspace
foundations are exactly the kind of thing "disproportionately expensive to
retrofit" — build the shape right the first time rather than fast.

**Boundaries**

- **Owns:** root `Cargo.toml` (converted to a workspace manifest);
  `crates/opencode-client/**` (new — the shared library); `crates/dashboard/**`
  (new — skeleton binary only, see Acceptance); `crates/opencode-bridge/**`
  (the existing MCP binary, moved here from `src/`); `.github/workflows/ci.yml`.
- **Must not touch:** `docs/**`, `tasks/**` (other than this contract's own
  gate report), `tmp/**`, `README.md`'s product description beyond paths that
  actually moved, anything not named above. Do not create `docs/specs/`
  entries or edit the spec tree — this is an implementation task, not a spec
  task.
- This breaks the "owns-list never touches pre-existing work" pattern every
  prior task in this run used, on purpose: the pre-existing uncommitted work
  in `src/main.rs`/`notify.rs`/`opencode.rs`/`sse.rs`/`tools.rs`/`log.rs` is
  now committed (`fe9c61b`) — moving it is `git mv` on tracked history, not a
  clobber of someone's in-progress diff.

**The split (per R1.1)** — R1.1 scopes the shared library to "authentication,
plus the session-list, session-message, and event-stream calls." Concretely:

- `crates/opencode-client/`: the `opencode.rs` `Client` (pairing, health,
  session/message HTTP calls) and the *raw* SSE connection/parsing primitive
  from `sse.rs` (connect to `/api/event`, decode frames into typed events).
  No TUI dependency, no MCP dependency (R1.1's own acceptance test).
- `crates/opencode-bridge/` (the existing MCP binary, renamed from its
  current root location): `main.rs`, `mcp.rs`, `tools.rs`, `state.rs`,
  `registry.rs`, `notify.rs`, `log.rs`, `error.rs`, and the MCP-specific half
  of `sse.rs` — the tracked-session registry glue, `complete_session`, and
  the CC-callback notify call that consumes the shared crate's raw event
  stream. This crate keeps the `opencode-bridge` package name, its existing
  crates.io metadata (`description`, `license`, `repository`, `readme`,
  `keywords`, `categories`, the `exclude` list), and its existing 29 tests —
  all must still pass, none should need behavioral changes, only import-path
  updates.
- `crates/dashboard/`: skeleton only — a binary crate that builds and runs
  (e.g. prints a "not yet implemented" line and exits 0), depending on
  `opencode-client`. No TUI dependency added yet, no session-fetching logic.
  Later M3 tasks fill this in.

If the exact sse.rs split is ambiguous once you're in the code, prefer
duplicating a thin re-export over inventing a third crate or a feature flag
— this is a happy-path migration, not a new abstraction layer.

**Conventions** — `cargo test`, `cargo clippy --all-targets -- -D warnings`,
`cargo fmt --all -- --check` (per `CONTRIBUTING.md`). **Formatting rule for
this task specifically:** moved files keep their current formatting in the
move commit(s) — do not run `cargo fmt` over inherited code as part of the
migration diff. If new code in `crates/dashboard`'s skeleton or the crate
boundary itself needs formatting, that's fine; a blanket reformat of ported
logic is a separate follow-up commit, never folded into the structural
change. This keeps the migration diff reviewable as "what moved" versus
"what changed."

**Skills to read and apply** — `code-quality`, `software-design` (real
component boundary being drawn here — the client/bridge split is exactly
what that skill's "layer stack" section is about).

**Acceptance — done when:**

1. `cargo build --workspace --all-targets --locked` succeeds.
2. `cargo test --workspace --all-targets --locked` passes — the existing 29
   tests, now living under `crates/opencode-bridge`, all still pass unchanged
   in substance (import-path updates only).
3. `cargo clippy --workspace --all-targets -- -D warnings` is clean.
4. `cargo fmt --all -- --check` is clean for every file in this task's
   owns-list (moved files carry their pre-migration formatting forward
   unchanged, so this should already hold; new skeleton files must be
   formatted).
5. `crates/opencode-client` has zero TUI and zero MCP dependencies in its own
   `Cargo.toml` (R1.1's literal acceptance test).
6. `cargo build --release --locked` still produces a binary at
   `target/release/opencode-bridge` — if the workspace restructure changes
   this path, `.github/workflows/ci.yml`'s "Name release binary" step is
   updated to match, and the diff explains why.
7. `.github/workflows/ci.yml` is updated for the workspace: `cargo package
   --list` needs a `-p opencode-bridge` (or run from that crate's directory)
   since a bare `cargo package` doesn't resolve unambiguously in a workspace;
   the internal-files check (`docs/internal|tasks/`) still runs against
   whatever that command lists.
8. `crates/dashboard` builds and its binary runs (prints its placeholder and
   exits 0) — no TUI dependency yet.
9. Nothing outside this contract's owns-list is modified.

**Gate** — report-only (refine-loop).

**Dependencies** — none (this is M3's first task; nothing in M3 can start
before it).

**Runner instructions — REVERTED, see below.**

~~Original plan: implementer = opencode `deepseek` agent, reviewer = opencode
`glm-5.3` agent, dispatched via `mcp__opencode-bridge__opencode_task` with
`wait: false` + `opencode_sessions` polling (design details struck, no
longer in force).~~

**Trial outcome (2026-09-02): failed, reverted.** The opencode `deepseek`
implementer was dispatched twice with an identical brief on this task.
Both times: no `crates/` directory ever appeared, no `cargo build`/`test`
ever ran, `git status` showed zero change outside pre-existing untracked
files, for ~58 minutes and ~26 minutes respectively, independently confirmed
via `opencode_sessions` polling (not a lookup mismatch). Cost/token counters
kept climbing (reasoning tokens into the thousands) with no corresponding
tool call that touched the filesystem — the model was consuming budget
without converging. This is a viability finding about `crof/deepseek-v4-flash`
on a task this size, not a dispatch/polling-plumbing problem — the plumbing
(dispatch, poll, cancel) worked correctly both times.

**Reverted binding for this task:** implementer = `coder` (Agent-tool
subagent), reviewer = `ask_opus` (Agent-tool subagent) — the same topology
M1/M2 used. Everything else in this contract (owns-list, acceptance
criteria, fmt-isolation rule, Review Frame) is unchanged; none of it caused
the stall.

## Review Frame

*Authored by the advisor. Governs disposition and review budget — never what
the reviewer may look at or discover. It cannot suppress credible severe
evidence.*

**As of** — contract version 1

**Product context** — `opencode-bridge` is a published crate: it ships to
crates.io and to GitHub tagged-release consumers. Those are real external
users, and they are downstream of this change even though the repo has no
external contributors. This task restructures that shipping crate's layout
and its release workflow while adding two new crates beside it. Everything
else in M3 is built on the shape this task establishes. The custodial risk
that made this task dangerous — moving files that carried uncommitted work —
was retired by commit `fe9c61b`; what remains is structural.

**Release expectations (disposition)** — `../delivery-profile.md` version 1
governs, and its Finding Disposition rubric applies verbatim. For this task
that resolves to:

- **Correct now** — anything that changes what a published-binary consumer
  observes, breaks the release path, loses behavior or test coverage that
  exists today, or compromises the crate boundary the profile names as a
  non-retrofittable foundation. Consequence decides this, not frequency and
  not how deep in the diff it sits.
- **Preserve foundation** — the client/bridge boundary is one of the two
  foundations the profile calls disproportionately expensive to retrofit. A
  split that technically compiles but puts the boundary in the wrong place
  is a release-bar finding, not a style preference.
- **Defer with trigger** — bounded concerns outside the supported release
  go to `deferred.md` with scenario, consequence, assumption, and trigger.
- **Reject** — alternative workspace topologies, dependency-declaration
  style preferences, and hypotheticals with no scenario attached.

**Depth** — narrow in surface area, high in severity sensitivity within that
surface. The task's criticality is high, so nothing inside the migration's
own blast radius gets a low bar; the ceiling limits *where to spend the
budget*, not how seriously to treat what is found there.

Out of budget, and not worth a finding: multi-crate publish tooling and
release-automation design beyond keeping today's workflow working;
workspace dependency-inheritance style; crate topology beyond the three this
contract names; performance; and the eventual design of the `dashboard`
skeleton, which is deliberately empty here.

Severe evidence overrides this ceiling in both directions — if something
outside the listed surface would break a release or silently drop existing
behavior, report it.

**Budget** — 2 passes. A third needs the runner to escalate rather than
spend it.

**Say clean if it's clean.** A migration whose whole goal is "nothing
changes except location" can legitimately have no findings. Report findings
ranked by severity; do not manufacture volume.
