# Changelog

All notable changes to `opencode-bridge` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Project prepared for open-source release (MIT license, README, CI, contributing docs).

## [0.1.0] — 2026-08-26

Initial release.

### Added
- MCP stdio server (`opencode-bridge`) that drives opencode2 over HTTP + SSE.
- Four MCP tools: `opencode_task`, `opencode_sessions`, `opencode_cancel`, `opencode_catalog`.
- Async-by-default turn dispatch with push-back callbacks into the launching CC session via the AF_UNIX inbox socket (`$CLAUDE_CODE_MESSAGING_SOCKET`).
- Credential-rot handling: re-`pair` once on connect failure / 401.
- SSE consumer with auto-reconnect, idle-timeout, missed-event reconciliation, and an independent 60s periodic sweep backstop.
- RAII notification-claim guard (`NotifyClaim`) for `wait=true` to keep CC cancellation safe.
- Correlation on a shared opencode server via the `cc-bridge:<origin>:<slug>` title tag and prompt `metadata`, scoped to the launching CC session's origin (CC socket PID).
- Free `wait=true` smoke path against `opencode-go/ox-alpha-free`.

[Unreleased]: https://github.com/madaboutcode/opencode-bridge/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/madaboutcode/opencode-bridge/releases/tag/v0.1.0
