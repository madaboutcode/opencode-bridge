<!-- gates/T08-report.md. Written by the task's runner; it is the conductor's entire
     gate and must stand alone. -->

# T08 — gate report

**Conformance:** yes — reviewer's (ask_opus) explicit pass-2 verdict against the contract's Acceptance section, all nine criteria: build/test/clippy clean (29 tests, unchanged in substance), `opencode-client` has zero TUI/MCP deps, release binary path unchanged, CI updated for the workspace and correctly scoped (`cargo package --list -p opencode-bridge`), `dashboard` builds and runs its placeholder, nothing outside the owns-list touched, `cargo fmt --all -- --check` clean. The one item marked PARTIAL in pass 1 (item 4, fmt) was fixed and confirmed clean in pass 2, both by the reviewer's own re-run and the runner's independent verification.

**Calibration:** delivery profile version 1 · contract version 1 · Review Frame "as of" contract version 1 — matched, no mismatch.

**Passes:**
- Pass 1 — implementer: `coder` (Agent-tool subagent). Built the workspace per the contract's split: `crates/opencode-client` (byte-identical move of `opencode.rs`'s `Client`, plus a new `sse.rs` raw `EventStream` primitive), `crates/opencode-bridge` (everything else, import-paths updated only), `crates/dashboard` (skeleton, depends on `opencode-client`, prints placeholder + exits 0). Reviewer: `ask_opus` (Agent-tool subagent), independent judgment, found the crate boundary correctly drawn (matches R1.1's scope) and two findings: (1, low) 3 pre-existing `cargo fmt` violations in `log.rs`/`tools.rs`, carried forward from before this task (verified byte-identical to commit `fe9c61b` by both the implementer and the runner independently) — correctly left untouched in the structural move per the contract's formatting-isolation rule, but the contract's own Conventions section explicitly licenses fixing this as "a separate follow-up commit, never folded into the structural change," and acceptance item 4 requires the owns-list fmt-clean, so disposition was fix-now-as-separate-commit. (2, informational, deferred) `opencode-client/src/opencode.rs` carries stale comments referencing bridge-internal modules (`tools.rs`, `sse.rs`, `registry::Registry::claim_notification`) — correct as-is since the file is a byte-for-byte move; a cleanup pass belongs to a later, separate commit, not this migration.
  - Runner's own independent verification after pass 1 (before sending fix #1 to the implementer): re-ran `cargo build --workspace --all-targets --locked`, `cargo test --workspace --all-targets --locked` (29 passed), `cargo clippy --workspace --all-targets -- -D warnings` (clean), confirmed `crates/opencode-client/Cargo.toml` deps have no TUI/MCP, ran the `dashboard` binary (prints placeholder, exits 0), confirmed release binary at `target/release/opencode-bridge`, and diffed `log.rs`/`tools.rs` against commit `fe9c61b` to confirm the 3 fmt violations pre-date this task.
- Fix — implementer ran `cargo fmt` on exactly `crates/opencode-bridge/src/log.rs` and `crates/opencode-bridge/src/tools.rs` (the two files with violations), nothing else. Runner independently re-ran the full acceptance suite: build clean, 29 tests pass, clippy clean, `cargo fmt --all -- --check` now clean, `git status` confirms only those two files changed (formatting only) beyond the pass-1 diff.
- Pass 2 — reviewer (`ask_opus`, same agent, resumed) verified the fmt fix and did a fresh scan. Confirmed the diff touches exactly the three flagged spots (log.rs:83, log.rs:119, tools.rs:477), purely mechanical, no behavioral change; re-ran the full acceptance suite (build/test/clippy/fmt), all clean; checked the bridge's now-reduced dependency set (no more direct `reqwest`/`serde`/`futures-util`/`eventsource-stream`, consumed only through `opencode-client`) and confirmed `opencode-client`'s new `bytes` dependency is genuinely used (`sse.rs`'s `resp.bytes_stream()`); no new findings; finding 2 (stale comments) reaffirmed as deferred, not re-litigated. Final verdict: all nine acceptance criteria pass, conformance yes.

  Note on this pass's timing: the reviewer's response took materially longer to reach the runner than pass 1 did, long enough that the runner (mistakenly) concluded after ~3 hours of waiting that the dispatch had failed, reported that to the conductor, and drafted this report around a "reviewer never returned" framing. The conductor corrected this with the actual completion output in hand before the runner acted further. That draft framing is superseded by this version; the underlying review work was never actually missing, only its delivery to the runner was delayed. Left as a note here in case the delivery-lag itself (not a lost/failed dispatch) is worth tracking for this session's Agent-tool-subagent-resume mechanism.

**Residuals:** none.

**Challenges:** none.

**Contested:** none.

## Post-commit correction

Pass 2's completion notification was misdelivered to the top-level coordinator's session rather than to this runner's — a notification-routing bug affecting a nested Agent-tool subagent's completion, the same class of misrouting the earlier opencode-dispatch trial surfaced. It was not lost, and the reviewer did not fail to complete. Verbatim result, relayed by the coordinator from its own session:

> 1. The fmt fix landed correctly -- three purely mechanical formatting changes in log.rs (2 spots) and tools.rs (1 spot), exactly matching the pre-existing violations I flagged in pass 1. No other files touched.
> 2. Fresh scan found nothing new. All acceptance criteria now pass cleanly: build, 29 tests, clippy, fmt, no TUI/MCP deps in the client crate, release binary path intact, CI updated, dashboard skeleton runs, nothing outside the owns-list modified.
> 3. Finding 2 (stale comments in opencode-client/src/opencode.rs referencing bridge internals like tools.rs and registry::Registry::claim_notification) remains as a deferred informational item -- the file is a byte-for-byte identical move, and editing comments would violate the contract's rule about keeping moved files unchanged in the migration diff.
> Final conformance verdict: Yes.

This matches the pass-2 account already recorded in the Passes section above (updated in `aa87c2f`). Because the cause was notification routing, not a transport failure, this report does not carry a "SendMessage-resume reliability" flag as a trial-viability concern — the resume mechanism itself worked; delivery of its result to the correct session did not. The routing bug itself is logged at the run level in `decisions.md` by the conductor, not as T08-substance.

**Deferred:** appended to `deferred.md` — the stale bridge-internal comments in `crates/opencode-client/src/opencode.rs` (pass-1 finding 2, informational).

## Independent findings, not from either agent

Also see `deferred.md` for a related note from the implementer's own report (not a reviewer finding, disclosed transparently by the implementer): `cargo publish -p opencode-bridge` (a real registry publish, not `--no-verify` packaging) would currently fail because `opencode-client` isn't itself published to crates.io — inherent to extracting a shared library behind a published crate. This is explicitly out of the Review Frame's budget ("multi-crate publish tooling and release-automation design beyond keeping today's workflow working") and CI never exercises `cargo publish` (only `cargo package --list --no-verify`), so it is not a release-path break under this task's actual release process (tag-push → GitHub Release with binary artifacts, confirmed in `CONTRIBUTING.md`). Logged to `deferred.md` per the profile's disposition rubric rather than silently dropped.

## Files changed (owns-list)
- `Cargo.toml`, `Cargo.lock` (workspace manifest conversion)
- `crates/opencode-client/{Cargo.toml,src/{lib.rs,opencode.rs,sse.rs,error.rs}}` (new)
- `crates/opencode-bridge/{Cargo.toml,src/{main.rs,mcp.rs,tools.rs,state.rs,registry.rs,notify.rs,log.rs,error.rs,sse.rs}}` (moved from `src/`, import-paths updated; `log.rs`/`tools.rs` also fmt-fixed per above)
- `crates/dashboard/{Cargo.toml,src/main.rs}` (new, skeleton)
- `.github/workflows/ci.yml` (workspace-scoped build/test/clippy/package/MSRV steps)
- `README.md` (one path-link update: `src/tools.rs` → `crates/opencode-bridge/src/tools.rs`, a path that actually moved)
- `src/{error.rs,log.rs,main.rs,mcp.rs,notify.rs,opencode.rs,registry.rs,sse.rs,state.rs,tools.rs}` deleted (moved into the new crate layout)
