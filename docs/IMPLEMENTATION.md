# OrganizeMyMac — IMPLEMENTATION.md

**Date:** 2026-09-28  
**Product name:** OrganizeMyMac  
**Goal:** build a native macOS utility inspired by the category of tools offered by CleanMyMac, with an initial focus on storage analysis, safe cleanup, application management, and system monitoring.

> Core principle: OrganizeMyMac must be **conservative when removing data**. Whenever possible, files should be moved to Trash instead of being permanently deleted. The app must clearly explain what will be removed and allow the user to review all destructive actions before execution.

---

# 1. Product Scope

OrganizeMyMac will be divided into independent modules:

1. **Dashboard / Mac Health**
2. **Storage Scanner**
3. **Large & Old Files**
4. **Downloads Cleanup**
5. **Duplicate Finder**
6. **Trash Cleanup**
7. **Space Map**
8. **Application Manager**
9. **App Leftovers**
10. **System / User Junk**
11. **Performance Monitor**
12. **Similar Images**
13. **Smart Care**
14. **Privacy Cleanup**
15. **Maintenance Tasks**
16. **Cloud Cleanup**
17. **App Updater**
18. **Security / Malware**

The first release should not attempt to reproduce every CleanMyMac feature.

The highest-value first version is a fast, visual, and trustworthy scanner that helps users understand where storage is being used and what they can safely remove.

---

# 2. Distribution Strategy

## Recommendation

Initially distribute OrganizeMyMac as:

- `.dmg`
- signed with **Developer ID Application**
- **Hardened Runtime** enabled
- Apple notarization
- automatic updates added later

Avoid using the Mac App Store as the primary distribution channel for the MVP.

Why:

Utilities that perform system maintenance and broad filesystem inspection can be significantly limited by the App Sandbox.

Even outside the Mac App Store, macOS privacy and security protections still apply.

---

## Full Disk Access

OrganizeMyMac should remain partially functional without Full Disk Access.

Features that require broader filesystem visibility should clearly explain why the permission is needed.

Users must grant this permission manually under:

`System Settings → Privacy & Security → Full Disk Access`

OrganizeMyMac must never attempt to bypass:

- TCC
- SIP
- sandbox restrictions
- protected system directories
- macOS security mechanisms

---

# 3. Recommended Stack

## UI

- Tauri 2
- React
- TypeScript
- Vite
- Tailwind CSS or CSS Modules
- Zustand for UI state
- TanStack Query for long-running asynchronous operations
- Lucide icons during early development

---

## Core

- Rust
- Tokio
- Rayon
- Serde
- WalkDir or JWalk
- BLAKE3
- plist
- SQLite using `rusqlite` or `sqlx`

---

## Native macOS Bridge

Use a small Swift helper when Apple frameworks provide a cleaner or more reliable implementation than Rust bindings.

Likely frameworks:

- AppKit
- Foundation
- Photos
- Vision
- ServiceManagement
- IOKit or modern equivalent APIs for energy/system metrics
- DiskArbitration where appropriate

The Swift helper can be packaged as a Tauri sidecar for non-privileged native operations.

Truly privileged actions should be isolated into a separate helper and always require explicit user authorization.

---

# 4. Architecture

```text
┌──────────────────────────────┐
│ React / TypeScript UI        │
│                              │
│ Dashboard                    │
│ Storage                      │
│ Duplicates                   │
│ Applications                 │
│ Performance                  │
└──────────────┬───────────────┘
               │ Tauri IPC
┌──────────────▼───────────────┐
│ Rust Core                    │
│                              │
│ filesystem scanner           │
│ hashing                      │
│ cleanup planner              │
│ app discovery                │
│ process metrics              │
│ SQLite                       │
└───────┬─────────────┬────────┘
        │             │
        │             │
┌───────▼──────┐ ┌────▼────────────┐
│ Swift helper │ │ Privileged       │
│              │ │ helper (future) │
│ Photos       │ │                  │
│ Vision       │ │ maintenance      │
│ AppKit       │ │ launchd tasks    │
│ native APIs  │ │ system actions   │
└──────────────┘ └─────────────────┘
```

Important rule:

**The UI must never execute shell commands directly.**

All filesystem, process, cleanup, and system operations must pass through a typed and validated Rust API.

---

# 5. Basic Data Model

```ts
type FileEntry = {
  path: string
  name: string
  sizeLogical: number
  sizeAllocated?: number
  createdAt?: string
  modifiedAt?: string
  extension?: string
  category?: FileCategory
  isDirectory: boolean
  isSymlink: boolean
}

type ScanResult = {
  id: string
  root: string
  startedAt: string
  finishedAt?: string
  files: number
  directories: number
  bytesScanned: number
  reclaimableBytes: number
}

type CleanupCandidate = {
  path: string
  size: number
  reason: string
  confidence: "safe" | "review" | "danger"
  selected: boolean
}
```

---

# 6. Safety Layer

This is one of the most important parts of OrganizeMyMac.

A cleanup module must never simply return paths and delete them immediately.

Required flow:

```text
scanner
   ↓
classifier
   ↓
cleanup plan
   ↓
safety validation
   ↓
user review
   ↓
trash / delete
```

---

## Minimum Safety Rules

Automatically block destructive operations targeting:

- `/System`
- `/bin`
- `/sbin`
- `/usr`
- `/private`
- `/Library` unless the rule is explicitly supported and reviewed
- read-only volumes
- symlinks escaping the selected scan root
- files belonging to OrganizeMyMac while it is running

Additional rules:

- follow symlinks: **no**
- permanent deletion: **disabled by default**
- always show file path and size before removal
- keep a local operation log
- support an ignore list
- scans must be cancellable
- scans should be resumable where practical

---

# 7. Scanner Engine

The scanner will be the foundation of most OrganizeMyMac features.

## Pipeline

1. receive scan root
2. validate permissions
3. enumerate filesystem
4. skip protected paths
5. collect metadata
6. aggregate directory sizes
7. stream progress to the UI
8. persist scan summary
9. classify cleanup candidates

---

## Suggested Events

```text
scan:start
scan:progress
scan:item
scan:warning
scan:complete
scan:cancelled
```

---

## APFS Considerations

Logical file size does not always equal physical disk usage.

OrganizeMyMac should account for:

- hard links
- sparse files
- APFS clones
- snapshots
- cloud placeholders

The MVP may initially display logical size, but the interface should make that distinction clear.

---

# 8. Large & Old Files

## Implementation

Allow users to filter by:

- folder or volume
- minimum size
- minimum age
- file extension
- file type
- modified date

Suggested presets:

```text
> 1 GB
> 500 MB
> 100 MB
```

For "old" files, initially use:

- `modifiedAt`
- optionally `createdAt`

Avoid relying on `lastAccessTime` as the primary age signal.

---

## Result Table

```text
Name | Size | Modified | Type | Location
```

Actions:

- Quick Look
- Reveal in Finder
- Move to Trash
- Ignore

---

# 9. Downloads Cleanup

Create a specialized scanner for:

```text
~/Downloads
```

Categories:

- DMG files
- ZIP archives
- installers
- screenshots
- old files
- large files

Recommended ranking:

```text
old installers
large files
archives
recent downloads
```

This is an excellent early feature because it is simple, useful, and relatively safe.

---

# 10. Duplicate Finder

## Three-Stage Strategy

Do not calculate full hashes for every file immediately.

### Step 1 — Size

Group files by size.

Files with unique sizes are discarded from duplicate analysis.

### Step 2 — Sample Hash

Calculate hashes for small blocks:

```text
first 64 KB
middle 64 KB
last 64 KB
```

### Step 3 — Full Hash

Only files that still remain candidates receive a full BLAKE3 hash.

Pipeline:

```text
files
 ↓
size groups
 ↓
sample hash
 ↓
full hash
 ↓
duplicate groups
```

---

## Safety Rules

Never suggest deleting every file in a duplicate group.

Always preserve at least one copy.

Detect:

- inode
- device id
- hard links

to avoid presenting hard links as independent duplicates.

---

# 11. Trash Cleanup

Phase 1:

```text
~/.Trash
```

Later phases may support:

- Trash folders on external volumes
- Mail Trash
- other special locations

Before cleanup, show:

```text
X files
Y folders
Z GB
```

Always require explicit confirmation.

---

# 12. Space Map

Build a simpler first version than CleanMyMac's Space Lens.

The backend should produce a storage tree:

```ts
type StorageNode = {
  name: string
  path: string
  size: number
  children?: StorageNode[]
}
```

Possible UI representations:

- treemap
- circles
- sunburst
- hierarchical list

Recommended MVP:

**Treemap + hierarchical sidebar.**

This is easier to navigate and simpler to implement than a physics-based bubble visualization.

---

# 13. Application Scanner

Initially enumerate:

```text
/Applications
~/Applications
```

Read metadata from `Info.plist`:

```text
CFBundleIdentifier
CFBundleDisplayName
CFBundleShortVersionString
CFBundleVersion
LSMinimumSystemVersion
```

Store:

```text
bundle id
path
version
size
last modified
```

---

# 14. App Uninstaller

Moving only the `.app` bundle to Trash is often incomplete.

After identifying the app bundle ID, search common locations such as:

```text
~/Library/Application Support
~/Library/Caches
~/Library/Preferences
~/Library/Logs
~/Library/Saved Application State
~/Library/WebKit
~/Library/HTTPStorages
~/Library/Containers
~/Library/Group Containers
```

---

## Critical Rule

Never remove a directory simply because its name loosely resembles the app name.

Use confidence levels:

```text
Exact bundle-id match    → safe
Known app-owned path     → safe
Vendor-name match        → review
Fuzzy name match         → manual review
Shared group container   → danger
```

Before removal, present a breakdown:

```text
Application                420 MB
Caches                      94 MB
Application Support        250 MB
Preferences                 80 KB
Logs                        18 MB
--------------------------------
Total                       782 MB
```

---

# 15. App Leftovers

Algorithm:

1. index installed applications
2. extract all bundle IDs
3. index known locations under `~/Library`
4. identify entries whose bundle IDs no longer match installed apps
5. classify each result by confidence

Do not remove ambiguous items automatically.

---

# 16. User / System Junk

Start only with low-risk locations.

## MVP

```text
~/Library/Caches
~/Library/Logs
Downloads/*.dmg
Downloads/*.pkg
Downloads/*.zip
```

Use a rule database:

```yaml
id: chrome-cache
paths:
  - ~/Library/Caches/Google/Chrome
risk: low
requires_app_closed: true
```

Each cleanup rule should define:

- location
- risk level
- prerequisites
- estimated size
- associated app
- rollback capability

Avoid in the MVP:

- arbitrary modifications under `/Library`
- unknown system caches
- deletion under `/System`
- modifications to SIP-protected components

---

# 17. Performance Monitor

## Initial Dashboard

Show:

- CPU
- memory
- swap
- free disk space
- uptime
- battery
- top apps by memory usage

Suggested refresh interval:

```text
1–2 seconds
```

Reduce refresh frequency when the OrganizeMyMac window is hidden.

---

## Processes

Separate process categories:

```text
applications
background processes
system processes
```

Allow `Quit` only for user applications.

Do not present `kill -9` as a normal user-facing action.

---

# 18. Menu Bar App

Create a compact menu bar view showing:

```text
CPU
RAM
Disk
Battery
Top consumers
```

The menu bar experience can become a strong product differentiator before more advanced cleanup features exist.

---

# 19. Similar Images

Use Apple's Vision framework.

Pipeline:

```text
image
 ↓
thumbnail
 ↓
Vision feature print
 ↓
distance comparison
 ↓
cluster
```

Avoid naive `N²` comparisons.

Pre-group candidates by:

- dimensions
- aspect ratio
- approximate date
- perceptual hash

Then run Vision feature-vector comparisons only on likely candidates.

---

## Photos Library

For content stored in Photos:

- request authorization
- use the Photos framework
- never manipulate the internal Photos Library directory structure directly

---

# 20. Smart Care

Smart Care should not be a separate scanner.

It should orchestrate existing scanners.

Example:

```text
Storage
  8.2 GB reclaimable

Downloads
  1.4 GB

Duplicates
  3.1 GB

Trash
  2.5 GB

Apps
  4 unused
```

The user reviews everything before execution.

---

## Recommendation Engine

Initially use deterministic rules.

Example:

```text
if trash_size > 2 GB:
    suggest("Empty Trash")

if downloads_dmg_count > 10:
    suggest("Review installers")
```

Avoid ML or AI during the first version.

---

# 21. Browser Privacy Cleanup

Implement in a later phase.

Start with:

- Chrome
- Chromium
- Firefox

Possible cleanup targets:

- cache
- history
- cookies
- download history

Potential issues:

- SQLite databases may be open
- browser schemas may change
- synced data may reappear
- Safari has additional protections

Rule:

**The browser must be closed before cleanup begins.**

---

# 22. Maintenance Tasks

Implement as a separate module from the scanner.

Possible future tasks:

- Flush DNS cache
- Reindex Spotlight
- inspect or manage Time Machine snapshots
- purgeable-space helpers

These operations may require:

- elevated privileges
- a privileged helper
- user authorization
- APIs or command-line tools that change between macOS releases

Do not include them in the first MVP.

---

# 23. Login Items / Background Items

Initially provide auditing only.

Display:

- running applications
- detected LaunchAgents
- detected LaunchDaemons
- known startup items

Do not modify undocumented macOS internal databases.

When no stable public API exists for a specific change, direct the user to the appropriate System Settings page instead.

---

# 24. Application Permissions

Do not manipulate the TCC database directly.

Initial implementation:

- explain permissions
- provide shortcuts to System Settings
- display permission status only when a supported API exists

Avoid relying on hacks involving:

```text
TCC.db
```

This would create a fragile dependency on undocumented macOS internals.

---

# 25. Cloud Cleanup

Do not include this in the MVP.

Possible future providers:

```text
iCloud Drive
Google Drive
Dropbox
OneDrive
```

Suggested abstraction:

```text
CloudProvider
 ├─ list()
 ├─ metadata()
 ├─ delete()
 ├─ download()
 └─ makeLocalOnly()
```

A first version could inspect locally synchronized files, but cloud-only placeholders must be treated correctly.

---

# 26. App Updater

Do not promise a universal updater in the first version.

Possible evolution:

### v1

Detect installed app versions.

### v2

Detect Sparkle-enabled apps.

### v3

Add trusted external version sources.

### v4

Add provider-specific integrations.

Updating third-party applications safely requires careful handling of:

- code signatures
- trusted download sources
- checksums
- CPU architecture
- permissions
- running processes

---

# 27. Malware Scanner

Treat malware protection as a separate product track inside OrganizeMyMac.

A serious scanner requires:

- threat intelligence
- signatures
- YARA rules or heuristics
- update infrastructure
- quarantine
- false-positive management
- code-signing analysis
- persistence analysis
- event monitoring
- ongoing security research

For real-time protection, macOS provides Endpoint Security, which requires specific entitlements and a System Extension.

Do not claim that users are "protected" until OrganizeMyMac has a trustworthy security engine.

---

## Possible Early Security Feature

An experimental **Security Audit** could detect:

- suspicious LaunchAgents
- unsigned binaries
- applications from unknown origins
- unusual persistence items

This should be presented as a security audit, not as antivirus protection.

---

# 28. Suggested Repository Structure

```text
organizemymac/
├── src/
│   ├── app/
│   ├── components/
│   ├── features/
│   │   ├── dashboard/
│   │   ├── storage/
│   │   ├── duplicates/
│   │   ├── applications/
│   │   ├── performance/
│   │   └── settings/
│   ├── hooks/
│   ├── stores/
│   └── types/
│
├── src-tauri/
│   ├── src/
│   │   ├── commands/
│   │   ├── filesystem/
│   │   │   ├── scanner.rs
│   │   │   ├── metadata.rs
│   │   │   └── safety.rs
│   │   ├── cleanup/
│   │   ├── duplicates/
│   │   ├── applications/
│   │   ├── processes/
│   │   ├── storage/
│   │   ├── db/
│   │   └── lib.rs
│   └── Cargo.toml
│
├── native/
│   └── macos-helper/
│       ├── Photos/
│       ├── Vision/
│       └── System/
│
├── rules/
│   ├── cleanup/
│   ├── apps/
│   └── exclusions/
│
├── tests/
│   ├── fixtures/
│   └── integration/
│
└── docs/
    ├── ARCHITECTURE.md
    ├── SAFETY.md
    └── PERMISSIONS.md
```

---

# 29. Milestones

## M0 — Foundation & Safety

### Deliverables

- Tauri + React project
- Rust command layer
- progress streaming
- cancellable jobs
- SQLite
- permissions onboarding
- safety layer
- logging
- test fixtures

### Definition of Done

A user-selected folder can be scanned without freezing the UI and without modifying any files.

---

## M1 — Storage MVP

### Features

- Disk Overview
- Large Files
- Old Files
- Downloads
- Reveal in Finder
- Quick Look
- Move to Trash

### Definition of Done

Users can identify and manually remove large or unnecessary files.

**At this stage, OrganizeMyMac is already a usable product.**

---

## M2 — Duplicate Finder

### Features

- size grouping
- sample hashing
- BLAKE3 full hashing
- duplicate groups
- smart default selection
- ignore list

### Definition of Done

The duplicate scanner avoids unnecessary full hashing and never selects every file in a duplicate group for deletion.

---

## M3 — Space Map

### Features

- aggregated storage tree
- treemap
- navigation
- breadcrumbs
- file selection

### Definition of Done

Users can visually identify where disk space is being consumed.

---

## M4 — Application Manager

### Features

- installed applications
- app size
- bundle metadata
- uninstall
- leftovers
- confidence classification

### Definition of Done

App removal provides a complete preview of associated files and protects shared containers from unsafe deletion.

---

## MVP 1.0

**M0–M4 form the recommended first public release of OrganizeMyMac.**

---

## M5 — Mac Health + Menu Bar

### Features

- CPU
- RAM
- disk
- battery
- uptime
- running apps
- top memory consumers

### Definition of Done

The menu bar component can remain active throughout the day with minimal CPU and energy impact.

---

## M6 — Similar Images

### Features

- image indexing
- perceptual pre-filter
- Vision feature prints
- similarity clustering
- Photos integration

---

## M7 — Smart Care

### Features

- central scan orchestration
- deterministic recommendations
- unified review screen
- safe cleanup plan

---

## M8 — Advanced Cleanup

### Features

- user caches
- logs
- browser cleanup
- mail attachments
- customizable cleanup rules

---

## M9 — Native Maintenance Helper

### Features

- privileged task service
- DNS cache operations
- Spotlight reindexing
- Time Machine snapshot utilities

Only proceed after a dedicated security and permissions review.

---

## M10 — Cloud + Updates

### Features

- Google Drive
- Dropbox
- OneDrive
- iCloud
- application update detection
- Sparkle support

---

## M11 — Security Research Track

### Features

- signed binary audit
- persistence audit
- YARA research
- malware signature service
- Endpoint Security prototype

This milestone must not block the rest of the OrganizeMyMac roadmap.

---

# 30. Prioritization

## Build First

```text
Storage scanner
Large files
Downloads
Duplicates
Space map
Applications
Leftovers
Mac health
```

---

## Build Later

```text
Similar photos
Smart Care
User junk
Privacy
Maintenance
Cloud
Updater
```

---

## Separate Research Track

```text
Malware
Real-time protection
Application permission revocation
Deep system cleanup
```

---

# 31. Testing Strategy

## Filesystem Fixtures

Create synthetic test directories containing:

```text
duplicates
hard links
symlinks
huge files
zero-byte files
sparse files
permission-denied folders
deep folder trees
unicode filenames
hidden files
```

---

## Safety Tests

Every cleanup rule should include tests for:

```text
allowed path
blocked path
symlink escape
root protection
shared container
read-only volume
```

---

## Performance Benchmarks

Minimum datasets:

```text
100k files
500k files
1M files
```

Measure:

- files per second
- memory peak
- cancellation latency
- hashing throughput

---

# 32. Internal Metrics

Avoid mandatory telemetry.

Store local-only metrics such as:

```text
scan duration
files scanned
bytes scanned
bytes reclaimed
module errors
last scan
```

These values can power the OrganizeMyMac dashboard without sending user data to external servers.

---

# 33. MVP Success Criteria

OrganizeMyMac does not need to be a complete CleanMyMac replacement in the first release.

It needs to answer three questions extremely well:

### 1.

**What is taking up space on my Mac?**

### 2.

**What can I safely remove?**

### 3.

**Which applications and leftover files are wasting storage?**

If those three workflows are fast, clear, attractive, and trustworthy, OrganizeMyMac already has a strong product foundation.

---

# 34. Recommended First Technical Task

Start with the filesystem scanner.

First implementation spike:

```text
scanDirectory(path)
```

Expected output:

```json
{
  "files": 128429,
  "directories": 18439,
  "totalBytes": 29482849282,
  "largestFiles": []
}
```

Then add:

- progress events
- cancellation
- exclusions
- safety checks
- directory aggregation

Do not begin with the visual dashboard.

The dashboard should be built on top of real data produced by the scanner.

---

# 35. Product Development Principle

OrganizeMyMac should differentiate itself through three characteristics:

## Transparency

Always explain what was detected and why an item is considered removable.

## Safety

Never optimize reclaimable space at the expense of user trust.

## Performance

Scanning millions of filesystem entries must not make the Mac feel slower.

The ideal experience is:

```text
Open OrganizeMyMac
      ↓
Scan
      ↓
Understand
      ↓
Review
      ↓
Clean
```

No fear-based warnings.

No exaggerated "issues found" counters.

No destructive actions hidden behind a single ambiguous button.

---

# 36. Official Technical References

- CleanMyMac — My Tools  
  https://macpaw.com/support/cleanmymac/knowledgebase/my-tools

- CleanMyMac — Smart Care  
  https://macpaw.com/support/cleanmymac/knowledgebase/smart-care

- CleanMyMac — Feature Availability  
  https://macpaw.com/support/cleanmymac/knowledgebase/missing-features

- CleanMyMac — Space Lens  
  https://macpaw.com/support/cleanmymac/knowledgebase/space-lens

- CleanMyMac — Maintenance Tasks  
  https://macpaw.com/support/cleanmymac/knowledgebase/maintenance-tasks

- CleanMyMac — Similar Images  
  https://macpaw.com/support/cleanmymac/knowledgebase/similar-images

- CleanMyMac — Cloud Cleanup  
  https://macpaw.com/support/cleanmymac/knowledgebase/cloud-cleanup

- Apple — App Sandbox File Access  
  https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox

- Apple — Endpoint Security  
  https://developer.apple.com/documentation/endpointsecurity

- Apple — ServiceManagement / SMAppService  
  https://developer.apple.com/documentation/servicemanagement/smappservice

- Apple — Vision Image Feature Prints  
  https://developer.apple.com/documentation/vision/vngenerateimagefeatureprintrequest

- Tauri — Sidecars  
  https://v2.tauri.app/develop/sidecar/

- Tauri — macOS Signing and Notarization  
  https://v2.tauri.app/distribute/sign/macos/
