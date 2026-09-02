# T01 — project-identity resolution spike: evidence

Date: 2026-09-02. Server: opencode2 `0.0.0-beta-18743`, paired on
`127.0.0.1:49374` (password from `~/.config/opencode/service.json`, per
`docs/internal/opencode-wire-behavior-2026-08-27.md`'s method). All curl
commands below used `-u opencode:<password>` basic auth against
`http://127.0.0.1:49374/api`, matching the auth shape in `src/opencode.rs`.

**Top-line verdict: R1.6 confirmed as written, with one untestable clause:**
case normalization on case-insensitive filesystems is not exercisable on
this machine's case-preserving filesystem (no fixture produces a case
mismatch — see "Untested corner of R1.6's canonicalization clause" below).
Everything else in R1.6 is confirmed. There's also one clarifying addition
worth folding in (see "Note on opencode's own `projectID`" below) and one
observation about `subpath` that doesn't change R1.6 but is useful context
for the adapter implementer.

## Why direct curl instead of the MCP tools for part 1

`mcp__opencode-bridge__opencode_sessions` can't answer this question: the
bridge's own `SessionInfo` struct (`src/opencode.rs:29`) deserializes only
`id, outcome, time, cost, tokens, title` — it has no field for `location`,
`projectID`, or `subpath`, so those keys are silently dropped by serde even
though the server sends them. This is a real gap in the bridge's own
session-shape modeling, worth flagging separately (out of this spike's
scope to fix). I used direct curl for checks 1–3 and curl-on-top-of-the-MCP-
launched-session for check 4, per the contract's "whichever is faster and
gives equivalent evidence."

## Part 1 — wire check

### Check 1: plain top-level session, directory = repo root

Created via `POST /session` with
`location.directory=/Users/ajeesh/projects/madaboutcode/opencode-mcp`, no
prompt sent — metadata read directly off the create response (inert, no
work performed, per the contract).

```json
{
  "id": "ses_f9fa817d6ffeZyL0m47xV870cy",
  "projectID": "4f05975d561a795e3b2daa05e492a01a59ba6b66",
  "location": {"directory": "/Users/ajeesh/projects/madaboutcode/opencode-mcp"}
}
```
No `subpath` key present at all.

**Matches R1.6:** yes — `location.directory` is exactly the directory
requested, present at creation, no work needed.

### Check 2: plain top-level session, directory = subfolder (`docs/internal`)

Created via `POST /session` with
`location.directory=/Users/ajeesh/projects/madaboutcode/opencode-mcp/docs/internal`,
no prompt sent.

```json
{
  "id": "ses_f9fa7e2f7ffecI8ooNg5YR3D70",
  "projectID": "4f05975d561a795e3b2daa05e492a01a59ba6b66",
  "location": {"directory": "/Users/ajeesh/projects/madaboutcode/opencode-mcp/docs/internal"},
  "subpath": "docs/internal"
}
```

**Matches R1.6:** yes. `location.directory` is the subfolder, as requested.
`projectID` is the SAME hex hash as check 1 — opencode's own project
grouping already treats the subfolder session as part of the repo-root
project, consistent with R1.6's "session launched in a monorepo subfolder is
grouped under that repo's single project box" clause. (R1.6's own
canonicalizer, tested in part 2, arrives at the same grouping independently,
via git toplevel rather than opencode's internal `projectID` — see the note
below on why R1.6 is still right not to depend on `projectID` directly.)

### Check 3: parent session delegates to a subagent

Parent created at directory = repo root (no prompt yet, inert):
```json
{"id": "ses_f9fa758a3ffeN3znml6jfupqD7", "location": {"directory": "/Users/ajeesh/projects/madaboutcode/opencode-mcp"}}
```

Parent then given one prompt: "Use the subagent tool once, agent=clerk,
description=Ping pong reply, prompt=Reply with the single word: pong. Do not
use any tools. Do not use any other tool yourself, and do not read, write,
edit, or run anything else." This is the same ping-pong pattern already used
in `docs/internal/opencode-sse-event-catalog-2026-09-01.md` §3's `subagent`
confirmation — a real tool call (subagent delegation can't be observed any
other way) but zero file/shell tool use by either the parent or the child.

Parent's tool-call result:
```json
{"name": "subagent", "state": {"metadata": {"sessionID": "ses_f9fa732c6ffePEQK00bkw8C1Vm", "status": "completed"}}}
```

Child session detail (`GET /session/ses_f9fa732c6ffePEQK00bkw8C1Vm`):
```json
{
  "id": "ses_f9fa732c6ffePEQK00bkw8C1Vm",
  "parentID": "ses_f9fa758a3ffeN3znml6jfupqD7",
  "projectID": "4f05975d561a795e3b2daa05e492a01a59ba6b66",
  "location": {"directory": "/Users/ajeesh/projects/madaboutcode/opencode-mcp"}
}
```

**Matches R1.6:** yes. The child inherits the parent's `location.directory`
and `projectID` exactly — no divergence to reconcile. A dashboard grouping
by canonicalized directory would put parent and child in the same project
box automatically, no special-casing needed for subagents at the
project-identity layer (subagents are still handled specially at the
session-identity/rendering layer per R1.5/R5.1, just not here).

### Check 4: explicit `directory` param via the MCP bridge, differing from the bridge's own cwd

The opencode-bridge MCP process's own cwd is
`/Users/ajeesh/projects/madaboutcode/opencode-mcp` (confirmed via
`lsof -p <bridge pid> | grep cwd`). Launched a session through
`mcp__opencode-bridge__opencode_task` with `directory` explicitly set to
`.../docs/internal` (different from the bridge's cwd) and an inert prompt
("Reply with the single word: pong. Do not use any tools."), then read the
raw session back with curl:

```json
{
  "id": "ses_f9fa69c73ffeEQ7ivdVt9xxWMx",
  "title": "cc-bridge:5924:t01-spike-check4-bridge-directory-param",
  "location": {"directory": "/Users/ajeesh/projects/madaboutcode/opencode-mcp/docs/internal"},
  "subpath": "docs/internal"
}
```

**Testable, and it matches R1.6:** `location.directory` reflects the
explicit `directory` param passed to the bridge tool, not the bridge
process's own cwd and not the opencode2 server process's own cwd (which is
`/Users/ajeesh`, confirmed separately via `lsof`). Bonus, unrelated to R1.6:
this also re-confirms the sse-event-catalog's `cc-bridge:<pid>:<slug>` title
format finding.

## Part 2 — canonicalizer check

Implemented in `src/main.rs` as `resolve_project_identity(dir: &Path) ->
io::Result<PathBuf>`: canonicalize `dir` (`std::fs::canonicalize` — resolves
symlinks, strips trailing slash, requires the path to exist), run
`git -C <canon_dir> rev-parse --show-toplevel`, and if that succeeds,
canonicalize ITS output too (needed — see check 2.1) and return it;
otherwise return the canonicalized input. Full output of `cargo run`,
reproduced verbatim:

```
2.1 non-git temp dir
  input:    ".../T/project-identity-spike-nogit-79221"
  resolved: "/private/var/folders/.../T/project-identity-spike-nogit-79221"

2.2 repo root
  input:    "/Users/ajeesh/projects/madaboutcode/opencode-mcp"
  resolved: "/Users/ajeesh/projects/madaboutcode/opencode-mcp"

2.3 repo subfolder (docs/internal)
  input:    "/Users/ajeesh/projects/madaboutcode/opencode-mcp/docs/internal"
  resolved: "/Users/ajeesh/projects/madaboutcode/opencode-mcp"

2.4 symlink -> repo root
  input:    ".../fixtures/symlink-to-repo-root"
  resolved: "/Users/ajeesh/projects/madaboutcode/opencode-mcp"

2.5a worktree A
  resolved: ".../fixtures/worktree-a"
2.5b worktree B
  resolved: ".../fixtures/worktree-b"
2.5 worktrees distinct identities: true
```

### Check 2.1: plain non-git temp directory

**First attempt was a fixture bug, not a resolver bug**, worth recording
because it's exactly the kind of mistake this check exists to catch: I
initially placed the "non-git" fixture under
`tmp/2026-09-02-project-identity-spike/fixtures/`, which is itself inside
THIS git repo — so `git rev-parse --show-toplevel` correctly found this repo
above it and returned the repo root, which looked like a resolver bug but
was actually a fixture that wasn't really non-git. Fixed by using
`std::env::temp_dir()` (real system temp dir, outside any repo) instead.

With the corrected fixture: resolves to its own canonicalized path
(`/var/folders/...` → `/private/var/folders/...` — macOS's `/var` is itself
a symlink to `/private/var`). **Matches R1.6:** yes, exactly the "fall back
to canonicalized working directory" case, and demonstrates why
canonicalizing git's own `--show-toplevel` output (not just the input) is
necessary in a real implementation — the non-git path needed the same
`/var` → `/private/var` resolution as any git toplevel would.

### Check 2.2: this repo's root

Resolves to itself. **Matches R1.6:** yes.

### Check 2.3: subfolder inside this repo (`docs/internal`)

Resolves to the same identity as check 2.2 (repo root). **Matches R1.6:**
yes — exactly the required behavior.

### Check 2.4: symlink pointing at this repo's root

A symlink at `tmp/2026-09-02-project-identity-spike/fixtures/symlink-to-repo-root`
→ repo root. Resolves to the same identity as check 2.2. **Matches R1.6:**
yes — `fs::canonicalize` on the symlink resolves it to the real repo path
before the git lookup runs, so no special-casing was needed.

### Check 2.5: two separate git worktree checkouts

Created via `git worktree add --detach <path> HEAD` twice, under this
spike's own `fixtures/` directory. Both resolve to their own path (not to
the main repo's toplevel and not to each other) — `git rev-parse
--show-toplevel` run from inside a worktree correctly returns that
worktree's own root, not the main checkout's. Result: `worktree-a` ≠
`worktree-b`, both ≠ main repo root. **Matches R1.6:** yes — "two git
worktrees of the same repository get separate project boxes," exactly as
specified. Both worktrees were removed (`git worktree remove`) and pruned
(`git worktree prune`) before this report was written; `git worktree list`
now shows only the main checkout.

### Untested corner of R1.6's canonicalization clause: case normalization

R1.6 says "normalize case on case-insensitive filesystems." This machine's
filesystem (macOS default APFS) is case-insensitive but case-PRESERVING —
`std::fs::canonicalize` does not fold case; it returns whatever case is
stored on disk regardless of what case you passed in on the way in. I did
not build or test a separate case-folding step, because none of the 5
fixtures exercise a genuine case mismatch (e.g. opening
`/Users/AJEESH/...` vs `/Users/ajeesh/...` and expecting them to compare
equal) — that would need either a literally-different-case path to exist on
disk (it doesn't) or an explicit `.to_lowercase()`-style normalization added
on top of `canonicalize`, which is an implementation decision for whoever
builds the real resolver, not something this spike's fixtures could
observe either way. Flagging this as a specific, scoped gap rather than
silently marking case normalization as passed by omission.

## Note on opencode's own `projectID`

Not a correction to R1.6 — R1.6 already explicitly rejects relying on
"an adapter-specific placeholder (e.g. opencode's own `global` project id)"
— but worth recording as context: in checks 1–4, opencode's own internal
`projectID` (a 40-char hex hash, not the literal `"global"` seen in the
sse-event-catalog spike's `/tmp` sessions) already happens to group
repo-root and subfolder sessions together, matching what R1.6's own
canonicalizer independently arrives at via git toplevel. This is a
convenient cross-check that R1.6's chosen definition lines up with
opencode's own internal grouping for the git-repo case — but R1.6 is still
right to build its own canonicalizer rather than read `projectID` directly,
since the only observed non-git behavior for `projectID` is the literal
string `"global"` for every non-git directory (collision across unrelated
projects), which is exactly the fallback R1.6 was written to avoid.

## Build/test hygiene

`cargo build`, `cargo test` (5/5 pass), `cargo clippy --all-targets -- -D
warnings`, and `cargo fmt --check` are all clean inside this directory
(`tmp/2026-09-02-project-identity-spike/`). No files outside this directory
were modified. `git worktree list` shows only the main checkout; no stale
entries under `.git/worktrees/`.
