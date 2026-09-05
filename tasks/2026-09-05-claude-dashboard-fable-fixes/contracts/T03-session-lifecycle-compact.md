# T03 — Session lifecycle: don't reset attention on compact/resume

**Contract version** — 1

**Context** — goal: stop a long session's tile from vanishing into the
footer during auto-compaction · who uses it: the person watching the
dashboard tile during a long-running session (this repo's own sessions
compact routinely) · scale: one developer, one dashboard process ·
criticality: a real, observable annoyance on the most common long-session
path, not an edge case.

**Delivery profile** — `tasks/2026-09-05-claude-dashboard-fable-fixes/delivery-profile.md` version 2 · task override: none

**Boundaries** — owns: `crates/dashboard/src/claude/state.rs`'s
`ClaudeEvent::SessionStart` match arm only; the module's top doc comment ·
must not touch: any other match arm, `hook.rs`/`wire.rs` (the `source`
field already exists on the wire — `SessionStartSource` enum with
`Startup`/`Resume`/`Clear`/`Compact` variants — this task only reads it,
doesn't add it), `docs/specs/dashboard/claude.md`.

**Conventions** — `cargo test -p dashboard`, `cargo clippy -p dashboard --all-targets`; baseline after T00-T02 land.

**Skills to read and apply** — `writing-unit-tests`.

**Acceptance — done when**:

1. `SessionStart` resets `attention` to `Idle { last_update: receipt }`
   only when either (a) the session was not already tracked (a genuinely
   new session, or the dashboard just started and is observing this
   session for the first time), or (b) it was already tracked and `source`
   is `Startup`, `Clear`, or absent (`None`) — a real fresh start or an
   ambiguous one, where resetting to `Idle` is the safe default. When the
   session was already tracked and `source` is `Resume` or `Compact`,
   `attention` is left exactly as it was — the whole point of this fix.
2. `ensure_tracked` still runs unconditionally first (an untracked session
   must still be created with its usual `Idle` initial state via the
   existing `ensure_tracked` path) — this task only changes what happens
   to an *already*-tracked session's `attention` field, not the creation
   path.
3. Regression tests: (a) a `SessionStart{source: Compact}` on an
   already-`Running` tracked session leaves `attention` as `Running`,
   unchanged; (b) same for `source: Resume`; (c) a `SessionStart{source:
   Startup}` on an already-tracked session (a restart re-observing the
   same session id) still resets to `Idle`; (d) a `SessionStart{source:
   Clear}` on an already-tracked session resets to `Idle`; (e) the very
   first `SessionStart` for a session not yet tracked always initializes
   to `Idle`, regardless of `source` — proves the untracked path is
   unaffected.
4. `cargo test -p dashboard` green (baseline + these), `cargo clippy -p dashboard --all-targets` clean.

**Gate** — report-only (refine-loop)

**Dependencies** — none (does not depend on T00/T01/T02's content; sequenced last only because all four share `state.rs` and per-task commits need a clean base each time)

## Review Frame

The smallest task in the run, and the risk is the truth table rather than the code. Five cases must be covered, not the two the finding names: untracked (any source), and tracked with each of Compact, Resume, Startup, Clear — plus absent `source`, which this contract maps to reset, not leave-alone. Confirm `ensure_tracked` still runs first and unconditionally: a fix that skipped creation for a Compact on an unknown session would hide the tile entirely, which is worse than the bug being fixed. Attention must be left byte-identical on the leave-alone paths, not recomputed.
