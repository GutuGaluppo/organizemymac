# Architecture

```text
React / TypeScript UI  ──Tauri IPC (typed commands + Channels)──▶  Rust core
  src/features/*                                                 src-tauri/src/*
  src/stores (Zustand)                                           scanner · safety · storage tree
  src/lib/ipc.ts (the only bridge)                               cleanup · duplicates · applications
                                                                 processes · SQLite
```

## Rules

- **The UI never runs shell commands or touches the filesystem.** Every operation is a typed Rust
  command in `src-tauri/src/commands`. The Tauri capability grants only `core:default` and the
  folder picker (`dialog:allow-open`); there is no shell or opener plugin.
- **Destructive operations always pass the safety layer** (`filesystem/safety.rs`) right before they
  run, even for paths the app produced itself. See [SAFETY.md](SAFETY.md).
- **Long operations are jobs.** `commands::start_job` gives each job an id and a cancel flag, runs it
  on a background thread with the *utility* QoS class and streams `JobEvent`s (`start`, `progress`,
  `warning`, `complete`, `cancelled`, `failed`) over a Tauri `Channel`. The UI keeps job state in a
  Zustand store, so switching sections does not lose a running scan.

## Scanner (`filesystem/scanner.rs`)

1. validate the root (absolute, exists, is a folder)
2. skip protected or prompting locations (`/System/Volumes`, `/Volumes`, `/dev`, cloud storage, and
   other apps' containers without Full Disk Access), reporting each as a warning
3. list folders in parallel on a small rayon pool (2–6 threads, utility QoS); each listing is one
   batch of names plus `lstat` metadata
4. batches go through a bounded channel to the calling thread, parents before children
5. the caller hands each entry to a `ScanVisitor`; modules combine visitors with `Fanout`
6. progress every 120 ms; a notice when macOS holds a read (usually a permission prompt)
7. the summary is stored in SQLite; the aggregated tree stays in memory for the last 3 scans

Symlinks are never followed. Hard links are counted once (by device and inode). Sizes are reported
as logical (`st_size`) and allocated (`st_blocks × 512`); cloud placeholders (`SF_DATALESS`) take no
local space. APFS clones and snapshots are not deduplicated, which the UI states.

### Benchmark

`cargo run --release --example bench_scan -- <folder>` (read-only). On an M4 home folder:
2.4 M entries in 13.8 s (≈175 k entries/s), 118 MB peak memory, cancellation in 9 ms.

## Storage tree (`storage/tree.rs`)

One node per folder plus files ≥ 1 MB; smaller files only add to their folder. Nodes are created
parent-first, so a single reverse pass computes totals. The Space Map queries it by path with
`storage_node`, and items moved to the Trash are removed from cached trees.

## Data (`db/`)

SQLite in `~/Library/Application Support/dev.galuppo.OrganizaMyMac`: scan history, operation log,
ignore list, settings and local-only metrics. Logs rotate daily in `~/Library/Logs/dev.galuppo.OrganizaMyMac`.
No telemetry.

## Development preview

`npm run dev` outside Tauri installs `src/dev/mock.ts`: commands answer with fictitious data, so
screens can be checked in a browser. It is never part of a production build.

## Mac Health and menu bar (`processes/`, `menubar.rs`)

`HealthMonitor` keeps one `sysinfo::System` and refreshes only what a caller needs: CPU and memory
for the menu bar, processes for the Performance screen. Battery comes from `pmset -g batt`, cached
for 20 s. Processes are grouped by their outermost `.app` bundle so helpers count toward their app;
"Quit" uses `NSRunningApplication.terminate` on the app's regular process (same as the app's Quit
menu item) and is refused for system processes, other users' processes and OrganizaMyMac itself.

The menu bar thread runs with the *background* QoS class: metrics every 5 s, processes every 30 s,
and it stops as soon as the item is turned off. With the item on, closing the window hides it; the
Dock icon or the menu bring it back.

## Swift helper (`native/macos-helper`)

A small command-line tool for what Apple frameworks do better than Rust bindings: Vision feature
prints and PhotoKit. `build.rs` compiles it with `swiftc` into `src-tauri/binaries/organiza-helper-<target>`
and Tauri ships it as a sidecar next to the app's executable. The Rust core starts it with fixed
arguments and JSON on stdin; progress comes back as JSON lines on stderr and the result on stdout.
The job's cancel flag kills the process. The helper never deletes files; Photos assets are only
deleted through `PHAssetChangeRequest`, which shows macOS's own confirmation.

Similar images: thumbnails (ImageIO, orientation applied) → 64-bit difference hash → candidate
pairs only between images whose aspect ratios differ by less than 8% and whose hashes differ by at
most 12/18 bits → Vision feature prints only for images in a pair → union-find on pairs under the
distance threshold (0.2 strict, 0.35 normal).
