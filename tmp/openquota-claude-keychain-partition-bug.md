# OpenQuota Rewrites Claude Code Keychain ACL and Causes Repeated Password Prompts

## Summary

OpenQuota 0.5.0 reads and refreshes the OAuth credential stored by Claude Code in the macOS login keychain. When OpenQuota persists a refreshed Claude token, it rewrites the keychain item's partition list so that only OpenQuota's signing team can decrypt it.

Claude Code subsequently invokes `/usr/bin/security` to read the credential. Because the rewritten item no longer includes the `apple-tool:` partition, macOS repeatedly displays this authorization dialog:

> `security` wants to use your confidential information stored in "Claude Code-credentials" in your keychain.

Choosing **Always Allow** does not solve the problem. `/usr/bin/security` is already present in the item's legacy application ACL, but the partition list still denies it. A later OpenQuota token refresh also overwrites manual partition repairs.

No credential values were captured during this investigation.

## Confirmed Environment

- macOS login keychain: `~/Library/Keychains/login.keychain-db`
- OpenQuota version: `0.5.0`
- OpenQuota bundle identifier: `io.github.deviffyy.openquota`
- OpenQuota signing team: `DNQ8ZUR59M`
- Claude Code version: `2.1.259`
- Claude Code bundle identifier: `com.anthropic.claude-code`
- Claude Code signing team: `Q6L2SF6YDW`
- Keychain item class: generic password (`genp`)
- Keychain service: `Claude Code-credentials`
- Keychain account: current macOS username

## Runtime Evidence

The process monitor repeatedly captured Claude Code launching this command directly:

```text
security find-generic-password -a ajeesh -w -s Claude Code-credentials
└── parent: ~/.local/bin/claude
```

This lookup was performed by Claude Code itself. It was not launched by a shell hook or cmux hook.

The broken keychain item had the following relevant ACL state:

```text
applications:
  /Applications/OpenQuota.app
  /System/Library/CoreServices/Applications/Keychain Access.app
  /Applications/OpenQuota.app (stale cdhash entry)
  /usr/bin/security

partition_id:
  teamid:DNQ8ZUR59M
```

Although `/usr/bin/security` appeared as an allowed application, the partition list contained only OpenQuota's team ID. It did not contain `apple-tool:`.

## Timestamp Correlation

The keychain item's modification timestamp exactly correlated with an OpenQuota Claude token refresh:

```text
Keychain item mdat:                  2026-09-03 16:36:27Z
OpenQuota token refresh succeeded:  2026-09-03 16:36:27.975Z
```

Relevant OpenQuota log sequence:

```text
2026-09-03T16:36:25.415Z [INFO] [plugin:claude] refresh start (force=false)
2026-09-03T16:36:26.820Z [INFO] [auth:claude] token refresh attempt
2026-09-03T16:36:27.975Z [INFO] [auth:claude] token refresh succeeded
2026-09-03T16:36:28.372Z [WARN] [plugin:claude] refresh failed (... Authentication ...)
```

OpenQuota refreshes providers approximately every five minutes. The credential ACL changed during the successful token refresh.

## Root Cause

The OpenQuota code that persists a refreshed Claude credential updates or replaces the macOS keychain item using OpenQuota's security context. The resulting item receives this partition list:

```text
teamid:DNQ8ZUR59M
```

This discards the partition required by Claude Code's `/usr/bin/security` credential lookup:

```text
apple-tool:
```

The bug is in OpenQuota's credential write path, not in Claude Code's read path. A newly started Claude process reproduced the prompt, ruling out stale authorization state in a long-running Claude process.

## Reproduction

1. Sign in with Claude Code so `Claude Code-credentials` exists in the login keychain.
2. Run OpenQuota with the Claude provider enabled.
3. Wait for OpenQuota to refresh the Claude OAuth token.
4. Inspect the item ACL without printing its password:

   ```bash
   security dump-keychain -a "$HOME/Library/Keychains/login.keychain-db"
   ```

5. Find the exact item whose service is `Claude Code-credentials` and inspect its `partition_id` entry.
6. Start or use Claude Code until it executes:

   ```bash
   security find-generic-password -a "$USER" -w -s "Claude Code-credentials"
   ```

7. Observe the repeated macOS keychain authorization dialog.

Do not print or log the output of the command in step 6; `-w` returns the secret.

## Temporary Recovery

OpenQuota must first be stopped, otherwise a later token refresh can undo the repair.

The item can then be repaired interactively:

```bash
security set-generic-password-partition-list \
  -a "$USER" \
  -s "Claude Code-credentials" \
  -S "apple-tool:,apple:,teamid:DNQ8ZUR59M" \
  "$HOME/Library/Keychains/login.keychain-db"
```

This command prompts for the macOS login-keychain password. Do not pass the password with `-k`, because that exposes it in process arguments and potentially shell history.

After repair, the relevant ACL is:

```text
partition_id:
  apple-tool:, apple:, teamid:DNQ8ZUR59M
```

The exact decrypt-path lookup then exits successfully without an authorization dialog:

```bash
security find-generic-password \
  -a "$USER" \
  -w \
  -s "Claude Code-credentials" \
  >/dev/null
```

## Required OpenQuota Fix

OpenQuota must not narrow or replace the access-control metadata of a credential owned by another application.

Preferred behavior:

- Preserve the existing keychain item's access control and partition list when updating only its secret data.
- Do not delete and recreate `Claude Code-credentials` during token refresh.
- If the macOS API used for updating the value implicitly rewrites access control, explicitly preserve and restore the original access object.
- Never reduce an existing partition list to only OpenQuota's team ID.
- Preserve at least `apple-tool:` for this Claude Code item because the official CLI reads it through `/usr/bin/security`.
- Avoid adding duplicate or stale application ACL entries on repeated refreshes.
- Do not log credential values while diagnosing or testing the fix.

An alternative design is for OpenQuota to avoid writing Claude-owned credentials entirely. It could use refreshed credentials only in memory and leave persistence to Claude Code. That avoids ownership and ACL conflicts but may lose refresh persistence across OpenQuota restarts.

## Acceptance Tests

1. Create or obtain a normal `Claude Code-credentials` item through Claude Code.
2. Record its ACL and partition list, excluding the secret value.
3. Run one forced OpenQuota Claude token refresh.
4. Verify that the keychain item's secret is updated without narrowing its original ACL or partition list.
5. Run multiple refreshes and verify that ACL entries are not duplicated.
6. Verify this exits `0` without showing a keychain dialog:

   ```bash
   security find-generic-password -a "$USER" -w -s "Claude Code-credentials" >/dev/null
   ```

7. Restart Claude Code and repeat the lookup to rule out process-local authorization caching.
8. Restart OpenQuota, refresh again, and repeat the lookup.
9. Verify OpenQuota can still read and refresh Claude usage data.

## Current Machine Mitigation

During investigation, OpenQuota was stopped and its launch agent was disabled. The credential partition was repaired and the exact Claude-style decrypt lookup completed with exit code `0` and no prompt.

Do not re-enable OpenQuota on this machine until its Claude credential persistence path is fixed, or the next successful token refresh may reintroduce the broken partition list.
