# Safety

OrganizaMyMac is conservative when removing data. Every cleanup follows:

```text
scanner → classifier → cleanup plan → safety validation → user review → Trash (or delete)
```

## Rules enforced in code (`src-tauri/src/filesystem/safety.rs`)

`SafetyPolicy::check` runs right before every destructive operation. It rejects:

| Rule | Examples |
| --- | --- |
| Relative paths or `..` components | `docs/a.txt`, `/Users/me/../x` |
| Items that no longer exist | removed since the scan |
| Protected system locations | `/System`, `/bin`, `/sbin`, `/usr`, `/private`, `/cores`, `/dev` |
| `/Library` | unless the request comes from an explicitly supported, reviewed rule |
| Essential folders themselves | `/`, `/Applications`, `/Users`, `/Volumes/<disk>`, the home folder, `~/Library`, `~/Documents`, `~/.Trash`… (their contents are fine) |
| Leaving the scanned folder | the item must still be inside the scan root after resolving symlinks in its parents; a path through a symlink that points outside is a *symlink escape* |
| Read-only volumes | checked with `statfs` |
| OrganizaMyMac's own files | its app bundle, database and logs |
| The ignore list | anything the user chose to ignore |

The resolved path keeps the last component as is: removing a symlink removes the link, never its
target.

## Behaviour

- Items go to the **Trash** (`NSFileManager trashItemAtURL`) and can be recovered. Permanent
  deletion is off by default and exists only where it is the point of the feature (emptying the
  Trash), always behind an explicit confirmation that shows the totals.
- Every removal shows the path and size first. Nothing is removed by a single ambiguous button.
- Every attempt, successful or not, is written to the local operation log (Settings).
- Duplicate groups always keep at least one copy. Shared app containers are marked *danger* and are
  never selected by default.
- Symlinks are never followed during scans.
- Removals run deepest path first, so a file inside a folder that is also being removed goes first.

## Applications

App files are classified before an uninstall (`applications/leftovers.rs`):

| Match | Confidence | Selected by default |
| --- | --- | --- |
| Name equals the bundle id (`com.vendor.App`, `.plist`, `.savedState`…) | safe | yes |
| Known app-owned path (`rules/apps/known-paths.json`) | safe | yes |
| Vendor folder (`Application Support/Vendor`) | review | no |
| Same name as the app, or a name containing it | review | no |
| Shared group container | danger | no |

A folder named after another bundle id is never matched by name, and one owned by another
installed app with a longer id is skipped. `uninstall_app` rebuilds the plan and rejects any path
that is not part of it, refuses apps that are running and Apple apps that ship with macOS, and only
touches ~/Library (never /Library).

Leftovers of removed apps must be named after a bundle id that no app on any Spotlight-indexed
volume has; Apple ids are ignored; if the vendor still has installed apps, the item is only
offered for review. `remove_orphans` re-checks each path before moving it.

## Tests

`safety.rs` has a test for each rule: allowed path, blocked path, `/Library`, symlink escape,
outside the scan root, root protection, own files, ignore list, relative and missing paths, and
read-only volumes. Integration tests in `src-tauri/tests` build synthetic fixtures (duplicates, hard
links, symlink loops, a 5 GB sparse file, zero-byte, unicode, hidden, 60-level-deep and
permission-denied folders).
