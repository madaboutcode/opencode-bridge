<!-- gates/T01-report.md. Written by the task's runner; it is the conductor's entire
     gate and must stand alone. -->

# T01 — gate report

**Conformance:** yes — reviewer's explicit verdict: the work meets the contract's acceptance criteria. All 9 required checks (4 wire + 5 canonicalizer) have explicit recorded outcomes with real observed values; top-line verdict "R1.6 confirmed as written" (with the case-normalization caveat, see Deferred) is justified by the evidence presented.
**Passes:**
- Pass 1 — reviewer found two minor, non-blocking issues: (1) `from_utf8_lossy` on git command output in the resolver silently mangles invalid UTF-8 instead of erroring; (2) the top-line verdict line said "R1.6 confirmed as written" without carrying the case-normalization caveat that the detailed write-up already documented. Fixed (2): edited `EVIDENCE.md`'s top-line verdict to state the caveat explicitly. (1) triaged as below the depth line, not fixed — see Skipped.
- Pass 2 — reviewer re-read the fixed top-line verdict and confirmed it accurately reflects the case-normalization gap; no new findings on the changed area. Verdict: yes.
**Residuals:** none.

**Deferred:**
- The MCP bridge's own `SessionInfo` struct (`src/opencode.rs:29`) doesn't deserialize `location`, `projectID`, or `subpath` even though the opencode server sends them on session metadata — the spike had to curl the server directly for evidence instead of using `opencode_sessions`. Real gap, relevant to M3's opencode adapter work, out of scope for T01 (its boundary excludes `src/`).
- R1.6's "normalize case on case-insensitive filesystems" clause is untested — the machine used for this spike has a case-preserving filesystem, so no fixture produced an actual case mismatch to exercise. Recorded as an explicit gap in `EVIDENCE.md`, not silently passed. Worth a dedicated check if a case-insensitive-filesystem environment becomes relevant to the dashboard.

**Skipped:**
- `from_utf8_lossy` on git subprocess output in the resolver (silently replaces invalid UTF-8 rather than erroring) — cosmetic for throwaway spike code run once on one machine against known ASCII paths; contract's stated criticality is low/reversible.
