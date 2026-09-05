# T00 — Relocate shared text-rendering helpers

**Contract version** — 1

**Context** — goal: make `looks_like_question` and the neutral parts of
`render_action_line` (`collapse_newlines`, `basename`) available to
`claude/state.rs` without `claude` depending on `opencode`'s internals ·
who uses it: T01 (`looks_like_question`) and T02 (`collapse_newlines`,
`basename`), both landing after this task · scale: one developer, no
runtime behavior change to either provider · criticality: pure move, zero
logic change — low risk if scoped exactly as written, but any accidental
logic change here would silently corrupt both providers' rendering.

**Delivery profile** — `tasks/2026-09-05-claude-dashboard-fable-fixes/delivery-profile.md` version 2 · task override: none

**Boundaries** — owns: a new shared top-level module (implementer's call
on the exact file name/path — e.g. `crates/dashboard/src/text.rs` — sibling
to `opencode`/`claude`/`naming` in `crates/dashboard/src/lib.rs`'s module
list); `crates/dashboard/src/opencode/question.rs`; `crates/dashboard/src/opencode/action_line.rs`; `crates/dashboard/src/opencode/reconcile.rs` (import line only) · must not touch: `crates/dashboard/src/claude/` (no consumer of the relocated helpers exists yet — that's T01/T02), `crates/dashboard/src/opencode/session_state.rs` or any other opencode file, any test assertions' expected values (behavior must not change).

**Conventions** — `cargo test -p dashboard` and `cargo clippy -p dashboard --all-targets` must both stay exactly as they are today (238 lib + 8 + 67 + 20 = 333 passed, clippy clean) — this task adds no new tests and changes no assertion, because nothing observable changes. Rust module convention in this crate: a shared top-level module sits beside `opencode`/`claude` in `lib.rs`'s `pub mod` list, mirroring the existing `naming` module's placement.

**Skills to read and apply** — `code-quality` (this task exists to fix a coupling-direction smell; make sure the new module's own boundary is clean — it should know nothing about either provider's tool-name vocabulary, only the neutral mechanism).

**Acceptance — done when**:
1. `looks_like_question(text: &str) -> bool` exists in the new shared module, byte-for-byte the same logic as today's `opencode/question.rs` version (same question-phrase list, same `?`-ending check, same empty-text handling). `opencode/question.rs` is deleted (or reduced to nothing worth keeping — implementer's call whether the file is removed entirely or the module simply no longer exists) and `opencode/reconcile.rs`'s import updates to the new location.
2. `collapse_newlines(text: &str) -> String` and `basename(path: &str) -> &str` exist in the new shared module, byte-for-byte the same logic as today's `opencode/action_line.rs` versions. `opencode/action_line.rs`'s `render_action_line` function (the `"shell"`/`"edit"` match) stays in place, unchanged in behavior, now calling the shared module's `collapse_newlines`/`basename` instead of the local private ones it currently defines.
3. `render_action_line` itself does NOT move — it stays OpenCode-specific in `opencode/action_line.rs`, because its match arms hardcode OpenCode's own tool names (`"shell"`, `"edit"`), which is exactly the coupling this task must not introduce into the shared module.
4. All existing tests in `opencode/question.rs`'s and `opencode/action_line.rs`'s `#[cfg(test)] mod tests` (or their new locations, if the implementer chooses to move the tests for the relocated functions alongside them) still exist and still pass, asserting the exact same behavior. No new tests are required — this is a pure move, and a new test would imply new behavior existing to test.
5. `cargo test -p dashboard` reports the same 333 passed / 0 failed it does today (test count may shift by a few if tests physically move files, but the total pass count and zero-failure property must be identical). `cargo clippy -p dashboard --all-targets` stays clean.
6. Nothing under `crates/dashboard/src/claude/` changes in this task.

**Gate** — report-only (refine-loop)

**Dependencies** — none

## Review Frame

This is a relocation, but not a pure move — it edits `opencode/action_line.rs`. So the check is not "did anything change besides imports," it is "does OpenCode still render identically." Verify the moved bodies are logically byte-identical, not merely similar: a silently improved `collapse_newlines` or `basename` corrupts both providers with no failing test, since the existing tests were written against the old behavior. Confirm the new module names no provider's tool vocabulary. No new tests is correct here — a new test would imply new behavior. Report any test-count drift rather than accepting "about 333."
