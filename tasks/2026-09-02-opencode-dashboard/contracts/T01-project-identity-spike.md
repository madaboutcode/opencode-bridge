# T01 — project-identity resolution spike (R1.6)

**Context** — goal: produce real evidence for R1.6's project-identity assumptions
(does `location.directory` reliably identify a session's project across every
session shape the dashboard will see; does a git-toplevel-based canonicalizer
behave as claimed) so R1.6 can be confirmed-or-corrected before spec-writing ·
who uses it: the run's coordinator, to finalize one paragraph of a requirements
doc — not shipped, not user-facing · scale: a handful of real opencode sessions
(3-4) and a handful of filesystem fixtures (5) — single-run verification, not a
long-lived service · criticality: low/reversible — throwaway spike code, the
worst outcome of a bug here is a wrong number in a report that gets caught at
review

**Boundaries** — owns: `tmp/2026-09-02-project-identity-spike/**` (new
directory, your own `Cargo.toml`, not a workspace member — same convention as
`tmp/20260901-prototype-dashboard-layout/`) · must not touch: anything in
`src/`, the root `Cargo.toml`/`Cargo.lock`, `tasks/2026-09-01-opencode-dashboard.requirements.md`
(read it, don't edit it — the coordinator updates R1.6 from your report), any
other `tmp/` directory

**Conventions** — this is a standalone crate, not part of the root package (root
has no `[workspace]`); your own `cargo build`, `cargo test`, `cargo clippy -- -D
warnings`, `cargo fmt --check` must all be clean inside your directory — no
exemption, this is new code. The root package currently has an unrelated
`cargo fmt --check` failure (`src/log.rs`, `src/tools.rs`, pre-existing,
uncommitted, not yours) — ignore it, it is out of your boundary entirely.

**Reference material — read before building:**
- `tasks/2026-09-01-opencode-dashboard.requirements.md` §R1.6 (the requirement
  under test) and its surrounding R1.3-R1.8.
- `docs/internal/opencode-sse-event-catalog-2026-09-01.md` §4 — the last time
  `projectID`/`subpath`/`location.directory` were checked on the wire, only for
  `/tmp` (non-git) sessions. That gap is what part 1 below closes.
- `docs/internal/opencode-wire-behavior-2026-08-27.md` — precedent for how to
  query a live opencode server directly (direct curl against the REST API,
  service.json password) and a working parent/subagent session pair example
  (scenario C) — reuse this method rather than inventing a new one.
- `src/opencode.rs` in this repo — the existing auth/request code against the
  live opencode server, for the password/header shape if you curl directly
  instead of going through an MCP tool.
- You have the `mcp__opencode-bridge__opencode_task` and
  `mcp__opencode-bridge__opencode_sessions` tools available — you can launch
  real sessions with an explicit `directory` param and inspect them directly
  through these instead of hand-rolling HTTP, if that's faster and gives the
  same evidence.

**Task, part 1 — wire check (primary question).** These probe sessions must be
**inert**: create the session and read its metadata only — never give it a
prompt that would make it use a tool or edit a file. The evidence needed
(`location.directory`/`projectID`/`subpath`) is present at session creation and
needs no work performed. This repo has uncommitted work in exactly the paths
you'll be pointing sessions at (its own root, `docs/internal`) — a probe
session that's actually given a task inherits none of your owns-list boundary
and could touch that work. Use a prompt that asks for nothing (or the smallest
inert acknowledgment your tooling requires to create a session) — not a real
task.

Launch real opencode sessions covering these shapes and record the actual
`location.directory`, `projectID`, and `subpath` for each:
  1. A plain top-level session with `directory` = this repo's root
     (`/Users/ajeesh/projects/madaboutcode/opencode-mcp`).
  2. A plain top-level session with `directory` = a subfolder of this repo
     (e.g. `docs/internal`).
  3. A session that delegates to a subagent (like wire-behavior spike C) —
     record `location.directory` for **both** the parent and the child.
  4. If your tooling supports it, a session launched with an explicit
     `directory` param that differs from the bridge process's own cwd (this
     repo's MCP bridge can start sessions this way — confirm whether the
     child's reported `location.directory` reflects the param or something
     else). If it isn't testable with the tooling available, "not testable,
     and here is why" is an acceptable recorded outcome for this check —
     acceptance below doesn't require all 9 to have a positive result, just an
     explicit recorded one.

**Task, part 2 — canonicalizer check (secondary, still real).** Implement the
resolver R1.6 describes (resolve to git repo toplevel; fall back to the
canonicalized working directory if not inside a repo; canonicalize symlinks /
trailing slash / case) as a small function, and run it against these real
filesystem fixtures, recording actual output for each:
  1. A plain non-git temp directory.
  2. This repo's root (a real git repo toplevel).
  3. A subfolder inside this repo (e.g. `docs/internal`) — must resolve to the
     same identity as #2.
  4. A symlink pointing at this repo's root — must resolve to the same
     identity as #2.
  5. Two separate `git worktree` checkouts of this repo (worktree directories
     under your own `tmp/` boundary) — must resolve to **two different**
     identities (per the user's confirmed R1.6 decision: worktrees are
     separate project boxes, not merged). `git worktree add` also writes to
     `.git/worktrees/` in the main repo and registers the worktree there until
     pruned — that mutation is expected and permitted for this check
     specifically (it is not a violation of "must not touch" outside your
     boundary). Before you commit: `git worktree remove` both, then
     `git worktree prune`, so nothing registered is left behind for a later
     session to misread as someone's in-progress work.

**Acceptance — done when** — a written evidence report (markdown, in your
directory) states, for each of the 9 checks above (4 wire + 5 canonicalizer),
the actual value/output observed and whether it matches what R1.6 assumes —
plus an explicit top-line verdict: "R1.6 confirmed as written" or "R1.6 needs
this specific correction: ...". Your own `cargo build/test/clippy/fmt` all
clean. No requirements-doc edits — that's the coordinator's follow-up once your
report lands.

**Gate** — report-only (refine-loop)

**Dependencies** — none
