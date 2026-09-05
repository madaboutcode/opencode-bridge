<!-- gates/T00-report.md -->

# T00 — gate report

**Conformance:** yes — reviewer's explicit verdict: "CONFORMS." All six acceptance criteria met: `looks_like_question` and `QUESTION_PHRASES` byte-identical to `HEAD:opencode/question.rs`; `collapse_newlines`/`basename` byte-identical except `fn` → `pub(crate) fn`; `render_action_line`'s `"shell"`/`"edit"`/`other` match arms untouched and still in `opencode/action_line.rs`; new shared module `crates/dashboard/src/text.rs` names no provider tool vocabulary; `crates/dashboard/src/claude/` untouched; no new tests added; `cargo test -p dashboard` 333 passed / 0 failed (exact baseline match, no per-bucket drift); `cargo clippy -p dashboard --all-targets` clean.

**Calibration:** delivery profile version 2 · contract version 1 (Review Frame as sealed)

**Passes:**
1. Reviewer found 4 low-severity issues, none blocking conformance: (1) module-doc provenance/spec-citation lost in the move — deferred to T01; (2) `text.rs` module doc mischaracterized `looks_like_question` as "string mechanics" — fixed; (3) `pub mod text;` exposed an empty public surface since all items are `pub(crate)` — fixed to `mod text;`; (4) no direct unit tests for `collapse_newlines`/`basename` — noted for T02, not a T00 defect (contract forbids new tests here).
2. Reviewer verified both fixes landed correctly (module doc now accurately distinguishes rendering mechanics from the R6.7 heuristic; `mod text;` compiles clean, internal consumers still resolve `crate::text::...`), re-verified byte-identity against HEAD, confirmed 333 passed / 0 failed and clippy clean, no new findings. No re-litigation of settled items.

**Residuals:** none.

**Challenges:** none.

**Contested:** none.

**Deferred:**
- `docs/specs/dashboard/client.md` R1.3 says question-detection is "opencode-specific guesswork" that must stay inside an adapter; that becomes false once T01 wires Claude to call `looks_like_question`, and the in-code comment carrying that claim was dropped in the move to `text.rs`. Deferral assumption: T00 changes no observable behavior, so it isn't the task that makes the spec sentence false — T01 is. Promotion trigger: T01's contract must name this reconciliation explicitly (update R1.3 and/or restore equivalent provenance) rather than leaving it implicit. Source: T00 reviewer, pass 1. Appended to `deferred.md`.
- `collapse_newlines`/`basename` have no direct unit tests; coverage is indirect via `render_action_line`'s tests in `opencode/action_line.rs`. Deferral assumption: T00's contract explicitly forbids new tests (pure move, no new behavior). Promotion trigger: T02, once Claude depends on these helpers directly, should add direct coverage so a regression is detectable without relying on OpenCode's test suite. Source: T00 reviewer, pass 1. Appended to `deferred.md`.

**Rejected:** none.
