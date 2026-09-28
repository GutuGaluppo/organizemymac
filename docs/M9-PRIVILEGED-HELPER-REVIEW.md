# M9 — Privileged maintenance helper: design and security review

**Status: not implemented.** The implementation plan says M9 may only proceed after a dedicated
security and permissions review. This document is the input for that review. No privileged code
exists in the repository.

## Why a separate helper

Some maintenance tasks need root: flushing the DNS cache (`dscacheutil -flushcache`, `killall -HUP
mDNSResponder`), rebuilding the Spotlight index (`mdutil -E`), and deleting local Time Machine
snapshots (`tmutil deletelocalsnapshots`). OrganizeMyMac itself must never run as root, and the UI must
never run commands.

## Proposed design

- A launchd daemon registered with `SMAppService.daemon(plistName:)` (macOS 13+), embedded in
  `Contents/Library/LaunchDaemons`. The user approves it once in System Settings → General →
  Login Items; the app only opens that pane.
- The app talks to it over **XPC** (`NSXPCConnection` with a Mach service). The daemon accepts a
  connection only if the client's code signature satisfies a fixed requirement: same Team ID,
  bundle id `dev.galuppo.OrganizeMyMac`, hardened runtime (`setCodeSigningRequirement`, macOS 13+).
- The protocol is a closed enum of operations, never a command string:

  | Operation | Implementation | Input |
  | --- | --- | --- |
  | `flushDNS` | `dscacheutil -flushcache` then HUP `mDNSResponder` | none |
  | `reindexSpotlight(volume)` | `mdutil -E <volume>` | a mounted volume path, validated against `getmntinfo` |
  | `deleteLocalSnapshot(date)` | `tmutil deletelocalsnapshots <date>` | a snapshot date string matching `^\d{4}-\d{2}-\d{2}-\d{6}$` and present in `tmutil listlocalsnapshotdates` |

  Executables are called by absolute path with `posix_spawn`, fixed argv and an empty environment.
- Every call is logged by the daemon (unified logging) and by the app (operation log).
- The daemon exits after 60 s idle; launchd starts it on demand.
- Uninstall: `SMAppService.unregister()` from Settings, and the app's uninstall notes.

## Threat model

| Threat | Mitigation |
| --- | --- |
| Another process on the Mac connects to the daemon | code-signing requirement on the XPC connection |
| A compromised or malicious UI asks for something else | closed operation enum; no strings are executed; inputs validated in the daemon |
| Argument injection | absolute executable paths, fixed argv, strict regex and existence checks |
| Replaced helper binary | the daemon lives inside the signed, notarized app bundle; SMAppService checks the signature |
| Privilege persistence after uninstall | unregister on uninstall; the daemon has no state |
| Destructive surprises | each operation is confirmed in the UI with an explanation; snapshots are listed first (read-only `tmutil listlocalsnapshots` does not need the helper) |

## Preconditions before implementing

1. Developer ID signing and notarization in place (the daemon cannot be registered by an ad-hoc
   signed app).
2. Review of this document, including which operations are worth the risk at all. Flushing DNS and
   reindexing Spotlight rarely help users; deleting local snapshots is the only one that frees space,
   and macOS already purges them under disk pressure.
3. Tests: XPC client-requirement rejection, input validation for every operation, and an
   end-to-end test on a CI Mac with the daemon installed.

## Recommendation

Implement only `deleteLocalSnapshot` first, and only if user research shows the need. Until then,
the app explains local snapshots and links to the Storage pane of System Settings.
