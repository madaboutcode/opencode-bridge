<!-- contracts/T03-client-spec.md. Read by both implementer and reviewer. -->

# T03 — client.md (harness-adapter / data-contract spec)

**Context** — goal: write the full technical contract for how sessions get discovered, identified, and normalized before the core ever sees them · who uses it: M3's implementer, building the `HarnessAdapter` trait and the opencode adapter against this as source of truth · scale: one adapter shipped in V1 (opencode), designed for future adapters but none built yet · criticality: moderate — this is the file M3 will be briefed from most literally; a wrong data shape here means M3 builds the wrong interface, but a human reviews before anything ships

**Boundaries** — owns: `docs/specs/dashboard/client.md` (new file) · must not touch: any other file under `docs/specs/`, the requirements doc (read-only), `src/`, `crates/`

**Conventions** — read `docs/specs/CLAUDE.md` (written by T02 — if it doesn't exist yet when you start, T02 hasn't gated, wait) before writing. Follow its format: R#/R#.# numbering carried over from the requirements doc (keep the same numbers — this is a reorganization into spec form, not a renumbering), one co-located Given/When/Then scenario per requirement, `[REVIEW: ...]` markers for carried-forward OPEN items.

**Source material** — `tasks/2026-09-01-opencode-dashboard.requirements.md`, sections: R1.3 (full — the `HarnessAdapter` boundary: what an adapter owns, what the core never sees, e.g. no raw tool name/args), R1.4 (session snapshot model: upsert-by-key or "gone" tombstone, one shared channel, no fine-grained event stream — state why event-sourcing was rejected, it's load-bearing context for anyone tempted to add it back), R1.5 (session identity tuple), R1.6 (project identity: git-toplevel resolution + fallback + canonicalization + the caching-obligation implementation note + worktree/subfolder/subagent behavior, all confirmed by the T01 spike — reference `tmp/2026-09-02-project-identity-spike/EVIDENCE.md` as the evidence source), R1.7 (staleness rule — mark the threshold/treatment as `[REVIEW: OPEN, see requirements doc]`, don't invent one), R1.8 (harness-tag slot — mark placement as `[REVIEW: OPEN]`), R4 (the opencode adapter's own mechanism: REST + SSE + 60s reconcile sweep), R6.4 (call_id→name correlation, confirmed field shapes), R6.5 (coarse tool→action-line mapping table), R6.6 (full tool vocabulary, phase-2 items explicitly `[REVIEW: OPEN]`). Also note (from the requirements doc's Open Questions): the MCP bridge's own `SessionInfo` struct doesn't currently deserialize `location`/`projectID`/`subpath` — flag this as an M3 implementation gap the adapter will need to close, `[REVIEW: see deferred.md]`.

**Skills to read and apply** — `writing-specs`

**Acceptance — done when** — `client.md` covers every R-number listed in Source material with the original numbering preserved, each with a co-located scenario, all confirmed OPEN items marked `[REVIEW: ...]` rather than resolved, and the file is self-contained enough that an M3 implementer could build the `HarnessAdapter` trait and the opencode adapter from it without re-reading the requirements doc.

**Gate** — report-only (refine-loop)

**Dependencies** — T02 (needs `docs/specs/CLAUDE.md`'s conventions to exist first)
