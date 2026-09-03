# T04 - gate report

**Conformance:** yes - Luna High's fresh independent v1 verification found no
above-line issues against the sealed T04 contract.

**Calibration:** delivery profile version 1; contract / Review Frame version 1.

**Passes:** DeepSeek implemented T04 and completed one bounded correction pass
after the first Luna review identified three runtime defects. Clerk performed
the mandatory independent spec validation and found all T04 modifications
valid. Luna then reran the full independent verification and returned CLEAN.

**Runtime:** the exact `dashboard claude-hook` dispatch precedes normal
startup; the helper uses a dedicated bounded stdin read and exits on its finite
deadline even when stdin remains open. Normal startup resolves T02's
user-scoped socket, binds before adapters, keeps OpenCode startup alive when
Claude binding is unavailable, and shuts down Claude tasks before runtime
drop. The listener uses T03 decoding and typed-channel delivery, fixed
concurrency, bounded timed one-frame intake, identity-checked stale-path and
shutdown cleanup, and exact-boundary multi-frame rejection.

**Correction evidence:**

- Held-open helper lifetime: `crates/dashboard/src/claude/command.rs:81-123`,
  regression `crates/dashboard/tests/claude_runtime.rs:438-481`.
- Replacement-safe cleanup for regular files, symlinks, and different
  sockets: `crates/dashboard/src/claude/listener.rs:172-204,269-283`,
  regressions `crates/dashboard/tests/claude_runtime.rs:739-803`.
- Exact-boundary multi-frame rejection while preserving exact-boundary
  single-frame EOF acceptance: `crates/dashboard/src/claude/listener.rs:303-363`,
  regressions `crates/dashboard/tests/claude_runtime.rs:806-870`.

**Verification:**

- `cargo test -p dashboard --test claude_runtime` passed 19 tests, repeated 3
  times.
- `cargo test -p dashboard --test claude_ingress` passed 35 tests.
- `cargo test -p dashboard --test claude_adapter` passed 8 tests.
- `cargo test -p dashboard --lib claude::` passed 49 tests.
- `cargo test -p dashboard --all-targets` passed.
- `cargo clippy -p dashboard --all-targets -- -D warnings` passed.
- `cargo fmt --all -- --check` passed without rewriting the worktree.
- `cargo check --workspace` passed.
- `git diff --check` passed.

**Feature path:** the real runtime test proves built binary command -> Unix
socket -> T02 envelope -> T03 decoder -> live ClaudeAdapter ->
provider-neutral event, with invalid input, timeout, saturation, cleanup,
privacy, and later-valid-client coverage.

**Boundaries:** T02 remains authoritative for parsing, path resolution, and
delivery; T03 remains authoritative for decoding, lifecycle mapping, and
snapshot construction. Only `SessionStart`, `StopFailure`, and `SessionEnd`
remain supported. No configuration, credentials, transcripts, persistence,
session control, public TCP endpoint, or shared fallback was added.

**Documentation:** Clerk verified the T04-owned Claude, client, and overview
spec changes and `spec-delta.md`. Manual opt-in/removal, T05 authenticated and
completeness limits, and all four credential-dependent deferrals remain
intact. Four pre-existing rubric exceptions remain disclosed separately:
intentional adapter-internal contract language in `client.md`, and stale
five-file references in non-owned `layout.md`, `visuals.md`, and
`interactions.md`. They are not T04 regressions.

**Dirty-worktree isolation:** the pre-existing icon-mode block in
`crates/dashboard/src/main.rs:13-33` remains byte-identical and outside the
T04 commit. T04 changes are limited to the approved dispatch, startup,
shutdown, and helper additions. Existing unrelated mosaic, OpenCode, and
documentation changes remain untouched.

**Residuals:** the path-based Unix cleanup has the unavoidable race between
identity verification and `remove_file`; device/inode and socket-type checks
provide the strongest protection available under the sealed contract's
path-based Unix API boundary. T05 retains authenticated full hook-to-dashboard
E2E, final staleness policy, and the four credential-dependent deferrals.

**Safety:** no global or project Claude configuration, credentials, or
transcript JSONL was accessed or modified.
