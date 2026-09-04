# T05 - Claude authenticated release verification

**Contract version** - 2

**Context** - goal: close the four evidence deferrals (`deferred.md`
S1/S2/S3/S4-S5) with real authenticated Claude sessions, finalize the
staleness policy and subagent-identity representation on that evidence, and
run the release regression/rollback gate that clears the Claude adapter for
guarded experimental exposure; who uses it: selected users who manually
install the documented hooks; scale: same local single-workstation scope as
T02-T04; criticality: highest in the run — this is the last gate before real
users see live session data, and it is the first task explicitly licensed to
touch sealed T02/T03 files if evidence requires it.

**Delivery profile** - `delivery-profile.md` version 1. Task override: one
DeepSeek Flash evidence-and-implementation pass per evidence area below (may
run serially since later areas can be informed by earlier ones), mandatory
independent spec validation on any spec change, and one fresh Luna High
verification per area touching sealed files. The conductor performs
bookkeeping, commits, and gate recording; the advisor holds the decomposition
Review Frame and the final release sign-off.

**Dependencies** - T01c evidence baseline `401887e`, T02 v6 ingress
`aeb8317`, T03 adapter `e631129`, T04 runtime `fd83209`, T04 post-gate fixes
`04a7cf5`/`bd35c5b`/`babf167` (all accepted in `decisions.md`'s 2026-09-04 M3
sign-off entry). The M3 live-validation run already proved the interactive
`SessionStart`/`SessionEnd` path on an authenticated session
(`claude-haiku-4-5-20251001`, CLI `2.1.259`) and recorded a partial S2
evidence status in `deferred.md` — T05 extends that baseline, it does not
repeat it.

**Boundaries** - owns:

- `crates/dashboard/src/claude/state.rs` — owned outright, but only for
  staleness-policy and subagent-identity logic. These were deferred to T05
  by design from T01, not defect discoveries; they are T05's planned
  deliverable, not conditional access. No other change to this file.
- `docs/specs/dashboard/claude.md`, `client.md`, `overview.md` — final S1-S5
  requirement language, the finalized staleness policy, and removal of the
  provisional/[REVIEW] markers this run's evidence resolves.
- `tasks/2026-09-03-claude-dashboard-support/deferred.md`,
  `decisions.md`, `spec-delta.md`, `gates/T05-report.md`.
- A new `tasks/2026-09-03-claude-dashboard-support/T05-evidence/` directory
  for metadata-only session records (counts, timings, event names, exit
  codes — never prompt text, tool arguments/results, or transcript content).

T05 may touch `hook.rs`, `wire.rs`, `listener.rs`, `command.rs`, or `mod.rs`
**only** under the 2026-09-04 post-gate-fix rule, and **only** to fix a
defect against a rule that file's sealed contract already states — the same
bound applied to `04a7cf5`. A decision entry naming the sealed contract
touched, the rule the change fixes against, and a test or a stated reason
none is needed must exist before or immediately alongside the change. Any
change that alters or adds a rule, rather than fixing a violation of one
that already exists, is a contract amendment: it goes to the advisor for
approval before implementation, not to a decision entry after the fact.
T05 must not touch OpenCode sources, shell sources, or any T01c/T02/T03/T04
evidence/gate file. T05 must not write to `~/.claude`,
project `.claude`, or any real Claude configuration outside the Session
3-scoped credential relaxation (`decisions.md`, "Live-validation credential
isolation amendment") — any broader use of real credentials needs its own
explicit user approval, not inheritance from that entry.

**Conventions** - default all evidence sessions to isolated `HOME`/
`CLAUDE_CONFIG_DIR`; use the real credential store opaquely only where an
isolated `HOME` cannot produce the evidence (as Session 3 did), with a fresh
decision entry per such session stating why isolation was insufficient.
Never read or retain `~/.claude`, project `.claude`, credentials, or
transcript JSONL. Capture only allowlisted metadata in `T05-evidence/`.
Apply the 2026-09-04 post-gate-fix and gate-closure-artifact-verification
rules throughout: every acceptance item below names the artifact and the
exact command that confirms it, not a self-report of completion.

**Evidence areas - done when:**

- **S1 (async hook viability):** at least one supported event configured
  `async: true` in a temporary settings file against a real authenticated
  interactive session; completion timing measured relative to CLI exit;
  production hook-install documentation states sync, async, or
  event-dependent as the recommended configuration, with the evidence cited.
- **S2 (successful-turn lifecycle, remainder):** `UserPromptSubmit`,
  `PreToolUse`, `PostToolUse`, `Notification`, and `Stop` (non-failure) wired
  and observed, in addition to the already-proven `SessionStart`/
  `SessionEnd`, on a session that submits a real prompt and invokes at least
  one tool; ordering recorded in `T05-evidence/`; `deferred.md`'s S2
  promotion trigger marked met or explicitly narrowed with reason.
- **S3 (startup-gap/discovery):** dashboard started after a session is
  already running (foreground) and separately after a background session
  starts; compared against `claude agents --json` output for both; the
  documented "foreground sessions undiscoverable at startup" limitation
  confirmed or corrected in `claude.md`.
- **S4 (staleness policy) + exit-path reliability:** ordinary exit, Ctrl-C
  interrupt, terminal close, and best-effort crash (`kill -9`) and
  sleep/resume exercised; `SessionEnd` delivery (or absence) recorded for
  each; a final bounded staleness policy selected on that evidence,
  replacing the provisional five-minute placeholder, with its user-visible
  treatment documented; `state.rs` changed if the selected policy requires
  logic beyond what already exists — this is T05's owned staleness
  deliverable, not conditional sealed-file access.
- **S5 (subagent identity):** a session that spawns a subagent (if the
  installed Claude version supports one) observed for whether
  `SubagentStart`/`SubagentStop` carry a stable parent-session identity;
  first-class representation added to `state.rs` if the evidence supports a
  stable identity. If the installed Claude version cannot spawn a subagent,
  S5 stays deferred — not closed — and the release documentation states
  subagents are unverified, distinct from a scope decision.
- **S6 (socket/IPC, closure by citation):** T02's gate (user-scoped socket
  path, end-to-end delivery deadline, busy-listener/`ListenerUnavailable`
  mapping) and T04's gate (fixed concurrency, bounded/timed one-frame
  intake, framing/version handling, stale-path identity checks, best-effort
  cleanup) are cited by name in `claude.md` as the artifacts that settle S6.
  If reviewing those artifacts surfaces a gap neither gate actually closed,
  that is a decomposition question for the advisor, not something T05
  resolves by writing new behavior into a sealed file.

**Failure branch** - per `delivery-profile.md`'s existing rule, failure of
the privacy boundary, non-blocking hook behavior, the stale-session policy,
or the OpenCode regression gate **blocks exposure** — it does not send T05
back to redesign a passing-looking policy. Concretely: if async hooks prove
unreliable, record that and recommend sync; if `SessionEnd` is lost often
enough on crash/sleep that no bounded staleness policy is defensible on the
evidence, record that outcome and hold release rather than selecting a
policy to make the gate pass. A release-verification gate that can only
report "done" is not a gate; T05's report must be able to say "blocked" and
name exactly which evidence area failed and why.

**Release regression & rollback - done when:**

- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D
  warnings`, and `cargo fmt --all -- --check` pass with all T05 changes
  included; exact command and pass count recorded in `gates/T05-report.md`.
- Rollback verified: removing the configured hook entries returns the
  dashboard to its pre-opt-in state with no residual process, socket, or
  config, and touches nothing under real Claude configuration beyond the
  user's own hook removal.
- A fresh S7 negative-privacy pass over the T05 authenticated sessions
  confirms no prompt text, tool arguments/results, transcript path/content,
  or secret crossed the hook or adapter boundary.
- **At least one acceptance criterion in this section is proven only by
  running the actual built `dashboard` binary through its real startup path
  — not a unit or integration-harness call into library code — matching the
  `babf167` lesson that 60+ passing tests across three gates did not catch a
  startup-composition panic.**

**Testing** - `cargo test --workspace`, `cargo test -p dashboard --test
claude_ingress`, `cargo test -p dashboard --test claude_runtime`, `cargo
clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --
check`, `cargo build -p dashboard` followed by a live run of the built
binary for the built-binary-only criterion above. No test may access
`~/.claude`, project `.claude`, credentials, or transcript JSONL.

**Gate** - the advisor holds a decomposition Review Frame on this contract
before any implementation pass starts. Each evidence area gets its own fresh
Luna High verification if it touches a sealed file; areas that touch only
`T05-evidence/` and documentation may share one verification pass. Final
release exposure requires the advisor's explicit sign-off naming every
closed acceptance item and its confirming artifact/command, not a milestone
summary alone.

## Review Frame

**As of** - contract version 2

**Context** - Closes S1-S5, finalizes staleness and subagent
representation on authenticated evidence, runs the release
regression/rollback gate.

**Expectations** - `state.rs` owned for staleness/subagent only.
Conditional sealed-file access limited to defects against rules those
contracts already state; any rule change comes to the advisor before
implementation. Evidence may block release - record that outcome, do not
engineer around it. No new events, no transcript access, no broadened
credential use.

**Depth** - Deep on evidence sufficiency, the staleness decision's basis,
and privacy. Advisor holds final release sign-off.

Sealed 2026-09-04 by advisor (standing in for Terra), conditional on the v1
review's four corrections plus the S5 minor, applied above without a
re-review round per the advisor's standing instruction. If any of the four
corrections could not be made as described — particularly if S6 turns out
not to be closed by the cited T02/T04 artifacts — that is a decomposition
question and returns to the advisor before implementation, not resolved in
this text.
