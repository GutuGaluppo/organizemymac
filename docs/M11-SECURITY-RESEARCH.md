# M11 — Security research track

The plan treats malware protection as a separate track that must not block the rest of the
product, and says the app must not claim users are "protected" until it has a trustworthy engine.

## Implemented: Security Audit (experimental)

`src-tauri/src/security`, screen **Auditoria de segurança**. Read-only; it changes nothing.

- **Persistence audit.** Every LaunchAgent and LaunchDaemon in `~/Library/LaunchAgents`,
  `/Library/LaunchAgents` and `/Library/LaunchDaemons`: label, program and arguments, run at
  load, keep alive, whether the program still exists, and its code signature.
- **Signed binary audit.** Every app in `/Applications` and `~/Applications`: signature kind
  (Apple, App Store, Developer ID, development, ad hoc, unsigned, invalid) and Team ID via
  `codesign`; Gatekeeper assessment (`spctl`) only for apps without an App Store, Apple or
  Developer ID signature, because the online notarization check takes seconds per app; the
  quarantine attribute.
- **Neutral observations**, never verdicts: unsigned, ad hoc or invalid signature, a `com.apple.`
  label not signed by Apple, programs in temporary, shared or hidden folders, inline scripts
  (`bash -c`), orphaned items whose program is gone.
- Actions are limited to Reveal in Finder and opening System Settings → Login Items (no stable
  public API exists to disable other apps' background items; the private BTM database is never
  touched).

Measured on the development Mac: 31 startup items in 1.8 s, 64 apps in 16.7 s. `codesign --verify`
runs without `--strict`, which rejects Finder metadata that updaters leave in Chrome and Zoom
even though Gatekeeper accepts them.

## Not implemented (research)

| Item | What it needs | Notes |
| --- | --- | --- |
| YARA rules | a rule set and its update infrastructure, false-positive management | `yara-x` (Rust) is a candidate engine; rules must come from a curated, signed feed |
| Malware signature service | threat intelligence, a server, signed updates | a product and operations commitment, not a feature |
| Endpoint Security prototype | the `com.apple.developer.endpoint-security.client` entitlement (granted by Apple on request), a System Extension, notarization | only real-time protection option on macOS; cannot be built or tested without the entitlement |
| Quarantine | a reviewed flow to move suspicious files to a protected area | depends on detection quality |

Until these exist, the audit stays labelled experimental and the UI avoids fear-based language and
"issues found" counters, as the plan's product principles require.
