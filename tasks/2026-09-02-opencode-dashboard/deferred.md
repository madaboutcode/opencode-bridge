# Deferred findings — opencode-dashboard run

Real-but-out-of-scale or out-of-boundary findings, parked here by each task's
runner at gate time. Not a backlog of nice-to-haves — only things a reviewer
or implementer actually found and judged real.

## T01 — project-identity spike

- The MCP bridge's own `SessionInfo` struct (`src/opencode.rs:29`) doesn't
  deserialize `location`, `projectID`, or `subpath` even though the opencode
  server sends them on session metadata — the spike had to curl the server
  directly for evidence instead of using `opencode_sessions`. Real gap,
  relevant to M3's opencode adapter work, out of scope for T01 (its boundary
  excludes `src/`).
- R1.6's "normalize case on case-insensitive filesystems" clause is untested —
  the machine used for this spike has a case-preserving filesystem, so no
  fixture produced an actual case mismatch to exercise. Recorded as an
  explicit gap in `tmp/2026-09-02-project-identity-spike/EVIDENCE.md`, not
  silently passed. Worth a dedicated check if a case-insensitive-filesystem
  environment becomes relevant to the dashboard.
