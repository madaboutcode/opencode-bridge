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
