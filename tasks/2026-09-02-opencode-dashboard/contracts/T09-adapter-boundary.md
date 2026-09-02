# T09 — HarnessAdapter boundary, core session model, opencode adapter

**Contract version** — 2 (advisor decomposition-review fix: named the
dedicated last-updated timestamp field R3's window filter reads — see
snapshot-shape list below)

**Context** — goal: build the `HarnessAdapter` boundary (`docs/specs/dashboard/client.md`
R1.3 full) and the one adapter that ships in V1 (opencode, R4/R6.4-R6.6), so that
the core receives fully-rendered, harness-agnostic session snapshots — no raw
SSE event, tool name, or JSON payload ever crosses the boundary · who uses it:
T10 (naming) reads session/project identity and creation time off this task's
types; T11 (render) reads every content field this task's snapshot carries;
T12's main loop drives this task's polling/reconcile loop every frame · scale:
one opencode adapter, `overview.md` R5.8 design center (~8 sessions/4
projects) · criticality: **high** — the delivery profile names the adapter
boundary as one of exactly two foundations "disproportionately expensive to
retrofit" (workflow 2), and this task also fixes the session/project identity
and creation-time shape that T10's claim-map depends on. Get the boundary
right; nothing downstream can cheaply undo a wrong shape here.

**Delivery profile** — `../delivery-profile.md` version 1 · task override: none.

**Boundaries**

- **Owns:** `crates/dashboard/src/` — new modules for the `HarnessAdapter`
  trait, the core session/project snapshot types, the project-identity
  resolver, and the opencode adapter implementation (exact file names are the
  implementer's choice; keep the adapter's opencode-specific code physically
  separate from the trait/model types, per `code-quality`'s encapsulation
  rule). `crates/dashboard/Cargo.toml` — add whatever non-TUI dependencies
  this needs (tokio, etc.); **no `ratatui` or any TUI crate** — that's T11's.
  `crates/opencode-client/src/opencode.rs` — extend `SessionInfo` to
  deserialize `location` (at least `.directory`), `projectID`, and `subpath`
  (see "Extending SessionInfo" below).
- **Must not touch:** `crates/opencode-bridge/**` (the MCP binary — this
  task's `SessionInfo` change must be additive-only and not require any
  change there; if it does, the change is wrong), any rendering/layout code
  (T11), any naming/claim-map code (T10), the main event loop / terminal
  handling / keyboard input (T12), `docs/specs/**`.

**Extending `SessionInfo`** — `crates/opencode-client/src/opencode.rs:29`
currently deserializes only `id, outcome, time, cost, tokens, title`, even
though the opencode server sends `location`, `projectID`, and `subpath` on
every session response (confirmed by the T01 spike;
`tmp/2026-09-02-project-identity-spike/EVIDENCE.md`, "Why direct curl instead
of the MCP tools for part 1"; also `deferred.md`'s T01 entry). Add the three
fields with `#[serde(default)]` so `opencode-bridge`'s existing 29 tests and
behavior are unaffected — this is a pure addition to a shared type, not a
reshape. R1.6's project identity is resolved from `location.directory`, not
from opencode's own `projectID` (that field collides every non-git directory
into opencode's own `"global"` bucket — confirmed in EVIDENCE.md — and must
never be used as the dashboard's project key).

**Session/project identity and the resolver (R1.5, R1.6)**

- Session identity is `(harness kind, harness-native session id)` — the full
  tuple, not the raw id alone (R1.5).
- Project identity is the canonical git-repository toplevel path of a
  session's working directory, falling back to the canonicalized working
  directory itself when there's no repo — never opencode's own `projectID`.
  Port the canonicalization logic (symlink resolution, trailing-slash strip,
  case-normalization stub, canonicalizing both the input dir and git's
  `--show-toplevel` output) from the already-verified T01 spike code under
  `tmp/2026-09-02-project-identity-spike/` — re-derive it from that evidence,
  don't design it again from the spec text alone.
- **Caching obligation** — resolving to git toplevel spawns a subprocess. A
  session's directory→toplevel mapping cannot change during that session's
  lifetime; cache it per session, don't re-spawn `git` on every snapshot or
  redraw (`client.md` R1.6's "Caching obligation" note).

**Session snapshot shape** — the core-facing type this task hands to T10/T11
must carry, at minimum, everything the render regime table (`layout.md`
R5.3) and the naming scheme (`visuals.md` R6.8) need to read with zero
opencode-specific knowledge:

- session identity, project identity, `parentID` (subagent sessions point at
  their parent — R5.6 — and are otherwise ordinary snapshots)
- attention state: `Running | NeedsYou { question: bool } | Idle`, plus the
  right timestamp basis for each (turn-start for running, turn-end for
  needs-you, last-update for idle) — **store the timestamp, not a pre-rendered
  "Nm ago" string**; render redraws every ~250ms per `client.md` R1.4's own
  design note, so a baked string goes stale between snapshots
- current action line (already-rendered text per R6.5 — never the raw tool
  name/args)
- wire title, final assistant text (for the question/needs-you elastic
  blocks), last user prompt text (`you: <text>`)
- files touched this turn, a bounded recent-actions ring (oldest-first, for
  the extended `running` block's elastic list)
- session creation time (T10 needs this for claim-order resolution)
- **a dedicated last-updated timestamp**, distinct from the per-state
  elapsed-time basis above — this is what `overview.md` R3's active/idle
  window filter reads (T12 computes "active" from time-since-this-field,
  never from an opencode-native "updated" value). Source it from the wire's
  `SessionTime.updated` (`crates/opencode-client`'s `SessionInfo.time`,
  already deserialized) and refresh it on every snapshot the adapter emits
  for that session, SSE-driven or reconcile-driven alike.
- a "gone" tombstone path (see Staleness below)

**The opencode adapter's own mechanism (R4, R6.4-R6.6)**

- `GET /api/session` (paginated) for the full list, `GET /api/event` SSE
  (via `opencode-client`'s existing `EventStream`, see `sse.rs`) for latency,
  plus a 60-second periodic reconcile sweep as the correctness source — SSE
  is not the sole source of truth (R4). A dropped SSE connection self-heals
  on the next sweep.
- Per-session `call_id → name` map: populated on `session.tool.input.started`
  (`{id, name}`), consumed on `session.tool.called` (`{id, input}`), then
  dropped — this is how the two are joined into one rendered action line
  (R6.4). The core-facing snapshot only ever sees the rendered result.
- Action-line mapping (R6.5, verbatim): `shell` → `input.command`, collapse
  newlines to `" · "`, truncate to tile width with `…`; `edit` → `"editing: "`
  + basename of `input.path` (fall back to full relative path if it fits);
  any other/unrecognized name → `"running: <name>"`.
- Confirmed wire tool vocabulary (R6.6): `shell, edit, grep, write, skill,
  subagent, glob, patch, read` — field shapes for 7 of the 9 are documented
  in `docs/internal/opencode-sse-event-catalog-2026-09-01.md` §3. `glob` and
  `patch` are unconfirmed on the wire; treat them (and every tool beyond
  `shell`/`edit`) with R6.5's generic `"running: <name>"` fallback — no
  per-tool detail beyond that mapping is in scope (R6.6's phase 2 is deferred).

**Staleness (R1.7) — mechanism only, not the display rule.** Track
time-since-last-snapshot per session. The opencode adapter's reconcile sweep
gives a concrete tombstone signal: when a previously-known session no longer
appears in `GET /api/session`, emit a "gone" tombstone for it. The exact
staleness *threshold* and its on-screen treatment are `[REVIEW: OPEN]` in the
spec and explicitly not this task's job to invent — do not add a dimming
rule, a dropped-from-view rule, or a numeric threshold. The only thing this
task must do is make the tombstone path exist and fire on an explicit
session-gone signal, because `visuals.md` R6.8's claim-map (T10, next) needs
it to release claims — a session that vanishes with no tombstone leaks its
claimed name forever.

**Conventions** — `cargo build --workspace --all-targets --locked`, `cargo
test --workspace --all-targets --locked`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo fmt --all -- --check` (per
`CONTRIBUTING.md`, same as T08). `SessionInfo`'s new fields are additive only
— if making them compile clean requires touching `opencode-bridge`, the
design is wrong, not the fix.

**Skills to read and apply** — `code-quality`, `software-design` (the
`HarnessAdapter` trait is a real component boundary — get the layer stack
right: trait/model types must not import anything opencode-specific).

**Acceptance — done when:**

1. `HarnessAdapter`-shaped trait exists; the opencode adapter is its only
   implementation; the core-facing snapshot type contains no opencode wire
   type, raw tool name, or `serde_json::Value` anywhere in its public shape.
2. Session snapshot carries every field listed above, with timestamps (not
   pre-rendered elapsed strings) for anything time-based.
3. Project identity resolution matches R1.6 exactly (ported from the T01
   spike's verified logic, not re-derived), including the caching
   obligation — a test proves the resolver isn't re-spawning `git` per call
   within one session's lifetime.
4. `SessionInfo` (opencode-client) carries `location`/`projectID`/`subpath`,
   additive-only; `cargo test --workspace` still passes the same 29
   `opencode-bridge` tests unchanged.
5. Tool-call correlation (R6.4) and action-line rendering (R6.5/R6.6) behave
   exactly as specified, proven with fixture SSE payloads (reuse or extend
   the shapes in `docs/internal/opencode-sse-event-catalog-2026-09-01.md`) —
   not a live-server-only claim.
6. R4's reconcile sweep runs on a fixed interval independent of SSE health;
   a test proves a dropped/absent SSE connection still gets corrected by the
   next sweep.
7. A tombstone fires when the reconcile sweep finds a previously-known
   session gone; nothing yet consumes it (T10 does) but the signal exists
   and is observable in a test.
8. No TUI dependency anywhere in this task's Cargo.toml changes.
9. `cargo build/test/clippy/fmt` clean workspace-wide; nothing outside this
   contract's owns-list touched.

**Gate** — report-only (refine-loop).

**Dependencies** — T08 (workspace shape — gated, `ac6962b`).

## Review Frame

*Authored by the advisor. Governs disposition and review budget — never what
the reviewer may look at or discover. It cannot suppress credible severe
evidence.*

**As of** — contract version 2

**Context** — The adapter boundary is one of two non-retrofittable foundations.
This task also fixes the snapshot shape T10-T12 all read.

**Expectations** — Core-facing snapshot type free of opencode wire types (AC 1,
workflow 2). Boundary correctly placed, not just compiling. Identity resolver
matches verified spike (AC 3). SessionInfo additive-only, opencode-bridge
untouched (AC 4, 9).

**Depth** — Boundary placement and snapshot shape are the release-bar surface;
adapter-internal SSE processing and reconcile intervals are reversible behind
the boundary — out of budget unless severe. 2 passes. Say clean if it's clean.
