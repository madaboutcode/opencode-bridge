---
name: Bug report
about: Something in opencode-bridge isn't working as documented.
title: "[bug] "
labels: ["bug"]
assignees: []
---

## What happened

<!-- One paragraph. "I did X, expected Y, got Z." -->

## Reproduction

The smallest `tools/call` that triggers it:

```json
{"name": "opencode_task", "arguments": { ... }}
```

Or, for an MCP-handshake problem, the JSON-RPC frames you piped in.

## Versions

- opencode-bridge (commit SHA or `--version`):
- opencode2 (`opencode2 --version`):
- OS:
- How installed (cargo install? source build?):

## Bridge stderr

<!-- Paste the relevant bridge stderr. If you don't have it, say so — it
helps us know whether the bug is upstream of the MCP layer. -->

```text
[paste here]
```

## Bridge stdout

<!-- stdout is the MCP protocol stream; usually one JSON line. Paste if
relevant — note that session ids and prompt text may appear here. -->

```text
[paste here]
```

## Anything else

<!-- Logs from opencode2 itself, related issues, etc. -->
