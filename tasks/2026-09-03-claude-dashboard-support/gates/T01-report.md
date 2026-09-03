<!-- gates/T01-report.md. Written by the task's runner; it is the conductor's entire
     gate and must stand alone. -->

# T01 — gate report

**Status:** ESCALATED — refine-loop pass 2 found issues above the depth line;
per the skill's stopping rule, no 3rd pass runs without the caller's explicit
say-so. No commit was made (escalation rule). See the escalation brief at the
end of this report and the runner's return message.

**Conformance:** partial — reviewer's (luna-high) explicit pass-2 verdict. The
work does not yet meet all of the contract's acceptance criteria; several S1-S5
items are indeterminate or overclaimed, and two above-the-line defects remain.
The contract-sanctioned blocked status for the authenticated scenario is
correctly reported.

**Calibration:** delivery profile version 1 · contract version 2 · Review
Frame "as of" contract version 2 — confirmed by direct read of the contract
before spawning either agent; matched, no mismatch.

**Passes used:** 2 of the 2-pass budget.

- **Pass 1** — Reviewer (luna-high, independent) found 6 findings (5 High,
  1 Medium), conformance partial: (1) `SessionStart` absence not established —
  implementer's fixtures used `async: true`; reviewer's fresh sync probe
  observed `SessionStart` firing unauthenticated; (2) async hook behavior
  presented as a decision but untested — reviewer's timing probe showed a 3s
  async hook did not run before unauthenticated CLI exit; (3) raw-payload
  collectors (`hook-debug.sh`, `hook-all.sh`) and retained raw logs violated
  the metadata-only posture; (4) run scripts invoked Claude from the repo cwd,
  not guaranteeing project-settings isolation; (5) bounded stale-session
  recommendation missing (S4 acceptance); (6) doc-derived schema claims not
  separated from observed (Medium). Triage: all 6 above the line → fixed.
- **Fixes** — Implementer (deepseek-flash, same session) corrected all 6:
  rewrote EVIDENCE.md (SessionStart observed with sync hooks; async marked
  indeterminate), rewrote redacted-schemas.md with observed/documentation-only/
  unknown labels, deleted raw collectors and historical raw logs, made
  `hook-observer.sh` record field presence only, added disposable-cwd probes
  (`test-synchronous-session-start.sh`, `test-async-behavior.sh`), added a
  provisional bounded staleness policy (300s timeout, mark-stale fallback,
  T05 validation), removed CJK text fragments.
- **Pass 2** — Same reviewer, verification framing. Verified: async behavior
  fixed; isolation fixed; staleness policy fixed; field provenance fixed;
  SessionStart correction partially fixed; raw-payload posture partially fixed.
  New findings above the line:
  1. **High — exact supported event set still not selected** (`EVIDENCE.md:11`
     says "all events ... are configurable" — not an exact allowlist).
  2. **High — synchronous trace not reproducible** — the landed sync probe
     records only `SessionStart`, not the claimed `SessionStart → StopFailure
     → SessionEnd` sequence (`EVIDENCE.md:37,41-44`); reviewer's independent
     probe reproduced `SessionStart`-only. Runner independently re-ran the
     landed probe and confirmed `SessionStart`-only as well. The full sequence
     has never been observed in a single run: the earlier async run captured
     `StopFailure`+`SessionEnd` (no `SessionStart`), the sync run captures
     `SessionStart` only.
  3. **Medium — probes do not capture CLI exit status** (contract lists exit
     status as permitted evidence metadata; scripts pipe through `head`
     without `pipefail`/status capture).
  4. **Medium — async timing analysis cannot establish event ordering** (no
     common start/end marker written by the observer).

**Residuals (above the line, unfixed — escalation trigger):**
- R1 (High): S1 exact supported event set not selected.
- R2 (High): S2/S1 synchronous lifecycle ordering overclaimed; not
  reproducible from the landed probe.
- R3 (Medium): CLI exit status not captured by probes.
- R4 (Medium): async ordering analysis lacks common markers.
- R5 (High, contested): sensitive values remain in evidence —
  `EVIDENCE.md:102` stores a transcript-path template
  (`.claude/projects/-Users-ajeesh-projects-madaboutcode-opencode-mcp/
  <session-uuid>.jsonl`); `EVIDENCE.md:134` and `redacted-schemas.md:49`
  include the error-type value `authentication_failed` as an example.

**Challenges / contested dispositions:**
- R5 is contested between reviewer and runner. Reviewer classifies all three
  (transcript-path template, `authentication_failed`, error example) as
  raw-sensitive-value violations of the metadata-only posture. Runner agrees
  the transcript-path template violates the contract's explicit "do not
  store ... transcript paths" rule (above the line), but classifies the
  `authentication_failed` error-type value as metadata-equivalent to exit
  status (the contract permits recording exit status and event names), i.e.
  below the line — though cheap to remove. Both views preserved; disposition
  left to the caller.
- Neither agent contested the delivery profile or the Review Frame.

**Deferred:** 4 items appended to `deferred.md` under T01 (authenticated
lifecycle ordering S2; startup gap and foreground/background discovery S3;
async hook viability for successful sessions S1; exit-path reliability and
subagent identity S4/S5). All are credential-blocked, contract-sanctioned
deferrals to T05's authenticated integrated gate, each with scenario,
consequence, deferral assumption, and promotion trigger.

**Skipped:** none above the line; the contested error-enum-value item is
recorded under Challenges, not silently dropped.

**Authenticated scenario:** BLOCKED — no credentials available
(ANTHROPIC_API_KEY, CLAUDE_CODE_OAUTH_TOKEN, ANTHROPIC_AUTH_TOKEN all unset;
interactive login/setup-token need a human; real `~/.claude` off-limits).
Unblock condition recorded in `EVIDENCE.md`: a credentialed rerun or T05 must
provide authentication and run an interactive session with a real model turn,
tool activity, and permission request. This is the contract-sanctioned outcome
for the real-CLI scenario; fixtures were not substituted for it.

**Files changed (owns-list, per the contract's Boundaries section):**
- `tasks/spikes/2026-09-03-claude-dashboard-support/EVIDENCE.md` (rewritten)
- `tasks/spikes/2026-09-03-claude-dashboard-support/redacted-schemas.md`
  (rewritten)
- `tasks/spikes/2026-09-03-claude-dashboard-support/hook-observer.sh`
  (field-presence only)
- `tasks/spikes/2026-09-03-claude-dashboard-support/test-synchronous-session-start.sh`
  (new)
- `tasks/spikes/2026-09-03-claude-dashboard-support/test-async-behavior.sh`
  (new)
- Deleted within the owns-list: `hook-debug.sh`, `hook-all.sh`,
  `debug-session-start.sh`, `run-claude-with-hooks.sh`,
  `run-claude-all-events.sh` (raw-payload collectors)
- `tasks/2026-09-03-claude-dashboard-support/deferred.md` (4 T01 deferrals
  appended)
- `tasks/2026-09-03-claude-dashboard-support/gates/T01-report.md` (this file)

Nothing outside the owns-list was touched: pre-existing dirty worktree files
(crates/, docs/, tasks/*.md, tmp/) were left untouched; no git add/commit was
run (escalation rule).

---

## Escalation brief (for the conductor / advisor)

**Situation:** T01 refine-loop used both passes; pass 2 still found issues
above the depth line (residuals R1-R5 above). Per the refine-loop skill, the
runner stops here and asks the caller: **fix-and-continue (3rd pass) or
ship-as-is?**

**Recommendation:** fix-and-continue for the five residuals. R1/R2 are
contract-acceptance defects (exact event set; reproducible lifecycle
ordering) and R5's transcript-path template is an explicit contract
violation. R3/R4 are cheap completeness fixes. All five are fixable without
authentication; none require the credentialed scenario.

**Cost estimate:** one implementer pass (same deepseek-flash session) to fix
R1-R5 + one reviewer verification pass (same luna-high session). The
authenticated scenario remains BLOCKED regardless and is contract-sanctioned.

**What "ship-as-is" would mean:** T01 evidence goes out with an unselected
event allowlist, an unreproducible lifecycle-ordering claim, a transcript-path
template in evidence, and missing exit-status/ordering metadata. That
contaminates T02/T03 (the contract's stated criticality) and violates the
metadata-only posture, so shipping is not recommended.

**Blocked items (not fixable here, deferred to T05):** authenticated lifecycle
traces (S2), startup-gap/foreground discovery (S3), async viability on
successful sessions (S1), exit-path reliability and subagent identity (S4/S5)
— all credential-blocked; recorded in `deferred.md` and acknowledged in
`EVIDENCE.md`.