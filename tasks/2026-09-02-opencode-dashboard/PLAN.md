# PLAN — opencode-dashboard (conductor run)

## Boundaries

This run turns the paper-only parts of the opencode-dashboard design into verified
artifacts, then implements the dashboard once specs exist. Concrete, in order:

- **M1 — project-identity spike (R1.6).** Primary question, per advisor
  correction 2026-09-02 (R1.6 already never uses opencode's `projectID` as the
  key, so that comparison is background colour, not the blocking unknown): is
  `location.directory` reliably present and correct on every session shape the
  dashboard will see — not just a plain top-level session, but a subagent/child
  session, and a session the MCP bridge started with an explicit `directory`
  param? Secondary, still real: does a git-toplevel-based canonicalizer behave
  as claimed across real fixtures (repo root, subfolder, symlink, two
  worktrees). Output: an evidence report + throwaway spike code under
  `tmp/2026-09-02-project-identity-spike/` (own `Cargo.toml`, isolated from the
  root package — same convention as `tmp/20260901-prototype-dashboard-layout/`).
  Done when R1.6 is either confirmed as-is or corrected with the real evidence.
- **M2 — spec tree.** `docs/specs/dashboard/{overview,layout,interactions,client}.md`
  written from the confirmed requirements doc (R1–R10), per the `writing-specs`
  skill. Decomposed in detail once M1 closes — R1.6's outcome may change what
  `client.md` says about project resolution.
- **M3 — dashboard implementation.** The actual `crates/dashboard` binary (R1/R1.1)
  plus the opencode adapter (R4/R6.4-R6.6) and the Mosaic layout
  (already-verified spike at `tmp/20260901-prototype-dashboard-layout/` gets
  promoted into real crate code, not rebuilt from scratch). Decomposed once M2's
  specs exist. **Flagged by advisor, 2026-09-02:** getting to R1/R1.1's
  `crates/dashboard` + `crates/opencode-client` shape means migrating today's
  single published crate (`opencode-bridge`, on crates.io, with `exclude` rules
  and tagged-release CI) into a workspace — moving `src/main.rs`, `opencode.rs`,
  `sse.rs`, `tools.rs` into a crate directory and rewriting `Cargo.toml` and the
  release workflow. Those are exactly the files carrying pre-existing
  uncommitted changes from before this run, so M1/M2's "owns-list never touches
  pre-existing work" guarantee cannot survive into M3 unchanged. The workspace
  migration gets named as its own first task of M3, with the release pipeline
  explicitly in its blast radius — not silently folded into "build the
  dashboard binary."

## Out of scope, this run

- Building a second harness adapter (Claude Code or any other) — R1.3-R1.8 are
  designed so this is additive later, not built now.
- R1.7's staleness threshold/UI treatment — explicitly deferred in the
  requirements doc until a push-only adapter exists to test it against.
- Edge cases and rare-case hardening at every task (malformed git repos,
  permission errors, non-UTF8 paths, concurrency/perf work) — happy-path first,
  real findings above that line get fixed, everything else goes to
  `deferred.md` for a dedicated pass later.
- Re-litigating anything already CONFIRMED in the requirements doc (layout R5
  series, attention model R6.7, etc).

## Project ground truth

- `CONTRIBUTING.md`: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`.
- **Baseline recorded 2026-09-02, before T01, root package (`opencode-bridge`):**
  `cargo test` → 29 passed, 0 failed. `cargo clippy --all-targets -- -D warnings`
  → clean. `cargo fmt --check` → **fails**, pre-existing, on `src/log.rs` and
  `src/tools.rs` (uncommitted work from before this run started, not owned by
  any task in this run). Gate policy: **no new failures**, not "green" — a
  runner is never blocked by, and never responsible for fixing, this baseline
  red. Root's `opencode-bridge` package is not a workspace (no `[workspace]` in
  `Cargo.toml`); every spike under `tmp/` gets its own standalone `Cargo.toml`
  (same as `tmp/20260901-prototype-dashboard-layout/`'s `dashboard-layout-spike`
  package) and so is structurally isolated from root's `cargo` commands and
  from this baseline red — a spike task's own `fmt`/`clippy`/`test` must be
  green on its own, full stop, no exemption.
- No repo-level `CLAUDE.md` exists — conventions live in `CONTRIBUTING.md` and the
  existing `src/*.rs` (this repo is currently one binary, `opencode-mcp`; the
  dashboard is new, not yet scaffolded as a workspace member).
- Durable spec sources: `tasks/2026-09-01-opencode-dashboard.requirements.md`
  (confirmed requirements, R1-R10), `tmp/20260901-prototype-dashboard-layout/`
  (verified layout spike + `BRIEF-v2.md`).

## Run config — roles

| Role | Binding | Notes |
|---|---|---|
| advisor | existing persistent agent `advisor` (Opus) | Already spun up this session, has read the requirements doc + BRIEF-v2 + reviewed the R1.3-R1.8 multi-harness proposal. Reused per the advisor skill's "don't respawn mid-session" rule. |
| runner | `coder` agent, one per reviewed task | Reads `refine-loop` skill + the task's contract, runs the loop, writes the gate report. |
| implementer | `coder` agent | Per user instruction: coder for runner and impl roles. |
| reviewer | `ask_opus` agent | Per user instruction: opus for review. **Calibration, not a checklist** (see refine-loop's own guardrail against "focus on X, Y, Z" phrasing) — achieved via each contract's Context line: scale/criticality set low for spike work (M1), which is what actually suppresses edge-case pedantry through refine-loop's built-in "in scope only if it threatens this task's stated goal at this task's stated scale" rule. Reviewer is always told which skill(s) to read and apply (`code-quality`, plus `software-design` for anything with real component boundaries) and is always given the contract's Acceptance criteria as the adherence-to-spec check. |

## Git policy

- Run branch: `conductor/opencode-dashboard`, cut from `main` at run start.
  **Note:** `main` has pre-existing uncommitted changes (`src/main.rs`, `notify.rs`,
  `opencode.rs`, `sse.rs`, `tools.rs`, plus several untracked files) unrelated to
  this run — branching does not discard them, they carry over as uncommitted.
  Task contracts' owns-lists are scoped to this run's own files only, so runner
  commits never touch that pre-existing work — **holds for M1/M2; see M3**,
  where the workspace migration necessarily touches those same files, named
  explicitly there rather than silently breaking this guarantee.
- Runner commits its task at loop-pass (fixed line in `runner-brief.md`): stage
  the contract's owns list + gate report + `deferred.md`, message `T##: <goal>`.
- Conductor commits bare (unreviewed) tasks at their gate, and milestone
  artifacts at milestone sign-off — explicitly: PLAN.md updates, decisions.md
  entries, advisor-brief.md, contracts/, gates/ outcome files, **and any
  project doc the milestone's task was scoped to correct** (e.g. M1's
  `tasks/2026-09-01-opencode-dashboard.requirements.md` edit — the milestone's
  actual product, not incidental to it). Named explicitly after M1's sign-off
  found the requirements doc had been edited but never committed.
- No force-push, no rewriting the run branch's history. Never touches `main`
  directly — merge/PR is a decision for the user at the end of the run, not
  something this run does on its own authority.

## Decomposition (stubbed — filled in per milestone)

### M1 — project-identity spike
- T01: build + run the spike, produce the evidence report. (see `contracts/T01-project-identity-spike.md` once written)
