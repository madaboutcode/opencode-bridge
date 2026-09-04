# Spec Delta — 2026-09-03-claude-dashboard-support (T04)

Cycle-transient record of every spec change made by the T04 runtime pass.
Reviewer: verify each declaration against the actual diff; changes are
intent-driven (runtime wiring landed) not implementation-driven.

## MODIFIED

- **R11** (claude.md): added the exact helper command `dashboard claude-hook`, the
  manual settings shape/scope (each supported event bound to that command, in
  user- or project-level Claude hook settings), and that normal-mode startup
  opens the listener before adapters.
  reason: T04 wires the hook command and listener into the runtime; the
  consumer-visible install path is now concrete.

- **R12** (claude.md): clarified that removal is simply deleting the hook
  entries, and added that dashboard shutdown closes the listener and removes
  its socket.
  reason: T04 listener cleanup is now observable; "nothing to un-write" now
  also covers the listener's own socket.

- **R16** (claude.md): before -> after — added the listener-side availability
  contract (bind failure or missing user-scoped path disables only Claude
  monitoring; the OpenCode dashboard continues), the bounded degraded
  connection behavior (fixed concurrency bound; malformed / unknown-version /
  unknown-event / out-of-bounds / oversized / unterminated / silent / multi-
  frame connections dropped category-only with later connections unaffected),
  and best-effort socket cleanup on dashboard shutdown. The existing
  half-second delivery deadline and exit-0 guarantee are unchanged.
  reason: T04 makes the listener's startup, degraded handling, and shutdown
  consumer-visible runtime behavior.

- **R17** (claude.md): added one clarifying sentence that wiring the listener
  into startup (T04) changes none of the completeness/authenticated boundary;
  the T05 gate and all four deferrals remain.
  reason: prevent a misreading that "runtime wired" implies completeness.

- **R1.3** (client.md): removed the stale claim that the Claude hook adapter
  is "experimental until its listener wiring lands (T04)"; the listener is
  wired into startup, the adapter remains opt-in, and authenticated evidence
  still depends on T05.
  reason: T04 landed the listener wiring.

- **"The Claude hook adapter (opt-in, experimental)" section** (client.md):
  before -> after — "no runtime listens yet, so Claude sessions do not appear
  in any default dashboard render" -> normal startup opens the listener and
  starts the adapter; sessions still appear only after the user configures
  hooks; the T05 [REVIEW] marker "Until T04 wires startup and T05 closes" ->
  "Until T05 closes".
  reason: T04 landed the listener wiring; opt-in and T05 markers preserved.

- **R1.8** (client.md): removed the parenthetical "off until its listener is
  wired (T04)"; the harness-tag slot is hidden by default because the Claude
  adapter is opt-in.
  reason: T04 landed the listener wiring.

- **R1.3 summary** (overview.md): removed "is experimental until its listener
  startup wiring lands (T04)"; listener is wired with T04, opt-in and
  experimental-pending-T05 remain. The adjacent [REVIEW] block no longer says
  "never active at dashboard startup"; it now says the listener is wired and
  monitoring stays opt-in with T05 owning staleness/authenticated evidence.
  reason: T04 landed the listener wiring.

- **File map row for `claude.md`** (overview.md): removed "experimental until
  T04 listener wiring"; now "listener wired into startup with T04; opt-in and
  experimental pending T05".
  reason: T04 landed the listener wiring.

- **Spec-tree file-count reference** (client.md): before -> after — "full
  five-file map" -> "full six-file map".
  reason: T02 added `claude.md` as the sixth dashboard spec; T04 corrected the
  remaining owned reference so the client spec agrees with the registered tree.

No requirements were ADDED or REMOVED; no seventh spec file was created.

## POST-T04 — 2026-09-04 (editorial correction, `04a7cf5`)

- **R15** (claude.md): "128 characters" / "4096 characters" -> "128 UTF-8
  bytes" / "4096 UTF-8 bytes" for `session_id` and `cwd` bounds.
  reason: editorial correction, not a rule-value change. `hook.rs`'s
  `valid_session_id`/`valid_cwd` used `value.len() <= MAX_SESSION_ID_LEN` /
  `MAX_CWD_LEN` at the T04 gate commit `fd83209`, and still do — verified by
  diff, the comparison is byte-for-byte identical before and after
  `04a7cf5`. `str::len()` in Rust is always UTF-8 byte length, never a
  character count, so the enforced bound never moved; only the spec's prose
  and doc-comment wording were wrong and are now corrected to match what the
  code always did. No T02 gate reopening required.
