# opencode-dashboard — Handoff

> Living document — Section 1 is current truth (rewrite each session), Section 2 is
> append-only history. Maintained via the session-continuity skill.

## Section 1: Current State

### Orientation
TUI dashboard for live coding-agent sessions (project-grouped, area-proportional "Mosaic" layout). Conductor run `conductor/opencode-dashboard` — **M1 (project-identity spike) gated, M2 (spec tree) signed off, M3 (implementation) in progress with T08 (Cargo workspace migration) gated**. Next action: decompose the rest of M3 (opencode adapter, Mosaic promotion, naming/claim-map, window controls, keyboard nav) into contracts using `docs/specs/dashboard/*.md` as source of truth.

### Map — read in this order
| Priority | File / section | What to look for |
|----------|----------------|------------------|
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-02-opencode-dashboard/PLAN.md` | Run plan, boundaries, git policy, and M2 file map. Source of truth for what this run owns. Note: truncated in working tree at 131 lines after M2 list — STATE.md carries the current M3 task table. |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-02-opencode-dashboard/decisions.md` | All scope/binding reversals with rationale and reversal triggers. Read before touching roles or delivery posture. Last entry: T08 pass-2 notification misrouting. |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-02-opencode-dashboard/delivery-profile.md` | User-approved v1 release bar (8 supported workflows), deferral posture, and 4 amendments that override earlier PLAN leanings (continuous scaling, zoom cut, R1.7 split). |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/docs/specs/dashboard/client.md` §R1.4 + `visuals.md` §R6.8 | Hard integration point: R1.4's claim-map exception and R6.8's two hard guarantees + cooldown + creation-time-ordered claim resolution. Both amended at M2 sign-off; cooldown-doesn't-survive-restart gap is known and recorded. |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tmp/orchestrator/2026-09-02-opencode-dashboard/STATE.md` | Current milestone/tasks/gate results and active agents. T08 gated `ac6962b` (+ correction `aa87c2f`); M3 not yet decomposed beyond T08. |
| Read closely | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-01-opencode-dashboard.requirements.md` §R5.7–R5.11, §R1.3–R1.8, §R6.8 Appendix | Durable requirements; R5.7 resolved as "accept the motion" (see State & Provenance); R1.3–R1.8 harness-agnostic core, R6.8 10 categories / 60 words frozen. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-02-opencode-dashboard/gates/T08-report.md` | T08 gate: conformance yes, 2 passes, fmt fix licensed, stale-comments deferred. Read the Post-commit correction — the pass-2 misrouting pattern. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tasks/2026-09-02-opencode-dashboard/deferred.md` | Parked real-but-out-of-scope findings (T01: SessionInfo gap, case normalization, canonicalize existence; T08: stale comments, cargo publish path-dep). Not a backlog. |
| Skim | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/tmp/20260901-prototype-dashboard-layout/BRIEF-v2.md` + `renders/real-150x42.txt` | Verified Mosaic build contract and render evidence. Still the layout source of truth; promotion into `crates/dashboard` is M3 work. |
| Reference | `/Users/ajeesh/projects/madaboutcode/opencode-mcp/Cargo.toml`, `crates/opencode-client/`, `crates/opencode-bridge/`, `crates/dashboard/` | Workspace post-T08. `opencode-client` has zero TUI/MCP deps; `dashboard` is a skeleton placeholder. |

### World-Facts & Tooling
- `squarify()` needs weight-descending sorted input or it produces sliver boxes 13.5–19.5:1 — fixed by sorting descending, carrying original index, remapping. Even sorted, greedy last-item still slivers a lone low-weight item (1-session `scratch-cli` at 4.21:1) — inherent, not a bug, accepted.
- `ratatui::TestBackend` + `--dump` flag is the verified non-interactive render pattern in this sandbox (`enable_raw_mode()` fails cleanly, no panic). Plain-text dumps carry no color — debug-print `Buffer` cell RGB, verify, revert.
- Claude Fable 5.1 (`fable-redesign`, `advisor`) reasoning solid, instruction-following unreliable — twice ignored "specs only, no code" and built artifacts anyway; `TaskStop` (hard stop) works, soft `SendMessage` redirect does not. Verify file timestamps/diffs, don't trust self-reported compliance.
- Agent naming: spawning `Agent` with a name already used by a completed agent creates `name-2` rather than resuming — to continue persistent context, use `SendMessage` to the existing name. Current live: `advisor` (Opus), `mosaic-coder-2`/`mosaic-reviewer-2` (idle, hold Mosaic context, not needed for M3). `fable-redesign` stopped.
- Cargo baseline (recorded 2026-09-02, root `opencode-bridge` before T01): `cargo test` 29 passed, `cargo clippy --all-targets -- -D warnings` clean, `cargo fmt --check` **fails** on `src/log.rs` + `src/tools.rs` (pre-existing uncommitted work) — gate is "no new failures" for root, not green. Spike/new crates under `tmp/` must be fully green, no exemption. Root had no `[workspace]` pre-T08, so spikes were structurally isolated from this red.
- `cargo package --list --no-verify` skips registry resolution — does NOT catch an unpublished path dep; `cargo publish -p opencode-bridge` would fail because `opencode-client` isn't on crates.io. Actual release process is tag-push → GitHub Release with binary artifacts (`CONTRIBUTING.md`), so this is deferred, not a release break.
- Workspace migration (T08): `git mv` preserves tracked modifications — an implementer reaching for `cp`+`rm` or `git checkout` would lose the user's 318 lines of uncommitted work across `src/main.rs`+38, `notify.rs`+5, `opencode.rs`+69, `sse.rs`+118, `tools.rs`+88 plus untracked `src/log.rs`. Pre-existing work was committed at `fe9c61b` before T08 to defuse this; after `ac6962b` the workspace is `crates/opencode-client` (shared `Client` + raw `EventStream`), `crates/opencode-bridge` (MCP binary), `crates/dashboard` (skeleton, exits 0).
- Opencode dispatch trial (`deepseek` implementer / `glm-5.3` reviewer via `mcp__opencode-bridge__opencode_task` with `agent:` param, `wait: false` + poll `opencode_sessions`): **failed, reverted** — dispatched twice with identical brief, both stalled with zero filesystem change for ~58min and ~26min while cost/reasoning tokens climbed, confirmed via `opencode_sessions` polling both times — model-capability finding, not plumbing. Reviewer never reached. `wait:true` caps at 240s (`src/tools.rs:23` `WAIT_CAP`) with async fallback and notification pre-claim (`src/tools.rs:264-270`) — do not interleave a new dispatch on a `session_id` still being polled.
- Nested subagent completion misrouting (observed twice, two transports): completion notification for a subagent spawned by a subagent via `SendMessage`-resume landed in the top-level coordinator session, not the logical spawner/ `runner-t08`. Same shape earlier via MCP dispatch callback. Not a lost dispatch — the work was done, delivery was to the wrong session. Runner `runner-t08` waited ~3h on pass 2, concluded non-response, reported to conductor, was corrected by conductor who held the real output. Rule: runners should ping conductor if wait is anomalously long rather than conclude non-response on elapsed time alone. No mitigation yet, logged in `decisions.md`.

### State & Provenance
- **R5.7 resolved — "accept the motion" (explicit, user-approved via AskUserQuestion)** — R5.7's "never moves" guarantee dropped; `squarify()`'s lack of positional stability is accepted as-is. R5/R5.1/R5.2/R5.3 rewritten in place and section promoted from leading-candidate to CONFIRMED (edits T15–T18, `2026-09-01-opencode-dashboard.requirements.md`). Related: on-screen reading order not guaranteed to match list order — same root cause, resolves alongside R5.7.
- **Requirements finalized (§R1.3–R1.8, §R6.7–R6.8, §R6.1–R6.6)** — multi-harness architecture converged (advisor review): adapters push **session snapshots** (upsert keyed by `(kind, id)`) not fine-grained events; `HarnessAdapter` owns all harness semantics (question heuristic, `call_id→name`, action-line rendering); project identity = canonical git toplevel with worktree/subfolder semantics defined; `R3` window keys off time-since-last-snapshot with optional adapter override; idle/active windowing `W=10m`. (Edits T33–T46, `requirements.md`.)
- **Conductor run scoped (explicit)** — branch `conductor/opencode-dashboard` cut from `main` on 2026-09-02; PLAN.md boundaries, out-of-scope, roles, and git policy established. Scoping sign-off withheld then granted after two hard fixes (runner brief created, baseline exemption recorded) and two soft fixes (git-policy M3 qualifier, T01 conditional-acceptance wording) — `advisor` sign-off.
- **T01 gated (M1 sign-off)** — project-identity spike: 9 checks matching (repo root, subfolder, symlink, two worktrees, parent+subagent, explicit `directory` param, non-repo fallback, `projectID` not used). Case-normalization untested (machine case-preserving) carried as deferred OPEN item. Evidence in `tmp/2026-09-02-project-identity-spike/EVIDENCE.md`; R1.6 confirmed with caching obligation flagged for `client.md` (`git rev-parse` per-session cached). Commit `9aa690e` → `444365b` milestone artifacts committed per corrected PLAN.md git policy.
- **Spec tree shipped (M2, 5 files not 4, decomposition explicit)** — `docs/specs/CLAUDE.md` + `README.md` bootstrapped (T02), then `client.md` (T03), `layout.md` (T04), `visuals.md` (T05), `interactions.md` (T06) fanned out, then T07 cross-link + mandatory `writing-specs` clerk validation. `visuals.md` split justified (§R6 series past split threshold); `glossary.md`/`interfaces/` skipped deliberately (no `greybeard` consumer). Validation "all items passing" reinterpreted as applicable-pass + justified-exception (documented in `gates/M1-outcome.md`, `M2-outcome.md`, `decisions.md`) — not a silent waiver.
- **R6.8 redesigned out-of-band between M1/M2 (coordinator-authored, advisor pressure-tested, explicit)** — single-word categorized names, two-layer hash-preferred claim scheme (project→category, session→word) with deterministic probe-on-conflict, cooldown-based recycling (not freelist), category exclusivity across live projects (screen-wide uniqueness given categories > projects). 10 categories / 60 words frozen in requirements Appendix. Reversal documented: old R6.8 chose determinism over uniqueness — explicitly traded away. Capacity edge case deferred. Pressure-test also surfaced: name recycling is invisible mutation, cross-project category collision (~59% with 4 projects / 8 categories) is more likely than within-project — both baked into R6.8's design.
- **M2 sign-off integration fixes (explicit, `advisor` unconditional)** — `client.md` R1.4 amended to carve out claim-map exception (no-derived-state rule now governs snapshot *content*, not all cross-frame state; owner named as core, reason: no adapter sees across projects); `visuals.md` R6.8 claim *order* pinned to ascending creation-time (project position = earliest live session's creation time), and over-specific mechanism (`mod n`, named counters, `wordlists/` path) stripped while keeping guarantees. Cooldown-not-surviving-restart gap recorded for M3 (known behavior, not fixed in M2).
- **Delivery profile retrofitted and user-approved v1 (explicit, 2026-09-02)** — 8 supported workflows, release bar, deferral posture, and 4 amendments accepted as drafted (see Map entry): 1) continuous scaling (reverses conductor's initial 3-line lean — R5.3 2026-09-02 beats R6.3 2026-09-01, plus render evidence rejecting fixed-3), 2) chrome/min-sizes already answered (borderless Mosaic build, BRIEF-v2 regime table), 3) zoom cut from v1 (no prototype, second view + fetch path, doesn't serve R7's "at a glance"), 4) R1.7 split (display defers, claim release on tombstone does not). Also in this sitting: M3 role binding switched from `coder`/`ask_opus` to opencode `deepseek`/`glm-5.3` per user instruction, `runner` stays `coder`, `advisor` stays Opus, with one-task trial and revert trigger.
- **T08 trial failed and reverted (conductor decision, pre-approved trigger)** — see World-Facts. Two dispatches, zero filesystem change, confirmed via `opencode_sessions` polling — reverted to `coder`/`ask_opus` for T08 and remainder of M3 absent a new explicit decision. Logged in `decisions.md` §2026-09-02 T08 trial. Runner instruction lapse once (told to cancel/report, instead started unauthorized second dispatch, then cancelled on request) — flagged, corrected.
- **T08 gated at `ac6962b` (corrections `aa87c2f`, bookkeeping `df8077a`)** — `coder` implementer + `ask_opus` reviewer via `refine-loop` 2 passes. Pass 1: 2 findings (low: 3 fmt violations in `log.rs`/`tools.rs` byte-identical to `fe9c61b` — correctly left in structural move, licensed fix; informational: stale bridge-internal comments in `opencode-client/src/opencode.rs` — deferred). Fix: `cargo fmt` on exactly those two files, nothing else. Pass 2: reviewer fresh scan clean, all 9 acceptance criteria pass (29 tests, zero TUI/MCP deps in client, release binary path, CI workspace-scoped, dashboard skeleton runs, nothing outside owns-list). Pass-2 notification misrouted (see World-Facts) — reviewer did complete, `gates/T08-report.md` corrected, deferred entry appended for stale comments; additional deferred from implementer: `cargo publish` path-dep (see World-Facts). Current branch state after `df8077a`: linear history, `PLAN.md` role binding reverted to `coder`/`ask_opus`, decisions logged, T08 contract now committed.
- **Git state pointers** — run branch `conductor/opencode-dashboard` ahead of `main`; milestones committed per PLAN.md git policy (PLAN + decisions + contracts/gates + corrected project doc). `tasks/2026-09-02-opencode-dashboard/PLAN.md`, `decisions.md`, `deferred.md`, `delivery-profile.md` are durable; `tmp/orchestrator/2026-09-02-opencode-dashboard/STATE.md` is the current task table.
- **Governance in effect** — Conductor run, advisor-gated milestones (advisor `advisor` Opus). Runner = `coder` per task via `refine-loop` skill; implementer = `coder` (M1–M2, M3 after revert), reviewer = `ask_opus` (calibration via Context line, never checklist). Contracts carry Review Frame + delivery profile refs; version must match (T08 was 1/1).

### Judgment — Recommended Next Moves *(opinion)*
- Decompose the rest of M3 now using the spec tree as source of truth — do not touch code until contracts exist. Order: opencode adapter (R1.3/R4/R6.4–R6.6, snapshot pushing, R1.7 staleness hook) first, then Mosaic promotion from verified spike into `crates/dashboard` (already T08-skeletoned), then naming/claim-map + window controls + keyboard nav. The adapter boundary is the v1 foundation most expensive to retrofit — get its contract reviewed first.
- Read-order for anyone decomposing M3: `decisions.md` (reversals/triggers) → `PLAN.md` (boundaries) → `delivery-profile.md` (release bar) → `client.md`+`visuals.md` (integration point) → `overview.md`/`layout.md`/`interactions.md` → `T08-report.md` (what workspace migration actually built).
- Keep `coder`/`ask_opus` as standing binding — a future retry of opencode agents needs a new explicit decision on a small/mostly-prose task, not a default. If retrying, verify notification-slot/`WAIT_CAP` handling and `opencode_sessions` polling before concluding about model quality.
- For R6.8's restart-reproducibility gap (cooldown lost on restart): record as known behavior in M3's claim-map notes or soften the spec sentence — don't add persistent cooldown storage in v1.
- Do not re-derive chrome (R6.2) or minimum tile sizes (R5.5) — delivery profile already closed them as confirmation-only.
- Agent routing that has worked: `coder` for implementation from a precise brief; `ask_opus` for independent review with a calibrated Context line (suppresses edge-case pedantry without silencing real findings); Opus/Fable 5.1 for design pressure-testing but with tight leash / read-only tools — verify timestamps/diffs, use `TaskStop` not soft redirect.

### Dead Ends & Corrections
- Dead end: pre-Mosaic flow-grid/card-demotion design — left ~70% blank at real scale (~8 sessions) where Mosaic fills 40/40 rows, ladder never engages. Abandoned, not fixed; 3 prior bugs moot.
- Dead end: opencode `deepseek-v4-flash`/`glm-5.3` as M3 implementer/reviewer — two identical dispatches, zero filesystem change, climbing cost — model-capability gap on large multi-file structural judgment, not plumbing. Reverted one task later; small/prose tasks remain untested.
- Correction: `redesign-specs.md` "aspect penalty negligible" and status-based tile weighting (question=4/needs-you=3/running=2/idle=1) — both false per advisor's screenshot check and whitespace math, fixed in `BRIEF-v2.md` to content-demand weighting (idle=1/active=2/+1 subagent) + reported sliver.
- Correction: `redesign-specs.md` "tiny-service cyan" → actually green by index mod 6; code correct, doc left as-is (superseded).
- Correction: `delivery-profile.md` v1-cut lean to fixed 3-line cards — backwards (R6.3 2026-09-01 vs R5.3 CONFIRMED 2026-09-02, plus render evidence rejecting fixed-3) — amended to continuous scaling via Amendment 1.
- Correction: M2's `client.md` R1.4 "never holds derived state" contradicting `visuals.md` R6.8 claim-map — fixed by amending R1.4 to scope the rule to snapshot content and name the core as owner of the claim-map.
- Correction: R6.8 unpinned claim order (only probe order pinned; restart arrival order = pagination) — fixed to creation-time-ordered claim resolution at M2 sign-off.
- Failed approach: soft `SendMessage` redirect to mid-task agent — twice landed after artifacts, before processing. `TaskStop` is what actually stops work. Now also: nested subagent completion via `SendMessage`-resume routes to top-level coordinator, not runner — don't misread delay as failure.

### Do-Not-Touch
- `tmp/20260901-prototype-dashboard-layout/` is the verified Mosaic spike (untracked) — authorize before modifying; M3 promotion should copy from it, not rewrite it.
- `tmp/2026-09-02-project-identity-spike/` — T01 evidence, throwaway crate; its committed symlinks with absolute targets (`fixtures/symlink-to-repo-root`, `fixtures/test-symlink-to-repo-root`) dangle on other machines — noted for `.gitignore` + removal next commit touching that area, don't treat as real fixtures.
- Pre-T08 uncommitted work committed at `fe9c61b` — now inside workspace crates; formatting isolation rule still applies: structural moves keep moved files byte-identical except via a separate licensed fmt follow-up.
- This run's git policy: runner commits only owns-list + gate report + `deferred.md` via `git add` (never `-A/-u`), conductor commits milestone artifacts at sign-off, no force-push, never touches `main`.

### Open Items — triaged
**Blocks next phase (resolve first):**
1. **Decompose M3 remainder into contracts** — coordinator (you) to draft using `docs/specs/dashboard/*.md` + `delivery-profile.md` + `T08-report.md` workspace reality. No open spec question blocks this; T08's skeleton and the corrected role binding are the prerequisites that just cleared.
2. **Carrier for `crates/opencode-client` comments** — informational deferred from T08 (`opencode.rs` stale bridge refs). Not blocking — fix when that file is next touched or in a dedicated comment-cleanup pass.

**Resolvable during the work:**
3. M2's "all items passing" reinterpretation is durable — future validation must be read as applicable-pass + justified-exception, not literal zero failures. Applies to any future spec edit that re-invokes the clerk.
4. R6.8 cooldown-restart gap — same live set can produce different names after a restart if cooldown state was active. Record as known behavior in M3; don't build persistent cooldown storage in v1.
5. `cargo fmt --all -- --check` / `cargo test --workspace` / `cargo clippy --workspace` baseline handling for the new workspace — T08 proved `cargo fmt --all -- --check` clean; M3 tasks must keep it clean with no new baseline.

**UNSPECIFIED (ask, don't guess):**
- None — delivery profile, spec tree, and T01/T08 evidence cover all confirmed assumptions. Any new cross-harness or scale assumption needs an explicit decision, not inference.

### Working Commands
```bash
# Verify run branch and recent commits
git -C /Users/ajeesh/projects/madaboutcode/opencode-mcp branch --show-current && git -C /Users/ajeesh/projects/madaboutcode/opencode-mcp log --oneline -8

# Workspace build/test/clippy/fmt (verified post-T08, all clean)
cargo build --workspace --all-targets --locked
cargo test --workspace --all-targets --locked   # 29 tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

# Run the dashboard placeholder (skeleton, not the real TUI yet)
cargo run -p dashboard

# Inspect opencode dispatch state (if experimenting with opencode agents again)
# Use mcp__opencode-bridge__opencode_sessions with the returned session_id, poll every 30-60s until terminal outcome; never reuse a session_id still being polled

# Regenerate Mosaic spike evidence (still untracked, isolated crate)
cd /Users/ajeesh/projects/madaboutcode/opencode-mcp/tmp/20260901-prototype-dashboard-layout && cargo run -- --dump
# writes renders/{real,stress}-{150x42,80x36}.txt

# Confirm spike isolation and run-branch hygiene
git -C /Users/ajeesh/projects/madaboutcode/opencode-mcp status --porcelain tmp/20260901-prototype-dashboard-layout/   # -> "?? tmp/..."
git -C /Users/ajeesh/projects/madaboutcode/opencode-mcp status --short
```

---

## Section 2: Session Log

### Session 1 — 2026-09-02

**Phase**: Layout design (reopened 2nd time) → Mosaic direction built and reviewed, one open decision blocking finalization

**Work done**:
- Reopened treemap-vs-flow-grid via brainstorm; picked squarify-based project sizing.
- Ran `/refine-loop` on first Rust spike (card + demotion ladder) — fixed 4 bugs p1, surfaced 2 HIGH + 1 MEDIUM p2, user chose ship as-is (later abandoned).
- Spawned Fable 5.1 `fable-redesign` for 4-direction exploration; reversed from Ledger to Mosaic once corrected to real scale (~8 sessions); force-stopped via `TaskStop` after ignored scope.
- Folded learnings into `tasks/2026-09-01-opencode-dashboard.requirements.md` as R5.8–R5.11; advisor tightened `BRIEF-v2.md`; adopted B1/B2/B3 from concept images.
- User confirmed content-demand weighting and no aspect floor; built Mosaic spike via `mosaic-coder` and gated via `/refine-loop` (p1 F1 fixed, p2 clean).

**Learned**:
- squarify sort-order bug, greedy sliver behavior, Fable 5.1 scope discipline, `ratatui::TestBackend` + `--dump` pattern, agent naming suffix behavior.

**Blockers surfaced**:
- R5.7 positional stability doesn't hold under mutation test — the live question when this session paused.

---

### Session 2 — 2026-09-02 (conductor run `eea0af06` — this session)

**Phase**: Conductor scoping → M1 spike → spec tree (M2) → M3 scoping/profile + T08 migration

**Work done**:
- Read prior handoff; resolved R5.7 as "accept the motion" (AskUserQuestion, 3-way choice, conductor recommendation option 3 noted, user picked option 1); rewrote R5/R5.1/R5.2/R5.3 in place.
- Finalized requirements: revised R6 lines, new R1.3–R1.8 harness-agnostic architecture (snapshots not events, adapter owns semantics, `(kind,id)` identity, project identity canonicalization, staleness, harness glyph), advisor multi-harness pressure-test folded in.
- Scoped conductor run on `conductor/opencode-dashboard` (`PLAN.md`, `advisor-brief.md`, `T01-project-identity-spike.md` + `T01-runner-brief.md`); handled 4 advisor-scoping corrections (runner brief, baseline exemption for `src/log.rs`/`tools.rs` fmt red, workspace-migration exposure, M1 primary-question reframe); granted scoping sign-off.
- Spawned `runner-t01` → `coder` implementer + `ask_opus` reviewer via `refine-loop`; T01 gated with 9-check evidence (EVIDENCE.md), committed R1.6 spike result + requirements edit + decisions/contracts; M1 sign-off (case-normalization deferred as OPEN item).
- Redesigned R6.8 naming out-of-band via advisor brainstorm: two-layer claim scheme, cooldown recycling, category exclusivity, 10 categories/60 words frozen; pressure-tested recycling + cross-project collision math and reversals.
- Decomposed M2 into 5 spec files (overview/client/layout/visuals/interactions via T02–T06, pipeline after T02 bootstrap); T07 cross-link + mandatory `writing-specs` clerk validation; handled R1.4/R6.8 state-ownership contradiction and claim-order pinning before sign-off; M2 signed off with reinterpreted "all items passing" and CLAUDE.md consumer-lens guidance (advisor sign-off across 5 validation checks).
- Retrofitted `delivery-profile.md` per scoping item 7, user approved all 4 amendments + switched M3 binding to opencode `deepseek`/`glm-5.3` for trial.
- Pre-T08: committed pre-existing uncommitted work at `fe9c61b` (318 lines) to defuse `git mv` loss exposure; adjusted T08 contract (polling not `wait:true`, owns-list enumerating moved files, `WAIT_CAP` 240s, notification pre-claim); scoped T08.
- T08 trial: dispatched opencode `deepseek` twice (~58min, ~26min), both zero filesystem change — confirmed via `opencode_sessions` polling, reverted binding to `coder`/`ask_opus` per pre-approved trigger, logged in `decisions.md`; reported to advisor.
- T08 real run (`coder`/`ask_opus`): 2-pass `refine-loop`, pass-1 2 findings (licensed fmt fix + stale-comments deferred), fix applied as separate `cargo fmt` on `log.rs`/`tools.rs`, pass-2 clean (all 9 acceptance criteria, 29 tests, workspace build/clippy/fmt clean). Pass-2 notification misrouted to coordinator not runner — corrected via conductor relay, report at `gates/T08-report.md` updated in `aa87c2f` + bookkeeping `df8077a`; deferred entries appended; committed at `ac6962b` + corrections.
- Observed two independent "completion misrouted to top-level" patterns (MCP dispatch + Agent-tool resume) — logged as harness behavior, not T08 substance.

**Learned**:
- Snapshot-not-event adapter boundary avoids double-folding per-session state; `git rev-parse` identity needs per-session caching (subprocess per snapshot would stall); claim-map state belongs at core not adapter (cross-project visibility); `wait:true` notification pre-claim + `WAIT_CAP` 240s shape the runner's polling choice; misrouted completions mimic failed dispatches — ping conductor before concluding non-response.

**Blockers surfaced**:
- Opencode `deepseek` unviable for large structural tasks (closed by revert). T08 pass-2 delivery lag ~3h to runner (closed by conductor correction). No open escalations; M3 remainder awaits decomposition.

---

<!-- NEXT SESSION: Append below this line -->
