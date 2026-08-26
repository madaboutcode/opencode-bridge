# Security

## Supported versions

| Version | Supported |
| --- | --- |
| 0.1.x   | yes       |
| < 0.1.0 | no        |

## Reporting a vulnerability

**Please don't file security issues in public GitHub issues.** Mail
**security@madaboutcode.dev** (or open a
[GitHub Security Advisory](https://github.com/madaboutcode/opencode-bridge/security/advisories/new))
with:

- A description of the vulnerability and its impact.
- A minimal reproduction (a `tools/call` payload is ideal).
- Build / commit SHA, opencode2 version, OS.

You'll get an acknowledgement within 72 hours and a fix or mitigation plan
within 14 days for anything in scope.

## Threat model — what this bridge does and doesn't protect against

Worth knowing what the bridge actually handles, because "opencode bridge" can
sound like a sandbox boundary when it isn't one:

- **opencode2 runs with the user's full filesystem permissions.** The bridge
  forwards `directory` to opencode; whatever you tell opencode to touch, it
  touches. The bridge does not sandbox it.
- **The CC callback channel uses the CC inbox socket.** Anyone who can write
  to `$CLAUDE_CODE_MESSAGING_SOCKET` can post into the launching CC session.
  The bridge only writes on opencode's terminal events for sessions it
  registered itself.
- **The bridge never re-authenticates opencode beyond Basic auth** with the
  creds from `opencode2 pair`. If the opencode service is compromised, the
  bridge will faithfully forward whatever it returns. Treat `opencode2 pair`'s
  output as trust-equivalent to the opencode server itself.
- **No transport encryption to opencode.** `opencode2 pair` returns a
  `127.0.0.1` URL — local-only by convention. Don't point the bridge at a
  non-loopback opencode service without understanding the implications.
- **Stderr logs may include session ids and short prompt text.** Don't ship
  stderr to a public log without redacting.
