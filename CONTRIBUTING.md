# Contributing to opencode-bridge

Thanks for your interest in contributing! `opencode-bridge` is a small, focused
project: a single-binary MCP stdio server that drives opencode2 over HTTP + SSE.
Most contributions will be bug fixes, small features, or improvements to the
robustness contracts documented in `SPEC.md`.

## Ground rules

- **Open an issue before opening a PR** for anything beyond a trivial fix.
  The MCP surface is small but the contracts (correlation, race guards,
  shared-server scoping, RAII notify claim) are subtle; discussing the change
  first saves everyone time.
- **Keep dependencies lean.** No MCP SDK (the transport is hand-rolled on
  purpose — see `SPEC.md` §4). If a new dependency is genuinely needed, explain
  why in the PR.
- **Don't add disk state.** The registry is intentionally in-memory; opencode2's
  server is the source of truth for session state (see `SPEC.md` §3
  "Statefulness"). New persistence is a design change.
- **All logs go to stderr.** stdout is the MCP protocol channel — a stray byte
  corrupts the stream. `cargo clippy -- -D warnings` must stay green.

## Development setup

```sh
git clone https://github.com/madaboutcode/opencode-bridge.git
cd opencode-bridge
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

To run against a live opencode2 service you need the service running
(`opencode2 service start`); the bridge calls `opencode2 pair` at startup.

## Pull request process

1. Fork the repo, create a branch off `main`.
2. Make your change. Keep commits focused; one logical change per commit where
   possible.
3. Run the full local gate:
   ```sh
   cargo build
   cargo test
   cargo clippy -- -D warnings
   cargo fmt
   ```
4. Push the branch and open a PR. Use the PR template. CI will run the same
   gates plus a build matrix on Linux and macOS.
5. Expect a review that asks "what does this look like with the SSE stream
   half-open?" and "what does this look like with a foreign session also on the
   server?" — both happen in production.

## Reporting bugs

Use the [bug report issue template](.github/ISSUE_TEMPLATE/bug_report.md).
Include:

- `opencode-bridge --version` (commit SHA if built from source).
- `opencode2 --version`.
- A minimal reproduction — which tool call, what arguments, what you got back.
- Stderr from the bridge if you have it (stdout is the MCP stream and is
  usually a single line of JSON).

For security issues, see [`SECURITY.md`](SECURITY.md) — please don't file them
in public issues.

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). By
participating you agree to its terms.
