# T02 — Tile content correctness

**Contract version** — 1

**Context** — goal: make a Running tile's action line and a Question
tile's content both show something real instead of a raw tool name or
stale text · who uses it: the person watching the dashboard tile · scale:
one developer, one dashboard process · criticality: finding 2 is the
review's #2 priority (also blocks live-proof from validating real
behavior); finding 4 makes the Question tile actively misleading today.

**Delivery profile** — `tasks/2026-09-05-claude-dashboard-fable-fixes/delivery-profile.md` version 2 · task override: none

**Boundaries** — owns: `crates/dashboard/src/claude/state.rs`'s
`PreToolUse`/`PostToolUse`/`PostToolUseFailure` match arm, `PermissionRequest`
and `Elicitation` match arms, and `UserPromptSubmit` match arm; a new
function (implementer's choice: inline in `state.rs`, or a new
`crates/dashboard/src/claude/action_line.rs` — this crate's precedent is a
dedicated file per `opencode/action_line.rs`) implementing the object
extraction described below; the module's top doc comment; **`docs/specs/dashboard/layout.md` R5.3's "Question" block wording** (see item 12 below) ·
must not touch: `PermissionDenied`/`ElicitationResult` arms (unaffected —
see rationale below), `crates/dashboard/src/claude/hook.rs`/`wire.rs` (no
field/wire change — `tool_input` is read from the envelope already on the
wire, not added to it), `docs/specs/dashboard/claude.md` (out of its
stated Scope — confirmed with advisor; this does NOT extend to
`layout.md`, which is in scope for the reason in item 12), `docs/specs/dashboard/client.md` R6.4-R6.6 (confirmed OpenCode-only — its own section headers say "(opencode adapter internals)" — finding 2's Claude action-line rendering is not governed by it and does not touch it).

**Conventions** — `cargo test -p dashboard`, `cargo clippy -p dashboard --all-targets`; baseline after T00+T01 land. Import `collapse_newlines`/`basename` from T00's shared module. `snapshot.rs`'s existing contract: `current_action` is "never a raw tool name or argument object" and `recent_actions` "never includes the current value of `current_action`" — this task exists specifically to make the code satisfy that contract, which it currently violates; the test at (the old) `state.rs:832` asserting the violating behavior as intended must be corrected to assert the fixed behavior, not treated as a spec to preserve.

**Skills to read and apply** — `code-quality`, `software-design` (the object-extraction/verb-formatting split below is a design suggestion, not a mandate — improve it if a cleaner shape presents itself, but keep the observable strings this contract pins), `writing-unit-tests`, `writing-specs` (for the layout.md R5.3 wording fix only — item 12).

**Acceptance — done when**:

**Finding 2 — action line.**

1. `tool_input` (a `String`, already on the wire, already possibly
   truncated per R14/R15's 4096-byte bound) is parsed as JSON
   (`serde_json::from_str::<Value>`). **Parse failure (a real possibility
   on truncated input) must degrade gracefully — never panic, never
   propagate an error — falling back to the bare-tool-name rendering in
   item 3.** This is the single most important correctness property in
   this task; a truncated `tool_input` is an expected, not exceptional,
   input.
2. On successful parse, extract an "object" string by `tool_name`
   (assumption, not verified against Claude's own tool-argument schema —
   flag if evidence surfaces otherwise during implementation): `"Bash"` →
   `input.command` (string), newlines collapsed via T00's
   `collapse_newlines`; `"Edit"`/`"Write"`/`"Read"` → basename of
   `input.file_path` via T00's `basename`; `"Grep"` → `input.pattern`;
   `"Agent"` → `input.description`. A missing field or wrong JSON type at
   this step is the same as parse failure — fall back to item 3, never a
   default/empty string standing in for the object.
3. Render the action line as `"{verb}: {object}"` when an object was
   extracted, `"running: {tool_name}"` otherwise (parse failure, unknown
   tool, or missing/wrong-typed field). Verbs: `Bash` → `running`, `Edit`/
   `Write` → `editing`, `Read` → `reading`, `Grep` → `searching`, `Agent` →
   `delegating`.
4. Only `PreToolUse` updates the ring and the current value: push the
   *previous* `current_action` (if `Some`) into `recent_actions` (not the
   bare tool name), then set `current_action` to the freshly rendered
   line for this call. `PostToolUse`/`PostToolUseFailure` touch neither
   `current_action` nor `recent_actions` — they still run the existing
   `clear_pending_tool_use`/attention-`Running`/routing logic unchanged.
   This means splitting the current combined `PreToolUse | PostToolUse |
   PostToolUseFailure` match arm into `PreToolUse` alone and `PostToolUse |
   PostToolUseFailure` together — say so in the diff, this is an
   intentional restructure, not scope creep.
5. Regression tests: (a) a `Bash` call with a valid `command` field renders
   `"running: <command>"` with newlines collapsed; (b) an `Edit` call
   renders `"editing: <basename>"`; (c) a `Read` call renders `"reading:
   <basename>"`; (d) a `Grep` call renders `"searching: <pattern>"`; (e)
   an `Agent` call renders `"delegating: <description>"`; (f) an unknown
   tool name renders `"running: <tool_name>"`; (g) **a `tool_input` that
   is truncated mid-object (invalid JSON) renders `"running: <tool_name>"`
   without panicking** — this is the truncation-safety test, not optional;
   (h) two consecutive `PreToolUse` calls: the ring gains exactly one entry
   (the first call's line) and `current_action` reads the second call's
   line — proves the double-count bug is fixed; (i) a `PostToolUse`
   following a `PreToolUse` for the same call leaves `current_action` and
   `recent_actions` exactly as `PreToolUse` left them.

**Finding 4 — Question tile content.**

6. `UserPromptSubmit` additionally clears `final_assistant_text = None` —
   a new turn's stale "previous answer" must not persist into whatever
   `NeedsYou` state this new turn eventually reaches.
7. `PermissionRequest` additionally sets `final_assistant_text = Some(format!("allow: {object_or_tool_name}"))`, reusing the same object-extraction logic from item 2 (parse failure/missing field falls back to the bare tool name, same as the action line).
8. `Elicitation` additionally sets `final_assistant_text = Some(elicitation_request.clone())` — this field is already natural-language request text (not JSON), no extraction needed.
9. **`PermissionDenied`/`ElicitationResult` additionally clear `final_assistant_text = None`.** (Corrected from this contract's earlier draft, which claimed no clearing was needed here — wrong: `clear_pending_tool_use` flips attention straight to `Running` on a match, and `Running`'s tile block list never shows `final_assistant_text` at all, so the stale "allow: X" text is briefly invisible — but it survives in the tracked session's state, and the next time this session goes `Idle` (its block list *does* show `final_assistant_text`, per `layout.md` R5.3) with no intervening `Stop`/`UserPromptSubmit` to overwrite it, the resolved-and-answered question text reappears on an idle tile. Same class of bug finding 4 exists to fix — stale text outliving the state that produced it — just a later, less obvious trigger than the Question tile itself. Clear it here for the same reason item 6 clears it on `UserPromptSubmit`.)
10. Regression tests: (a) `UserPromptSubmit` after a prior `Stop` had set `final_assistant_text` clears it to `None`; (b) a `PermissionRequest` for a `Bash` call sets `final_assistant_text` to `"allow: <command>"`; (c) an `Elicitation` sets `final_assistant_text` to the raw `elicitation_request` text; (d) a `PermissionDenied` clears `final_assistant_text` to `None`; (e) an `ElicitationResult` clears `final_assistant_text` to `None`.

11. `cargo test -p dashboard` green (baseline + all of the above), `cargo clippy -p dashboard --all-targets` clean.

**Spec correction — `layout.md` R5.3.**

12. `docs/specs/dashboard/layout.md` R5.3's "Question" block, item 3,
    currently reads: "the session's **final assistant text**, wrapped."
    That asserts a specific meaning — the assistant's own closing
    statement — which items 7-8 above make no longer true for a
    permission/elicitation-driven Question tile (it now holds a
    synthesized "allow: X" string, or the elicitation's request text,
    neither of which is assistant-authored). Reword item 3 to describe
    what the block actually holds across all paths that reach a Question
    tile: the assistant's final text when a turn ended in a question
    (`Stop`, unchanged by this task), or what is being asked for when a
    permission or elicitation request is pending (this task's new
    behavior). Keep every structural detail in that item unchanged
    (elastic, wrapped, tail-kept with `⋯` on overflow) — only the
    one-clause description of *what* the block contains needs correcting.
    Follow the `writing-specs` skill for the edit itself (this is a
    correction to an existing requirement's wording, not a new
    requirement — no new `R` number, no `[REVIEW]` marker) and record it
    in a short `spec-delta.md`-style note appended to this run's
    `deferred.md` or `decisions.md` (implementer's call which fits
    better) naming what changed and why, matching the discipline the
    R13/R14 rounds already established.

13. `docs/specs/dashboard/client.md` R6.4-R6.6 is confirmed OpenCode-only
    (its own section headers say "(opencode adapter internals)") and is
    NOT touched by this task — finding 2's Claude-side action-line
    rendering has no shared spec text to correct, only `state.rs`'s own
    module doc comment (per T01's established pattern).

**Gate** — report-only (refine-loop)

**Dependencies** — T00 (and lands after T01 per `PLAN.md`'s pipeline order, though it does not depend on T01's content)

## Review Frame

The largest task, and its real risk is pointwise fixes to a class bug. `final_assistant_text` is now set at two arms and cleared at three; ask whether a fourth exit path exists that does neither, rather than only checking the five listed tests. Test (g) is the one that must genuinely fail before the fix — confirm it feeds real truncated JSON, not a hand-picked invalid string. Verify the `layout.md` edit moved one clause and left elastic/wrapped/tail-kept untouched. The `PreToolUse` arm split is intentional; check `PostToolUse` still clears pending.
