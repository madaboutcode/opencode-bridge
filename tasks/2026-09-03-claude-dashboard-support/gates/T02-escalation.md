# T02 Escalation Brief

## Area

T02 ingress implementation, Cargo integration tests, Claude spec, and spec-tree
convention/index updates.

## Calibration and History

Delivery profile version 1; T02 contract version 5; Review Frame version 5.
DeepSeek completed the implementation and reported 32 passing ingress tests,
format, clippy, and spec work. One direct Luna review completed. No T02 commit
was made.

## Residual Findings

- **High - socket scope:** `crates/dashboard/src/claude/hook.rs:467-493`
  falls back to `std::env::temp_dir()/dashboard-claude.sock`, which may be a
  shared `/tmp` path and does not guarantee user-scoped socket isolation.
- **High - delivery bound:** `hook.rs:434-452` performs `symlink_metadata`
  outside a timeout, so the complete delivery attempt is not strictly bounded
  by the R16 limit.
- **High - stale index:** `docs/specs/README.md:8-33` still registers only
  five specs, contradicting the updated `docs/specs/CLAUDE.md` and actual six
  file tree, despite being explicitly T02-owned in v5.
- **Medium - spec shape:** `docs/specs/dashboard/claude.md:151-155` places
  two separate Given/When/Then cases under one R15 requirement, violating the
  one-scenario-per-requirement convention.
- **Medium - test gaps:** `claude_ingress.rs:240-334` does not exercise a
  full/busy listener or assert `ListenerUnavailable`; privacy tests inspect
  envelopes/frames but do not capture logs. The implementation's category-only
  logging has not been negatively tested.
- **Out-of-boundary convention issue:** `docs/specs/dashboard/overview.md:16-31`
  also retains a stale five-file registry, but it is outside T02's owns-list.

## Evidence and Consequence

The basic parser, conservative allowlist, transient filtering, size limits,
real Unix socket, concurrency, absent/stale/restarting behavior, and 32-test
suite passed. The residuals affect privacy/operational release criteria and
the documented spec-tree contract. A shared socket fallback can violate local
isolation; an unbounded metadata call can violate non-blocking behavior; and
missing failure/log assertions leave the failure boundary unproven. The stale
overview registry indicates the current decomposition does not cover all
spec-tree entry points.

## Caller Hypothesis

The implementation optimized for the happy path and treated the temporary
socket fallback and filesystem check as harmless, while T02's contract required
strict user scoping and a hard delivery bound. The documentation task updated
the convention and index partially, but the file-level owners did not include
the overview registry. The test gap reflects a contract that named absent and
restarting listeners but did not explicitly name busy/listener-unavailable or
log-capture assertions.

## Advisor Decision Requested

Choose which findings are Correct now versus a contract/decomposition re-cut.
Decide whether T02 should own the stale `overview.md` registry and whether to
authorize one bounded correction plus one fresh Luna verification. Preserve the
metadata-only, user-scoped, bounded, no-global-`~/.claude`, no-transcript, and
T05 authenticated E2E constraints. Do not start T03 until T02 is clean and
durably committed.
