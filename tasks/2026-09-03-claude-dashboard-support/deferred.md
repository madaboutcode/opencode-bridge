# Deferred - claude-dashboard-support

Entries require a scenario, consequence, deferral assumption, and promotion
trigger.

## T01 - Authenticated lifecycle ordering (S2)

- **Scenario:** real authenticated Claude session needed to observe
  `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PermissionRequest`,
  `Notification`, `Stop` (non-failure), and their ordering in a successful
  turn. All T01 CLI runs were unauthenticated (no credentials in the
  environment; real `~/.claude` off-limits).
- **Consequence:** the adapter's lifecycle/attention mapping for successful
  turns cannot be finalized from T01 evidence; downstream T02/T03 must treat
  successful-turn ordering as unverified.
- **Deferral assumption:** T05's authenticated integrated gate can provide a
  credentialed session and capture these traces through the full path.
- **Promotion trigger:** T05 records authenticated ordered traces for a
  successful turn, tool activity, permission wait, and user exit.
- **Evidence status (2026-09-04):** partially informed, not satisfied. A
  post-T04 live run (`decisions.md`, "M3 cross-task sign-off") drove a real
  authenticated interactive Claude Haiku session through the configured
  hook -> Unix socket -> listener -> adapter path and observed exactly
  `SessionStart` and `SessionEnd` (dashboard count moved 27/252 -> 28/253 on
  start, back to 27/252 on `/exit`), because the test hook wired only those
  two events plus `StopFailure`. `UserPromptSubmit`, `PreToolUse`,
  `PostToolUse`, `PermissionRequest`, `Notification`, and `Stop`
  (non-failure) remain unobserved. The promotion trigger above is not met.

## T01 - Startup gap and foreground/background discovery (S3)

- **Scenario:** live-session behavior of `claude agents --json` and
  hook-based discovery when a dashboard starts after `SessionStart`; no live
  sessions exist without authentication, so only the empty-array case was
  observed.
- **Consequence:** the dashboard's completeness claim stays bounded; the
  documented limitation that foreground interactive sessions are not listed
  by `claude agents --json` remains documentation-based, not runtime-proven.
- **Deferral assumption:** T05's authenticated runs can start background and
  foreground sessions and compare `claude agents --json` against hook
  observations.
- **Promotion trigger:** T05 records foreground/background session
  discoverability and startup-gap behavior with live authenticated sessions.

## T01 - Async hook viability for successful sessions (S1)

- **Scenario:** asynchronous (`async: true`) command-hook execution for a
  successfully initialized session. Async timing/order was not measured;
  successful-session async viability is indeterminate and deferred to T05.
- **Consequence:** the ingress contract cannot select `async: true` for the
  production hook; synchronous execution is the only evidence-backed option
  so far.
- **Deferral assumption:** T05's authenticated integrated gate can measure
  async hook completion relative to CLI exit on a real session.
- **Promotion trigger:** T05 records async-vs-sync hook timing on an
  authenticated successful session.
- **Live finding (2026-09-04):** in the M3 live-validation run, a
  `claude --print --no-session-persistence` probe with a deliberately
  delayed synchronous `SessionEnd` command hook had that hook canceled by
  Claude at process shutdown, with the exact message "SessionEnd hook [...]
  failed: Hook cancelled." An interactive session with an idle pause then
  `/exit` delivered `SessionEnd` reliably instead. This is one observation on
  one CLI version (`2.1.259`), not async-viability evidence; it means T05
  should not rely on delayed `--print`-mode hooks to observe exit-path
  behavior and should use interactive idle-then-exit as the reliable probe
  shape.

## T01 - Exit-path reliability and subagent identity (S4/S5)

- **Scenario:** `SessionEnd` delivery across ordinary exit, interrupt,
  terminal close, crash, and machine sleep, plus `SubagentStart`/`SubagentStop`
  parent identity. Not safely testable without an authenticated active
  session; the provisional 5-minute staleness policy in `EVIDENCE.md` (S4)
  is explicitly pending this evidence.
- **Consequence:** staleness policy remains provisional; subagent parent
  identity representation remains unverified for the adapter.
- **Deferral assumption:** T05's authenticated runs can exercise exit paths
  and subagents through the configured hooks.
- **Promotion trigger:** T05 records `SessionEnd` reliability across exit
  paths and verifies subagent parent identity (or confirms it is not
  representable).

## Live finding - project identity resolution under a plain subdirectory

- **Scenario:** the M3 live-validation run first tried a plain directory
  under `./tmp` as the disposable Claude project cwd; it resolved to the
  parent `opencode-mcp` git root instead of its own identity, because
  project-identity resolution walks up to the nearest git root. A nested
  disposable git repository under `./tmp` isolated the live-test project
  correctly.
- **Consequence:** none for T01c/T02/T03/T04 — no owned code changed and no
  contract is violated. This is a disclosure gap: the behavior is real and
  was not previously recorded in any spec, contract, or deferral, though it
  is not one of the original S1-S7 items either.
- **Deferral assumption:** none needed to close this item; it does not block
  T05. It is recorded here so a future test author does not rediscover it.
- **Promotion trigger:** not applicable. If T05 or a later task changes
  project-identity resolution behavior (owned by T03's
  `project_identity.rs`), that task's own contract governs the change, not
  this entry.
