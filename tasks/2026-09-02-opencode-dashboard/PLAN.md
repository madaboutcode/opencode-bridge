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

## M2 ground truth (spec-writing conventions, shared by T02-T07)

- Skill: `writing-specs` (`~/.claude/skills/writing-specs/SKILL.md`). This repo has no `docs/specs/` tree yet — T02 bootstraps `docs/specs/CLAUDE.md` (format/convention doc) and `docs/specs/README.md` (index). `docs/specs/glossary.md` and `docs/specs/interfaces/` are the skill's two "fixed locations," but both exist specifically to serve the `greybeard`/QA multi-agent process, which this run isn't using — skipped for V1 as a deliberate simplification, not an oversight. Logged in `decisions.md`.
- Mandatory validation: writing-specs requires a separate `clerk` agent to run `~/.claude/skills/writing-specs/references/validation-rubric.md` against each spec file after writing. T07 owns this, after all five content files exist.
- File map (decomposition adjustment from PLAN's original 4-file sketch, made now that the full requirements doc is CONFIRMED and in hand): the R6 series (card content, attention states, chrome, nickname scheme) is substantial enough on its own — projected well past the skill's ~80-100 line split threshold once expanded with scenarios — to warrant its own file, `visuals.md`, rather than folding into `layout.md`. Five spec files, not four:
  - `overview.md` — R1, R1.1, R1.2, R1.3 (summary only, full contract lives in client.md), R2, R3-R3.2, R5.8, R10.
  - `client.md` — R1.3 (full), R1.4, R1.5, R1.6, R1.7, R1.8, R4, R6.4, R6.5, R6.6.
  - `layout.md` — R5-R5.11, R9-R9.2 (empty/too-small/degrade states — grouped here since they're layout's edge-case output, not a separate concern).
  - `visuals.md` — R6, R6.1, R6.2, R6.3, R6.7, R6.8 (+ the R6.8 wordlist appendix, copied from the requirements doc).
  - `interactions.md` — R7-R7.1, R8-R8.1.
- Every spec file: co-locate one Given/When/Then scenario per requirement (writing-specs convention), cross-link siblings by relative path + R-number where behavior depends on another file (e.g. visuals.md's R6.3 line-3 content depends on client.md's R6.5 mapping), carry forward OPEN items from the requirements doc as `[REVIEW: ...]` markers rather than silently resolving them.
- Context line for every T02-T07 contract: who uses it = future M3 implementers (and the coordinator) as the source of truth for building the dashboard crate; scale = 5-6 files, single project, no multi-team consumers; criticality = moderate — a wrong spec means M3 builds the wrong thing, but a human (the user) reviews before anything ships, this isn't unattended production. This keeps reviewers focused on faithfulness-to-requirements-doc and internal consistency, not edge-case pedantry — same calibration approach as T01, restated for spec-writing instead of code.

## Decomposition (stubbed — filled in per milestone)

### M1 — project-identity spike
- T01: build + run the spike, produce the evidence report. (see `contracts/T01-project-identity-spike.md` once written)

### M2 — spec tree
- T02: bootstrap `docs/specs/CLAUDE.md` + `docs/specs/README.md`, write `overview.md`. (pipeline — gates before T03-T06 start, since they must read T02's conventions doc first)
- T03: write `client.md`. (fans out with T04-T06, depends on T02)
- T04: write `layout.md`. (fans out with T03/T05/T06, depends on T02)
- T05: write `visuals.md`. (fans out with T03/T04/T06, depends on T02)
- T06: write `interactions.md`. (fans out with T03/T04/T05, depends on T02)
- T07: cross-link pass across all five files + update `README.md` index + spawn the mandatory `writing-specs` validation agent per file, fix what it flags. (depends on T02-T06 all gated)
