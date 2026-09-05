# T05 Gate Report

## Conformance

**No.** T05 was not implemented or reviewed. The required implementer and
fresh independent reviewer could not be started in this subagent: the
subagent tool rejected nested spawning at depth 1, and the local `opencode`
CLI rejected the existing user configuration (`subagent_depth` and
`permissions` are unrecognized) before starting an agent. The configuration
was not modified.

## Passes Used

0. No implementation/review pass was possible.

## Verification

- `cargo test -p dashboard`: passed (265 unit tests, plus integration suites).
- `cargo clippy -p dashboard --all-targets`: passed.

These are baseline results only and do not establish T05 conformance.

## Deferred

None. Agent startup failure is an execution blocker, not a product deferral.

## Skipped

All T05 acceptance criteria, including serde conversion, exact wire fixtures,
all fifteen variants, duplicate-key behavior, recursion boundaries, and
hook-side validation review.

## Residuals

The existing hand-written wire conversion remains unchanged and unreviewed
against contract v2. No commit was created.

## Direct Runner Passes

The user-authorized direct runner completed four implementation passes. Pass 1
made the active conversion serde-backed. Pass 2 removed the obsolete duplicate
hand-written mapper and added duplicate-key and recursion-boundary tests. Pass
3 added all 38 optional-field combinations. Pass 4 added explicit JSON-null
decoding and full `ClaudeIpcEnvelope` equality for those combinations. No
T04 or unrelated files were included.

The advisor amended the contract to version 3 / Review Frame v2: the locked
serde_json behavior is 127 nested containers accepted and 128 rejected as
category-only `Malformed`; parser configuration remains unchanged.

**Final independent reviewer:** fresh `luna` — PASS, no findings.

**Verification:**

- focused hook tests: 54 passed
- focused wire tests: 15 passed
- Claude ingress tests: 68 passed
- `cargo test -p dashboard`: 280 unit tests plus all integration/doc tests
  passed
- `cargo clippy -p dashboard --all-targets`: passed
- `git diff --check`: passed

**Conformance:** yes. T05 is ready to commit.
