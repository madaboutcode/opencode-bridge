# Implementation Plan: Claude Dashboard Review Remediation

## Purpose

This plan separates one concrete regression in the committed Claude ingress from
unrelated OpenCode and icon changes that are currently unstaged. It defines the
smallest remediation for the R15/R16 envelope failure, the direct Tokio feature
declaration, citation and precision corrections, and the evidence gates needed
before any dirty-worktree change is touched.

This is a plan only. It does not claim that any finding is fixed, and it must
not be used to rewrite the T02/T03/T04 commit history or to absorb the current
UI/OpenCode work into the Claude remediation.

## Review Basis And Boundary

The review was checked against the actual worktree, not only the reported
findings:

- T01c is the current metadata-only evidence baseline at `401887e`; it does not
  claim historical unchanged lineage and keeps all four T05 deferrals.
- T02 is committed at `aeb8317`; its contract is v6 and requires bounded,
  best-effort, category-only ingress with no hook-visible failure.
- T03 is committed at `e631129`; its typed decoder and provider-neutral adapter
  boundary remain authoritative.
- T04 is committed at `fd83209`; its gate reports the runtime feature path,
  bounded command/listener behavior, cleanup tests, privacy tests, and all
  workspace quality checks as passing at the time of that commit.
- The T04 gate itself records an unavoidable path-based cleanup race and keeps
  authenticated end-to-end behavior, final staleness policy, and the four
  credential-dependent evidence promotions with T05.

### Pre-plan Worktree Snapshot

Before creating this plan, `HEAD` was `fd83209` (`T04: wire Claude runtime and
hook listener`) on `conductor/claude-dashboard-support`. The relevant status was:

```text
 M crates/dashboard/src/main.rs
 M crates/dashboard/src/mosaic/ladder.rs
 M crates/dashboard/src/mosaic/palette.rs
 M crates/dashboard/src/mosaic/render.rs
 M crates/dashboard/src/mosaic/view.rs
 M crates/dashboard/src/opencode/mod.rs
 M crates/opencode-client/src/opencode.rs
?? docs/internal/opencode-sse-event-catalog-2026-09-01.md
?? docs/internal/opencode-wire-behavior-2026-08-27.md
?? tasks/2026-09-01-opencode-dashboard.layout-brainstorm.md
?? tasks/2026-09-02-conductor-review-calibration.plan.md
?? tasks/2026-09-03-claude-dashboard-support.plan.md
?? tasks/2026-09-03-claude-dashboard-support/
?? tasks/claude-dashboard-support.handoff.md
?? tasks/opencode-dashboard.handoff.md
?? tasks/spikes/2026-09-01-auth-discovery.md
?? tasks/spikes/2026-09-01-session-shape.md
?? tasks/spikes/2026-09-01-status-signals.md
?? tmp/2026-09-02-dashboard-explainer/
?? tmp/20260901-prototype-dashboard-layout/
?? tmp/dashboard-chrome-prototype/
?? tmp/dashboard-spike/
?? tmp/orchestrator/
```

The `tasks/2026-09-03-claude-dashboard-support/` entry contains the already
recorded T01c/T02/T03/T04 contracts and gate artifacts. It is not an instruction
to edit those artifacts. The requested plan file is the only additional file
this planning pass creates.

A later verification status also showed the unrelated untracked
`tmp/openquota-claude-keychain-partition-bug.md`; this plan did not create or
modify it, and it remains outside the remediation boundary.

## Root Problem

The whole-change review crosses three different change boundaries:

1. A committed T02 typed record accepts field values by UTF-8 byte length, but
   `to_wire` assumes those values always serialize below the separate 8 KiB
   envelope bound. JSON escaping can invalidate that assumption, so a valid
   input can panic inside the hook helper instead of becoming the R15/R16
   category-only drop it is supposed to be.
2. T04's committed code has one invalid `FALLBACK-OK` section citation, and the
   dashboard manifest obtains `tokio::fs` only accidentally through a
   transitive dependency feature.
3. The worktree also contains an unstaged shared-client query change and a
   user-visible icon/render change. Those changes have different owners,
   compatibility questions, and release decisions; mixing them into a Claude
   fix would destroy the reviewed boundary.

The remediation goal is to close only the demonstrated committed defects and
honesty/precision gaps, while leaving all unsupported compatibility, threat
model, and product decisions explicit.

## File Tree

### Committed Claude remediation candidates

```text
crates/dashboard/Cargo.toml                    # direct Tokio fs declaration
crates/dashboard/src/claude/hook.rs            # fallible envelope bound
crates/dashboard/src/claude/wire.rs            # byte-unit precision and callers
crates/dashboard/src/claude/listener.rs        # citation and cleanup claim
crates/dashboard/tests/claude_ingress.rs       # ingress regression coverage
crates/dashboard/tests/claude_runtime.rs       # helper feature regression
docs/specs/dashboard/claude.md                 # byte-unit wording
```

`hook.rs` has inline unit tests. The exact test file additions are part of the
future implementation, not an assertion that they already exist.

### Current unstaged changes, deliberately separate

```text
crates/dashboard/src/main.rs
crates/dashboard/src/mosaic/ladder.rs
crates/dashboard/src/mosaic/palette.rs
crates/dashboard/src/mosaic/render.rs
crates/dashboard/src/mosaic/view.rs
crates/dashboard/src/opencode/mod.rs
crates/opencode-client/src/opencode.rs
```

The first five files are the icon/subagent/render block. The last two are the
shared `list_sessions`/dashboard pagination block. They remain dirty and
untouched by this plan unless their owners approve a separate workstream.

### Baseline and deferred artifacts

The T01c/T02/T03/T04 contracts, gate reports, `deferred.md`, T05 evidence
deferrals, existing OpenCode dashboard specifications, and the untracked spike,
handoff, and internal-wire artifacts are review inputs or existing worktree
state. They are not Claude remediation targets. In particular, the T09
pagination deferral in `tasks/2026-09-02-opencode-dashboard/deferred.md` remains
the baseline owner for the sequential message-fetch observation.

## Data And API Contracts

No Claude wire schema, event allowlist, privacy field, socket path, protocol
version, or timestamp representation changes.

The remediation adds an explicit failure state to the T02 serialization seam:

```text
EnvelopeSerializeError::Oversized
ClaudeIpcEnvelope::to_wire() -> Result<String, EnvelopeSerializeError>
serialize_envelope(record) -> Result<String, EnvelopeSerializeError>
ParseOutcome::Dropped(DropReason::OversizedEnvelope)
DeliveryOutcome::EnvelopeTooLarge
```

The names above are the contract shape; implementation may choose equivalent
names if it preserves the same observable states. The error carries no payload,
path, or JSON text. The serialized size is measured in UTF-8 bytes including
the trailing newline. `MAX_SESSION_ID_LEN` and `MAX_CWD_LEN` remain hard UTF-8
byte limits. No field is truncated, normalized, or widened to make an
envelope fit.

For records produced by `parse_hook_input`, the parser must reject an envelope
whose complete serialized form exceeds `MAX_ENVELOPE_BYTES`. The delivery seam
must also handle a serialization-size error as a non-panicking category-only
drop so a manually constructed or future caller-provided record cannot restore
the hook panic.

## Findings Disposition

| Finding | State | Disposition | Minimal remediation or reason not to fix |
|---|---|---|---|
| P2-1 committed T02 envelope assertion | VALID | Fix in Claude remediation | Make serialization fallible, preflight the complete escaped frame, return a category-only drop, and prove no socket write or helper failure. |
| P3-1 unstaged `limit=500` compatibility concern | SUSPECT | Decision gate, no current edit | Repository evidence shows cursor pagination and no documented request `limit`; the current comment's hand verification is not a durable contract or test. Do not touch the dirty shared client until compatibility is established. |
| P3-2 unstaged limit amplifies sequential `GET /message` calls | BASELINE | Defer to OpenCode scale/performance work | The sequential loop predates the dirty limit change and is already recorded under the T09 50+ session deferral. It is not a Claude release blocker. |
| P3-3 invalid `Bounds And Errors` citation | VALID | Fix in Claude remediation | Replace it with the exact T04 design heading `Bounds And Shutdown` and sweep T04-owned `FALLBACK-OK` citations for stale section names. |
| P3-4 missing direct Tokio `fs` feature | VALID | Fix in Claude remediation | Add `fs` to the dashboard crate's direct Tokio feature list. Current compilation is accidentally enabled by `reqwest`'s transitive graph, not by dashboard ownership. |
| P3-5 Unix permissions and peer identity | SUSPECT | Threat-model decision gate, no current edit | User-scoped runtime/home paths, AF_UNIX local delivery, and the existing no-permission-widening contract are evidenced; no repository requirement sets socket mode or peer-credential authentication. Do not expand the security boundary without a hostile-local-user decision. |
| P3-6a Nerd Font default | INTENTIONAL | Preserve in Claude remediation | The dirty code explicitly chooses Nerd as the default and provides a plain opt-out because font support cannot be detected reliably. This is a product decision, not a demonstrated Claude bug. |
| P3-6b icon environment/flag semantics | DECISION REQUIRED | Separate UI contract gate | `DASHBOARD_ICONS=plain`, `--icons=plain`, and `--no-nerd-font` have behavior, but invalid values, precedence, and help text are not specified or tested. Resolve them in the icon workstream. |
| P3-6c icon width assumptions | SUSPECT | Do not change now | The renderer already uses character-count width assumptions and the new glyphs are single code points intended to occupy one terminal cell. No repository render evidence demonstrates a new width failure. Treat any terminal-specific issue as a UI compatibility investigation. |
| P3-6d missing icon help/tests | VALID | Separate UI follow-up | A user-visible CLI change should have discoverable help and resolver/render tests, but adding them to T04 would silently claim ownership of dirty UI work. |
| P3-7a byte-vs-character wording | VALID | Precision-only correction | Align Claude spec, hook comments, wire comments, and boundary tests with the existing byte-based `str::len()` behavior. Do not change the limits. |
| P3-7b sort-order comment | VALID | Separate UI precision follow-up | Say that stable ties preserve the caller-provided input order, not unsupported "spawn order". Preserve the current urgency sort behavior, but do not edit dirty `view.rs` here. |
| P3-7c listener identity cleanup note | VALID | Security-claim correction | Qualify the cleanup comment to acknowledge the unavoidable check/remove TOCTOU race already recorded by the T04 gate. Preserve the existing identity/type checks and replacement tests. |

### P2-1: Envelope Bound

`hook.rs:248-259` currently serializes a typed record, appends a newline, and
asserts `out.len() <= MAX_ENVELOPE_BYTES`. The field validators at
`hook.rs:389-395` use byte length, so a `cwd` containing many quotes, control
characters, or other JSON-escaped content can be at most 4096 input bytes while
producing more than 8192 wire bytes. The same reasoning applies to
`session_id`. The current `deliver_before` path calls serialization before
metadata, connect, or write, so the assertion can panic the helper before T02
can return its promised best-effort outcome.

This is a real, reproducible contract violation, not a theoretical objection:
R15 says the complete envelope is bounded and that exceeding any bound drops
the whole event; R16 says malformed, dropped, or unavailable delivery exits
successfully and never fails Claude. The existing ASCII max-field test does not
exercise escaped serialization.

Choice: use a fallible full-envelope preflight and drop the event. Do not
truncate, partially send, silently alter escaping, or lower the field limits.
The escaped byte representation is the actual wire size and must be checked
before any write. Keep a second fallible check at delivery as an invariant at
the public seam; it should return `EnvelopeTooLarge`, log only a category, and
make no filesystem or socket attempt.

### P3-1: Shared `limit=500`

The dirty `crates/opencode-client/src/opencode.rs:593-598` change is shared by
the dashboard (`crates/dashboard/src/opencode/mod.rs:122`) and the MCP bridge
(`crates/opencode-bridge/src/tools.rs:592`). The current repository contract in
`SPEC.md` documents `GET /api/session` without a query parameter. The session
shape spike records a cursor with `previous`/`next`, explicitly says there is no
`limit` field in the response, and observed a first page of 50. None of that
proves that a request query is rejected, but it also does not support claiming
that `limit=500` is a compatible API feature. The current source comment saying
it was confirmed by hand is not a checked-in wire contract or automated test.

Disposition: SUSPECT, not accepted as a compatibility regression. The minimal
decision gate is to obtain durable evidence from the supported server version
or an explicit API contract. If the parameter is supported, the separate
OpenCode workstream must document it in the appropriate API contract and add a
request-shape/server-compatibility test covering both consumers. If it is not
supported or cannot be established, remove only the dirty query change and
retain the existing one-page behavior under its documented scale deferral. No
fallback that retries a second URL should be invented without a contract.

### P3-2: Sequential Message Fetches

The committed T09 adapter already loops over every non-running session and
awaits `list_messages` serially at `crates/dashboard/src/opencode/mod.rs:130-137`.
The current unstaged `limit=500` can increase the number of iterations, but it
does not create the sequential design. `T09` and the OpenCode delivery profile
already record one-page pagination and 50+ session scale as deferred, with a
trigger based on observed missing sessions or an unusable dashboard.

Disposition: BASELINE and deferred. Do not block or modify Claude remediation.
The bounded follow-up belongs to the OpenCode owner and should choose one
evidenced strategy before implementation: cursor pagination with an explicit
request budget, or bounded concurrent detail/message fetches with a server and
latency budget. It must include a synthetic large-session test and must not
silently turn a shared client into an unbounded request fan-out.

### P3-3: Citation

`listener.rs:277` cites `design "Bounds And Errors"`, but the referenced T04
design has `## Bounds And Shutdown` at its exact section heading. The correction
is a comment-only replacement with a checkable path and heading, for example:
`tasks/2026-09-03-claude-dashboard-t04-runtime.design.md`, section `Bounds And Shutdown`.
The implementation must also search every `FALLBACK-OK` in T04-owned Rust files
and ensure each section citation exists in the cited design/spec. It must not
rewrite unrelated repository citations or remove a citation merely to make the
sweep shorter.

### P3-4: Tokio Feature Ownership

`hook.rs:470` directly calls `tokio::fs::symlink_metadata`, while
`crates/dashboard/Cargo.toml` declares Tokio features `rt-multi-thread`,
`macros`, `sync`, `time`, `io-util`, and `net`, but not `fs`. The current
dashboard graph still exposes `tokio/fs` through `reqwest`'s `stream` feature
transitively pulled by `opencode-client`; `cargo tree -p dashboard -e features`
confirmed that accidental route. That is not a stable ownership declaration:
removing or changing the unrelated client dependency could make the direct
dashboard source fail to compile.

Disposition: VALID declaration defect. Add only the direct `fs` feature to the
dashboard Tokio dependency. No client dependency, lockfile behavior, or runtime
filesystem design needs to change.

### P3-5: Socket Permissions And Identity

The resolver chooses, in order, an explicit `DASHBOARD_CLAUDE_SOCKET`,
`$XDG_RUNTIME_DIR/dashboard-claude.sock`, or
`$HOME/.local/state/dashboard/claude.sock`. The latter two are user-scoped
locations by convention. The listener uses AF_UNIX, does not create a shared
fallback, and the T02/T04 contracts explicitly prohibit permission widening.
The T04 cleanup code uses `symlink_metadata`, socket type, and device/inode
identity; the gate records the remaining race between identity verification and
path removal.

The implementation does not enforce parent-directory ownership, set an
explicit `0600` socket mode, or validate peer credentials. That could matter if
the product threat model includes a hostile same-host user, a permissive custom
environment override, or a nonstandard runtime directory. The repository does
not currently state that threat model or require peer authentication, and the
metadata-only payload limits the sensitivity of injected data while not
providing authenticity.

Disposition: SUSPECT, with a decision gate. Do not add `chmod`, directory
creation, peer-credential APIs, or a new authentication protocol in this
remediation. If hostile same-host users are in scope, stop and write a separate
security design covering supported Unix platforms, parent-directory ownership,
socket mode, peer identity, failure behavior, and tests for authorized and
unauthorized clients. If the intended model is same-user local tooling, record
that assumption and retain the current contract without hardening by guesswork.

### P3-6: Dirty Icon Work

The dirty block changes `main.rs`, `palette.rs`, `ladder.rs`, `render.rs`, and
`view.rs`. It makes Nerd Font glyphs the default, provides plain mode through
two flags and one environment value, changes subagent rendering and urgency
sorting, and uses existing character-count layout conventions.

The default is intentional in the source comment: there is no reliable
terminal-side font capability signal, so the product chooses Nerd by default
and an explicit plain opt-out. That choice should not be retroactively
reclassified as a Claude defect. The real open questions are discoverability
and contract shape: whether values other than `plain` should be rejected or
treated as Nerd, whether the flag always wins over the environment, whether
both flag spellings remain supported, and what `--help` promises.

The missing tests are real for the dirty UI feature, but they are not evidence
that T04 is wrong. The separate UI follow-up should exercise flag/environment
precedence in a subprocess or pure resolver seam, document the accepted values
in help, and render both modes at representative widths. It should only add a
Unicode display-width dependency if a reproducible terminal/render test shows
that the existing monospace/one-cell assumption fails. Until that workstream is
approved, all icon files remain outside this plan's implementation set.

### P3-7: Precision Nits

The code uses `str::len()` and `String::len()`, which count UTF-8 bytes, while
`claude.md:157-159` says "characters". Update the spec to say UTF-8 bytes and
make the related hook and wire comments/tests use the same vocabulary. This is
worth fixing because the units define the acceptance boundary; it does not
change the limits or acceptance behavior.

The dirty `view.rs:247-252` comment says stable equal-state subagents preserve
"spawn order". The function preserves the input slice order, and no spawn-order
field exists in the view model. Change the comment to caller-provided input
order and leave the stable urgency sort unchanged, but make that comment-only
change in the separate UI workstream because this remediation must not edit
dirty `view.rs`.

The listener comments describe identity checking as if a replacement can never
be removed. The deterministic replacement tests are sound, but a path check and
later `remove_file` cannot be atomic through this API. Qualify the comment to
describe the identity/type guard and its best-effort limitation, matching the
T04 gate's residual-risk statement. Do not weaken or redesign the cleanup code.

## Boundaries

### IN SCOPE

- Repair the T02 envelope-size failure without changing the protocol or privacy
  allowlist.
- Add the direct Tokio `fs` feature to the dashboard manifest.
- Correct the T04 `FALLBACK-OK` citation and sweep T04-owned citations for exact
  section names.
- Correct byte-unit wording in the Claude spec and relevant hook/wire comments.
- Correct the listener cleanup claim and preserve its existing identity/type
  checks.
- Add unit, ingress, helper-feature, and full-quality regression coverage for
  the accepted Claude fixes.
- Record the dirty OpenCode, socket-threat-model, and icon decisions as gates,
  without implementing them here.

### OUT OF SCOPE

- Implementing or validating `limit=500`, cursor pagination, or a new OpenCode
  request strategy.
- Optimizing or parallelizing sequential `GET /message` calls.
- Completing icon-mode help, environment parsing, Nerd Font policy, or render
  width support.
- Adding socket authentication, peer credentials, explicit mode enforcement,
  parent-directory creation, or a new security protocol.
- Authenticated Claude CLI flows, successful-turn evidence, final staleness
  policy, foreground discovery, async-hook viability, exit-path reliability,
  or subagent identity; these remain T05 concerns.

### MUST NOT CHANGE

- Do not amend, rewrite, reset, or remove commits `aeb8317`, `e631129`, or
  `fd83209`.
- Do not modify the current unstaged files listed in the worktree snapshot.
- Do not modify `crates/opencode-bridge/**`, the OpenCode client query, the
  shell, provider-neutral snapshots, event allowlist, socket path precedence,
  T02/T03 ownership split, or T04 startup ordering.
- Do not modify T01c/T02/T03/T04 gate reports, historical contracts, T05
  deferrals, or unrelated spike/handoff artifacts merely to make the report
  look clean.
- Do not read or write `~/.claude`, project `.claude`, credentials, Claude
  settings, transcript files, or transcript JSONL.

### MUST FOLLOW

- Preserve R14 metadata-only behavior and category-only logs. Rejected values,
  paths, and OS error text must not appear in logs or errors.
- Preserve R15 whole-frame semantics: check the complete escaped JSON frame,
  include the newline in the byte bound, and never truncate or partially send.
- Preserve R16: every expected drop is successful from the hook's perspective,
  the single 500 ms delivery deadline is unchanged, and the serialization drop
  occurs before filesystem or socket work.
- Keep T02 authoritative for parsing, path resolution, serialization, and
  delivery; keep T03 authoritative for decoding and lifecycle mapping; keep T04
  runtime code free of lifecycle and snapshot logic.
- Apply `code-quality`: each fallback must be asserted or carry a citation to a
  source that was read; do not add broad catches or speculative recovery.
- Use isolated temporary Unix sockets in tests. No test may access Claude
  configuration, credentials, or transcript JSONL.
- Preserve unrelated dirty bytes and verify the final patch by path ownership,
  not by staging all changes.

## Pre-Decisions

### Repair the already-committed T02 defect

Decision: repair it in a new follow-up change; do not rewrite the existing
commit.

Options: leave the assertion because T02 gated clean; lower field limits; make
serialization escape-aware and reject; or make the serialization path fallible
and drop.

Choice: make the full serialization path fallible and drop the whole event.

Rationale: the escaped input state is within the documented field bounds and is
reproducible, while the assertion directly contradicts R15/R16. Lowering or
changing field limits would alter the evidence-backed contract. A fallible
preflight preserves the wire format, privacy boundary, and no-truncation rule.
The remediation is a forward fix after `fd83209`; it is not a reason to amend
or invalidate the reviewed history.

### Envelope failure shape

Decision: classify serialization overflow as a normal category-only drop, not a
panic and not a listener-unavailable condition.

Options: panic on an impossible internal state; return an incorrect absent
listener result; truncate/escape-normalize to fit; or expose an explicit
oversized-envelope result.

Choice: expose an explicit error/drop category and make both parser and delivery
handle it before any socket operation.

Rationale: the size is a distinct R15 bound. Naming it keeps observability
honest, makes the helper's exit-0 behavior testable, and prevents a future
caller from reintroducing the assertion.

### Dirty OpenCode work

Decision: do not touch `limit=500` or the sequential message loop in this
remediation.

Options: remove the dirty query immediately; keep it and add a fallback; accept
it as proven; or hold it at a compatibility gate.

Choice: hold it at a compatibility gate and leave the existing dirty bytes
unchanged.

Rationale: the change is shared by the dashboard and MCP bridge, while the
repository's durable API evidence documents cursor pagination but not a limit
query. A fallback would make the shared client more complex without a
contract. The T09 scale deferral already covers the sequential baseline.

### Socket security boundary

Decision: do not harden modes or peer identity in the Claude remediation.

Options: enforce `0600`; enforce user-owned `0700` parent directories; validate
AF_UNIX peer credentials; or retain the same-user local threat model.

Choice: retain the current behavior pending an explicit threat-model decision.

Rationale: the current contracts require user-scoped paths and prohibit
permission widening, but do not require authentication or a hostile-local-user
boundary. Adding platform-specific security machinery without that decision
would expand the system's claimed security model rather than fix an evidenced
contract failure.

### Dirty icon work

Decision: keep all icon/render changes outside this Claude remediation.

Options: revert the default, add tests/help now, or preserve the product block
and make it a separate UI release decision.

Choice: preserve the dirty block byte-for-byte and require a separate UI
decision gate.

Rationale: Nerd Font default is explicitly intentional, while help, environment
semantics, and width behavior are unresolved product/API questions. The T04
contract explicitly approved only isolated `main.rs` runtime hunks and required
the icon hunk to remain untouched.

## Orchestration

### Claude envelope flow

```text
hook input
  -> T02 parse and allowlist fields
  -> construct typed ClaudeHookRecord
  -> serialize the complete protocol-v1 envelope, including newline
  -> if serialized bytes exceed 8 KiB: report category-only drop and exit 0
  -> otherwise resolve path, inspect socket, connect, and write under R16
  -> if delivery-side serialization reports overflow: category-only drop,
     with no metadata or socket operation
```

Contract-level pseudocode:

```text
parse_hook_input(input, receipt):
  reject input and field byte bounds as today
  build only the allowlisted typed record
  attempt full envelope serialization
  return Dropped(OversizedEnvelope) when the serialized byte bound fails
  return Accepted(record) otherwise

deliver_before(record, path, deadline):
  attempt full serialization before metadata or connect
  on Oversized: report the category and return EnvelopeTooLarge
  on success: preserve existing bounded metadata/connect/write behavior
  report only the existing delivery category; never include record values
```

### Citation and manifest flow

```text
inspect T04-owned FALLBACK-OK citations
  -> replace the one stale design heading
  -> verify each remaining citation against its exact source
  -> declare tokio/fs directly in dashboard Cargo.toml
  -> run targeted Claude tests and workspace quality gates
```

### Deferred decision flow

```text
record compatibility, threat-model, and icon questions
  -> assign each to its OpenCode/security/UI owner
  -> obtain evidence or product approval
  -> update the owning contract/spec before implementation
  -> execute a separate scoped plan with its own tests and rollback
```

## Testing Strategy

### P2-1 feature verification

- Add a built-binary feature case in `claude_runtime.rs` that sends a valid
  `SessionStart` payload whose `cwd` or `session_id` is under its input byte
  bound but contains escape-heavy characters that make the serialized envelope
  exceed 8 KiB. Assert process exit 0, empty stdout, category-only stderr, no
  panic text, and no frame at a real Unix listener. This proves the user-facing
  helper path rather than only a private helper.
- Add a T02 unit test in `hook.rs` proving the parser returns
  `DropReason::OversizedEnvelope` for that payload and never returns an accepted
  record. Capture the category-only log and assert that no sentinel/value is
  present.
- Add a T02 unit test constructing a typed record with an oversized escaped
  wire representation and proving serialization returns the explicit error
  rather than panicking. Keep the ordinary ASCII maximum-field case accepted.
- Add an ingress integration test in `claude_ingress.rs` that calls the delivery
  seam with the overflow case, asserts `DeliveryOutcome::EnvelopeTooLarge`,
  and proves a real listener receives no partial or complete frame.
- Keep or update the existing exact-boundary tests so an envelope at exactly
  `MAX_ENVELOPE_BYTES` remains accepted and an envelope over it is dropped.

### P3-4 feature declaration

- Run `cargo check -p dashboard --locked` and the Claude test targets after the
  manifest change. Compilation must no longer depend solely on `reqwest`'s
  transitive Tokio feature.
- Inspect `cargo tree -p dashboard -e features` and the manifest diff to confirm
  `fs` is explicitly present in the dashboard Tokio dependency. This is a
  dependency-ownership check, not a new runtime behavior test.

### P3-3 and P3-7 precision/citation checks

- Run a scoped search for `Bounds And Errors` and require no result in T04-owned
  code; verify the replacement resolves to the exact `Bounds And Shutdown`
  heading.
- Search all `FALLBACK-OK` comments in T04-owned Rust files, follow each
  citation to a real section/spec requirement, and report any remaining stale
  citation rather than deleting it.
- Add byte-boundary unit cases with multibyte UTF-8 values: a value whose
  character count is within the nominal limit but whose UTF-8 byte length is
  over it must be rejected, while an exactly byte-bounded value remains subject
  to the existing envelope check. Mirror the validator wording in the hook and
  wire tests.
- Keep the existing deterministic listener replacement tests for regular files,
  symlinks, and different sockets. Do not add a timing-sensitive TOCTOU test;
  the comment correction must state the race that the API cannot eliminate.

### Regression and integration suite

- Run `cargo test -p dashboard --test claude_ingress --locked`.
- Run `cargo test -p dashboard --test claude_runtime --locked`.
- Run `cargo test -p dashboard --test claude_adapter --locked`.
- Run `cargo test -p dashboard --lib claude:: --locked`.
- Run `cargo test --workspace --all-targets --locked`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.
- Run `cargo fmt --all -- --check` and `git diff --check`.
- Run `cargo check --workspace --locked`.
- Confirm no test reads `~/.claude`, project `.claude`, credentials, settings,
  transcripts, or transcript JSONL.

### Conditional tests for rejected/deferred findings

- P3-1, if retained by its owner, requires an HTTP request-shape test proving
  the exact query is accepted by the supported server contract, a documented
  API/spec update, and bridge plus dashboard smoke coverage because the client
  is shared. No such test is part of this Claude plan.
- P3-2 requires a separate synthetic large-session/performance test proving
  bounded request count and latency before changing pagination or concurrency.
- P3-5, only if the threat model expands, requires Unix-platform security tests
  for parent-directory ownership, socket mode, peer authorization, and safe
  category-only failure. No security-hardening test is justified by the current
  contract alone.
- P3-6 requires a separate UI test plan for flag/environment precedence,
  invalid-value behavior, `--help`, Nerd/plain render snapshots, and measured
  display width. These tests must not be added by silently editing T04's dirty
  `main.rs` block.

## Verification Checkpoints

| After step | Verify | Fail action |
|---|---|---|
| Worktree capture | `git status --short`, `git diff --stat`, and path ownership match the pre-plan snapshot plus only approved remediation paths | Stop; do not stage or rewrite unrelated dirty files |
| Envelope contract | Hook unit test proves escaped overflow is a categorized drop and direct serialization cannot panic | Fix the T02 API/test seam before touching runtime tests |
| Delivery regression | Ingress and built-helper tests prove exit 0, no stdout, no payload log, and no frame for overflow | Treat any panic, write, or value leak as a release blocker |
| Manifest ownership | Direct `fs` feature is present and `cargo check -p dashboard --locked` passes | Correct the dashboard manifest; do not rely on transitive features |
| Citation/precision sweep | No stale T04 heading; every owned fallback is cited; byte wording matches `len()` | Correct comments/spec before full test run |
| Claude regression suite | T02/T03/T04 targeted tests and all dashboard targets pass | Diagnose the first failure; do not weaken the new assertion |
| Workspace quality | workspace test/check/clippy/fmt/diff checks pass, with existing dirty files preserved | Separate any baseline failure from new remediation failure |
| Boundary review | No OpenCode, icon, T05, credential, or transcript file changed | Remove scope creep from the proposed patch, never reset user work |

## Migration And Rollback Notes

There is no data migration. The protocol remains version 1, the JSON envelope
shape is unchanged, and valid frames remain byte-for-byte equivalent. The only
behavioral change is that an input previously capable of panicking now drops
successfully as required by R15/R16.

The manifest change is build metadata only. No lockfile update is expected from
adding an already-resolved Tokio feature, but the implementation must inspect
the actual diff rather than assume this.

The preferred rollback is forward-only because reverting the T02 change
reintroduces a known hook failure. If a rollback is unavoidable, revert only
the remediation commit, preserve the dirty worktree, and explicitly accept the
R16 regression for that rollback window. Never use `git reset --hard`, checkout
over user files, or amend the historical task commits.

Deferred OpenCode, socket-security, and UI changes must have independent
rollback units and must not be bundled into this Claude remediation.

## Acceptance Criteria

- [ ] A field-bounded but escape-heavy Claude payload cannot panic the parser,
      serializer, delivery path, or `dashboard claude-hook` process.
- [ ] An escaped serialized envelope over `MAX_ENVELOPE_BYTES` is dropped whole
      with an explicit category, no socket operation, no partial write, no
      stdout, and no payload value in stderr.
- [ ] A valid envelope exactly at the byte bound remains accepted and decoded;
      no truncation or protocol-shape change is introduced.
- [ ] `MAX_HOOK_INPUT_BYTES`, `MAX_SESSION_ID_LEN`, `MAX_CWD_LEN`, and
      `MAX_ENVELOPE_BYTES` documentation consistently states byte semantics.
- [ ] `crates/dashboard/Cargo.toml` directly declares Tokio's `fs` feature, and
      dashboard/workspace compilation passes with locked dependencies.
- [ ] The `listener.rs` fallback citation names the existing T04 design section
      `Bounds And Shutdown`; the T04-owned citation sweep finds no stale section.
- [ ] The listener cleanup comment states the identity/type guard and its
      unavoidable path race without claiming impossible atomic safety; existing
      replacement tests remain green.
- [ ] The current unstaged icon/OpenCode files are byte-for-byte untouched by
      the Claude remediation.
- [ ] `limit=500`, sequential message-fetch optimization, socket hardening,
      icon defaults, icon help, and icon width changes remain unimplemented and
      explicitly assigned to their decision gates.
- [ ] T01c, all four T05 deferrals, no-global-config/no-transcript rules, and
      T05 authenticated/privacy ownership remain unchanged.
- [ ] All targeted and workspace quality commands in the testing strategy pass
      without claiming authenticated Claude coverage.

## Task Breakdown

### Task 1: Make envelope serialization fallible

- **What:** Change the T02 serialization seam to report an oversized complete
  wire frame, preflight parser-produced records, and make delivery return an
  explicit non-panicking category before filesystem/socket work.
- **Files:** `crates/dashboard/src/claude/hook.rs`
- **Depends on:** none after the worktree boundary is captured
- **Agent:** senior implementer
- **Verify:** hook unit tests cover escaped overflow, exact-boundary success, and
  direct serialization without panic.

### Task 2: Add ingress and helper regression coverage

- **What:** Prove the escaped overflow through the real T02 ingress and built
  `dashboard claude-hook` paths, including no frame, exit 0, stdout silence,
  and category-only logging.
- **Files:** `crates/dashboard/tests/claude_ingress.rs`,
  `crates/dashboard/tests/claude_runtime.rs`
- **Depends on:** Task 1
- **Agent:** test-focused implementer
- **Verify:** targeted ingress and runtime tests pass on real temporary Unix
  sockets.

### Task 3: Align wire callers and byte-boundary tests

- **What:** Update T03 wire tests and test helpers for the fallible serializer,
  preserve exact-frame behavior, and add multibyte byte-limit cases. Correct
  wire comments without changing decoder policy.
- **Files:** `crates/dashboard/src/claude/wire.rs`
- **Depends on:** Task 1
- **Agent:** bounded implementer
- **Verify:** `cargo test -p dashboard --lib claude:: --locked` passes and decoder
  accepts only the existing version-1 bounded wire contract.

### Task 4: Declare Tokio filesystem ownership

- **What:** Add `fs` to dashboard's direct Tokio feature list and inspect the
  feature tree for direct ownership.
- **Files:** `crates/dashboard/Cargo.toml`
- **Depends on:** none
- **Agent:** mechanical implementer
- **Verify:** manifest inspection, `cargo tree -p dashboard -e features`, and
  `cargo check -p dashboard --locked` pass.

### Task 5: Correct citations and precision claims

- **What:** Replace the invalid T04 design heading, sweep T04-owned fallback
  citations, state UTF-8 byte units in the Claude spec and hook/wire comments,
  and qualify the listener cleanup comment about the path race.
- **Files:** `crates/dashboard/src/claude/listener.rs`,
  `crates/dashboard/src/claude/hook.rs`, `crates/dashboard/src/claude/wire.rs`,
  `docs/specs/dashboard/claude.md`
- **Depends on:** Tasks 1 and 3 for comment/test consistency
- **Agent:** reviewer/implementer
- **Verify:** scoped citation and wording sweep plus format and diff checks.

### Task 6: Resolve separate dirty-worktree decisions

- **What:** Obtain explicit owner decisions for `limit=500` compatibility, the
  OpenCode sequential-fetch follow-up, the hostile-local-user threat model,
  and icon-mode semantics/help/tests. Do not edit source in this task.
- **Files:** current dirty OpenCode/icon files and their owning contracts only
  after approval; no files are changed by this Claude plan
- **Depends on:** none, but must be complete before any dirty work is merged
- **Agent:** product/architecture owner
- **Verify:** each decision has repository evidence or an updated owning spec,
  named owner, and separate implementation scope.

### Task 7: Run the release-boundary verification

- **What:** Run targeted Claude tests, workspace checks, citation sweep, and
  final path-ownership review. Confirm T05 boundaries and the dirty-worktree
  isolation.
- **Files:** no source changes; verification artifacts only if an owner later
  authorizes them
- **Depends on:** Tasks 1-5; Task 6 decisions may remain unresolved only if all
  dirty findings stay out of the patch
- **Agent:** release gate owner
- **Verify:** every checkpoint and acceptance criterion passes, with no claim of
  T05 authenticated coverage.

## Residual Risks And T05 Boundary

The chosen overflow behavior intentionally drops unusual escape-heavy metadata
instead of trying to preserve it. That can reduce Claude visibility for an
event, but it is the specified safe outcome and is preferable to blocking or
failing Claude. The separate 8 KiB envelope bound remains smaller than some
field-bound combinations by design; the preflight makes that state explicit.

The local socket remains unauthenticated at the application layer. User-scoped
path conventions and metadata-only filtering limit the current same-user
scenario, but they do not prove peer authenticity. Any stronger claim requires
the P3-5 threat-model gate and must not be inferred from this remediation.

T05 remains the sole owner of authenticated hook-to-helper-to-socket-to-adapter
to-dashboard proof, successful-turn ordering, async-hook viability, exit-path
reliability, startup-gap/foreground discovery, subagent identity, final
staleness policy, and the four credential-dependent evidence promotions. This
plan neither accesses credentials nor promotes any T05 evidence.

## Recommended Implementation Order

1. Capture the worktree boundary again immediately before implementation and
   create a path allowlist for the remediation.
2. Implement the fallible T02 envelope contract and its unit regression first.
3. Add the real ingress and built-helper escape-heavy feature tests.
4. Update T03 wire callers/tests and precise byte wording.
5. Add the direct Tokio `fs` feature.
6. Correct the citation and listener cleanup claim, then run the scoped citation
   sweep.
7. Run all targeted and workspace quality gates without staging unrelated work.
8. Handle P3-1, P3-2, P3-5, and P3-6 only in separately approved workstreams.

## Questions And Decisions Before Code Changes

- Is the recommended fallible whole-envelope drop, with no truncation and no
  field-limit change, approved for the already-committed T02 defect?
- For P3-1, does the supported OpenCode server contract explicitly accept
  `GET /api/session?limit=500`, and should that shared behavior be retained for
  both the dashboard and MCP bridge?
- If `limit=500` is retained, which owner will add the API evidence/spec and
  compatibility tests, and what is the cursor-pagination trigger?
- Is the intended Claude threat model limited to same-user local processes, or
  must a hostile same-host user be prevented from injecting or connecting to
  the Unix socket?
- For P3-6, are `DASHBOARD_ICONS=plain`, `--icons=plain`, and
  `--no-nerd-font` the final supported interface, what should invalid values do,
  and should the CLI expose `--help` now?
- Does the UI owner want a separate display-width compatibility test before
  adding any Unicode-width dependency?
- Are the existing T09 50+ pagination/message-fetch deferrals still the desired
  release posture, with no OpenCode performance work blocking this Claude fix?
