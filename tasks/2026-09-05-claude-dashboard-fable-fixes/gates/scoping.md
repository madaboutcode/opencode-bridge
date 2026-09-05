# Gate — Scoping · APPROVED

**Date** — 2026-09-05 · chaired by `advisor` (Opus) · run `2026-09-05-claude-dashboard-fable-fixes`

**Verdict** — approved, unconditionally. All seven definition-of-ready items hold.

## The seven items

| Item | State |
|---|---|
| Boundaries | Yes — findings 1, 2, 4, 5 plus four structural smells, enumerated in `PLAN.md`. |
| Out-of-scope | Yes — finding 3 (field-set narrowing, user-rejected) and the live end-to-end proof. |
| Project ground truth | Yes — spec `docs/specs/dashboard/claude.md` R13–R14, the named code paths, and the `cargo test` / `cargo clippy` bar. |
| Milestones real | Yes — M1 behaviour fixes, M2 structural rewrites, with a stated dependency (M2 rewrites code M1 also touches, so it is sequenced after M1's corrected behaviour and new tests exist). |
| Roles bound | Yes — advisor / runner / implementer / reviewer, user-approved. |
| Git policy | Yes — branch `claude-dashboard-fable-fixes` off checkpoint `69addca`; runner commits per task at loop-pass, conductor commits bare tasks and milestone artifacts. |
| Delivery profile | Yes — `delivery-profile.md` v2, user-approved. |

## What was sent back first, and how it resolved

1. **Finding 2 contradicted finding 3's exclusion.** Finding 2's written direction dropped `tool_input` from the wire — a captured-field removal, which the user had ruled out. Resolved: render the action line in `state.rs` from the `tool_input` already on the wire. No field added or dropped, no wire or spec-field change, and consistent with how OpenCode places its own action-line rendering. Recorded in the profile and `decisions.md`.
2. **Git policy was unstated**, with ~22 modified files already in the tree. Resolved as above; the unrelated mosaic/shell work was confirmed left uncommitted and untouched.
3. **Profile had no user approval.** Narrowed to the two posture statements the user had never actually made — memory-only privacy, and no scale/concurrency requirement. Both confirmed as written.

## What the advisor checked, and what it trusted

Checked from documents: `PLAN.md`, the profile, the run brief, the Fable findings file. No code, diffs, or transcripts read — per the advisor's standing diet.

Three facts were reported rather than verified by the advisor, and the conductor subsequently verified all three itself: `69addca`'s exact file list (12 files, no sweep-in of the mosaic/shell stream); OpenCode's action-line call site in `session_state.rs`; and the 333-passing / clippy-clean baseline at the branch tip.

## Carried into Decomposition

- **M1's split is open.** Six items land in `state.rs`. One task is a large diff for a `coder` reviewer to hold in a single pass; six tasks serialize on one file. The advisor's instinct — group by turn-termination (finding 1, subagent `Stop` → `Idle`, question-glyph), tile-content (findings 2 and 4), and session-lifecycle (finding 5 alone) — is a starting point to argue against, not adopt. The decomposition returns to the advisor for judgment.
- **Finding 3 watch.** Any task whose clean implementation would drop an unused captured field must flag it rather than do it. That is finding 3 by another name.
- **M2 reviewer strength.** The turn-state derivation replaces the state machine's core representation and every other fix depends on it. A missed transition fails as a wrong tile during live-proof, not as a red test. The user should be offered the choice to escalate the reviewer for that task alone, when M2's contract is written — not before.
