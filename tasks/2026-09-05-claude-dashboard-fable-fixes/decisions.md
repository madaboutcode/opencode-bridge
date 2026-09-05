<!-- Append-only. Per-run scope only — never promoted to a project-level store. -->

## 2026-09-05 — Scoping: scope, finding-3 exclusion, finding-2 resolution, milestones

**Considered** — whether to include finding 3 (narrow R14's captured field
set) and the 4 structural smells in this run; how to resolve finding 2's
own text contradicting finding 3's exclusion (its direction says to drop
`tool_input` from the wire); one vs. two milestones.

**Chosen** — Finding 3 stays out (user reversed an initial "include it"
answer back to keeping wide capture, "easy to reverse right?"). All 4
structural smells are in, overriding the findings file's own
recommendation to defer them to after live-proof. Finding 2 renders its
action line in `state.rs` from the `tool_input` already on the wire — no
field added, none dropped — matching how OpenCode renders its own action
line in `session_state.rs` (verified: `render_action_line` is called from
there, not from the wire/parse boundary). Two milestones: M1 behaviour
fixes (findings 1/2/4/5 + 2 local smells), M2 the two structural rewrites,
sequenced after M1 so they rewrite against settled, tested behaviour.

**Why** — Finding 3 is a product-scope reversal of a decision the user
made deliberately earlier in this thread; the user's final call governs.
Finding 2's literal text would have silently re-opened that exclusion;
resolving it via option (b) keeps the wire/spec untouched and is also the
architecturally consistent choice. M1/M2 split avoids the structural
rewrite (which touches the same code) racing against still-changing
behaviour.

**Limitations** — the "render in state.rs" choice means `state.rs` parses
`tool_input` per event at snapshot time rather than once at ingress;
acceptable at this scale (one developer, no concurrency requirement per
the approved profile).

**Reversal** — if a later run reopens finding 3, this decision's
"no wire/spec change" premise for finding 2 should be re-examined at the
same time, since a narrowed field set could remove `tool_input` outright.

## 2026-09-05 — Scoping sign-off

**Considered** — advisor withheld sign-off pending: (1) finding 2/3
contradiction, (2) unstated git policy, (3) user approval of two
profile postures (memory-only privacy risk accepted; no scale/concurrency
requirement) that were advisor's own reading rather than restated user
decisions.

**Chosen** — All three resolved: (1) per above; (2) committed pre-existing
round 1/2 + subagent-bug-fix work as checkpoint `69addca` (exact filenames
staged, left an unrelated mosaic/shell work stream untouched), branched
`claude-dashboard-fable-fixes` off it, runner-commits-per-task policy
recorded in PLAN.md; (3) user confirmed both profile lines as written via
AskUserQuestion.

**Why** — advisor's sign-off gate requires all seven definition-of-ready
items; these were the three not yet closed.

**Limitations** — none identified.

**Reversal** — either profile posture (privacy, scale) can be amended
later with user approval per the profile's own Amendment clause; would
require re-checking any task's disposition decided under the old posture.

## 2026-09-05 — Decomposition: shared-module relocation over visibility bump

**Considered** — T01 needs `looks_like_question`, T02 needs
`render_action_line`; both live in `opencode`'s private modules
(`mod question;`/`mod action_line;`, not `pub(crate)`), invisible outside
`opencode`. Options: (a) bump both to `pub(crate)` in place; (b) relocate
the provider-neutral parts to a new shared top-level module.

**Chosen** — (b). Further refined after reading both functions directly:
`looks_like_question` is genuinely neutral (pure text classification) and
moves whole. `render_action_line` is NOT neutral — its match arms hardcode
OpenCode's own tool names (`"shell"`, `"edit"`) — so only its two neutral
helpers (`collapse_newlines`, `basename`) relocate; the tool-name dispatch
stays in `opencode/action_line.rs`, calling the shared helpers instead of
defining them locally. Claude gets its own new dispatch function in `claude/`
(T02) using the same shared helpers. This relocation is its own task, T00,
ahead of the T01/T02/T03 pipeline — not folded into whichever of T01/T02
happened to need it first.

**Why** — (a) makes `claude` depend on `opencode`'s internals for code that
isn't about either provider — backwards coupling this run's own
structural-smells remit exists to remove, not add. The `naming` module is
existing precedent for a shared top-level module serving both providers.
Splitting the relocation into its own task keeps a pure "did anything
change besides import paths?" diff separate from each finding's actual
behavior change — a reviewer holding both at once would have the move's
noise hide the fix's substance.

**Limitations** — widens this run's stated file boundaries beyond
`crates/dashboard/src/claude/` to include a new shared module and three
`opencode/` files (import-only changes, no logic/behavior change to
OpenCode). Recorded here and in `PLAN.md` so this is stated scope, not
discovered drift.

**Reversal** — if a future provider needs a third variant of either
helper and the shared module's shape stops fitting cleanly, revisit; not
expected given both extracted pieces are already minimal and generic.

## 2026-09-05 — T01 item 3a: closes the deferred.md `client.md` R1.3 entry

**Considered** — T00's deferred.md entry ("`client.md` R1.3 will become
false once Claude calls `looks_like_question`") named its own promotion
trigger: T01's contract must name the reconciliation explicitly. T01's
contract (v2, item 3a) does so.

**Chosen** — Reworded only the one clause in `client.md` R1.3 that named
the heuristic "opencode-specific guesswork with no real wire signal." It
now reads as a shared, text-based heuristic any harness without a real
wire signal falls back to, naming both opencode's and Claude's `Stop`
paths as the two current callers. The forward-looking clause immediately
after it ("a harness with an actual 'waiting on you' signal reports it
directly, not re-derived from text") was left untouched — it stays true:
neither opencode nor Claude has such a signal today, and if a future
harness does, that clause already covers it. No other part of R1.3, its
surrounding bullets, or the rest of `client.md` was touched.

**Why** — The claim was falsified by T01 itself: `crates/dashboard/src/
claude/state.rs`'s top-level `Stop` arm now calls the same
`crate::text::looks_like_question` opencode's `reconcile.rs` calls, so
"opencode-specific" stopped being an accurate description the moment that
call landed.

**Limitations** — none identified; this is a documentation correction
with no code or behavior implication.

**Reversal** — none anticipated; the wording tracks actual callers, so it
only needs revisiting if a third heuristic caller is added with a real
wire signal (at which point the forward-looking clause, not this one,
would need updating).

## 2026-09-05 — T02 item 12: `layout.md` R5.3 Question block, spec-delta

**Considered** — T02 (finding 4, items 7-8) makes a Question tile's content
also come from a pending `PermissionRequest`'s synthesized `"allow: X"`
text or an `Elicitation`'s raw request text, not only from a turn-ending
`Stop`'s assistant text. `layout.md` R5.3's Question block, item 3, said
the elastic block holds "the session's final assistant text" — true only
for the `Stop` path, now false for the other two.

**Chosen (spec-delta, MODIFIED)** — **R5.3** (`layout.md`, Question block,
item 3): reworded the one clause describing *what* the block contains,
from "the session's final assistant text" to "what is being asked: the
assistant's own final text when the turn ended by asking a question
(`Stop`), or, for a pending permission or elicitation request, a
synthesized "allow: X" string or the elicitation's own request text (see
`claude.md` R14)." Every structural detail in that item — elastic,
wrapped, tail-kept with `⋯` on overflow — is unchanged. No new `R` number
(this corrects R5.3 in place, per the file's own numbering convention);
no `[REVIEW]` marker (the correction is settled, not open).
Reason: T02's own item 7/8 acceptance criteria (this task's contract,
`tasks/2026-09-05-claude-dashboard-fable-fixes/contracts/
T02-tile-content-correctness.md`).

**Why** — Same class of correction as the earlier `client.md` R1.3 entry
above: code now does something the spec text explicitly said it didn't.
Rewording in place (not adding a new requirement) matches this file's
convention that a corrected requirement keeps its number and states the
correction inline.

**Limitations** — none identified; this is a documentation correction with
no code or behavior implication of its own (the behavior it describes was
already implemented by T02's items 6-9).

**Reversal** — none anticipated; would need revisiting only if a future
path sets `final_assistant_text` for a reason not covered by "turn ended
with a question" or "permission/elicitation pending."

## 2026-09-05 — M1 milestone: correct T01's Claude-spec boundary

**Considered** — whether the M1 fresh-eyes finding that R13 contradicted
T01's Notification mapping was a code defect, a new task, or a documentation
boundary problem. T01 explicitly excluded `docs/specs/dashboard/claude.md`,
while the approved delivery profile requires changed attention mapping to be
documented.

**Chosen** — classify this as a decomposition flaw and correct only R13's
Notification paragraph as an M1 milestone artifact. It now states that
`idle_prompt` maps to `NeedsYou` without the question flag,
`permission_prompt` and `agent_needs_input` map to `NeedsYou` with the
question flag, and absent or unrecognized subtypes leave attention unchanged.
No implementation task is recut.

**Why** — the implementation was coherent and the defect was solely that the
source of truth still described the old behavior. A bounded spec correction is
smaller and safer than reopening a passed task for a documentation-only hunk.

**Limitations** — this records the milestone correction; future tasks that
change attention mapping must include the affected Claude spec in their owns
list rather than excluding it by default.

**Reversal** — if the adapter mapping changes again, update R13 and its
co-located scenario in the same change before treating the behavior as gated.

## 2026-09-05 — M1 milestone: reconcile the Claude adapter cross-reference

**Considered** — whether the stale Claude adapter description in
`docs/specs/dashboard/client.md` could remain as historical context because it
predated M1, or whether it contradicted the canonical `claude.md` R13-R15
contract and current implementation strongly enough to block sign-off.

**Chosen** — correct the bounded adapter-description block as an M1
documentation artifact. It now references all fifteen R13 events and the
bounded R14-R15 content contract, states that fields are event-dependent, and
retains the SessionEnd tombstone behavior. No implementation task is recut.

**Why** — `client.md` is part of the project source of truth, and its old
three-event/empty-content claims directly told implementers the opposite of
the current behavior. The correction removes duplication instead of creating
another event matrix.

**Limitations** — broad pre-existing spec-style issues found by the validator
are not part of this correction; they are recorded as a later documentation
reconciliation item in `deferred.md`.

**Reversal** — if the Claude event matrix or bounded-field policy changes,
update `claude.md` first and revisit this cross-reference in the same change.

## 2026-09-05 — M1 milestone sign-off

**Considered** — whether the four gated M1 tasks, the two corrected
cross-file spec artifacts, and the recorded deferrals satisfy the approved
delivery profile after fresh-eyes integration review.

**Chosen** — advisor signed off M1 as **FIT PASS**. Retain the documented
deferrals and proceed to M2 decomposition for turn-state derivation and the
serde-derived wire schema. The module-comment mismatch saying "the two
sub-types" while naming three Notification subtypes is deferred as a
documentation-only correction.

**Why** — all supported M1 workflows have task-level regression coverage,
the integration seams fit, the documented-truth blockers are corrected, and
the remaining concerns are bounded with explicit promotion triggers.

**Limitations** — live end-to-end hook proof and broad hardening/spec cleanup
remain outside M1; M2 must preserve the accepted M1 behavior before changing
the underlying representation.

**Reversal** — if a retained deferral's assumption fails during M2 or live
proof, promote it to a scoped task and reopen the affected milestone decision.
