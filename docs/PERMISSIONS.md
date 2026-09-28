# Permissions

OrganizeMyMac never tries to bypass TCC, SIP, the sandbox or any macOS protection, and never reads
or edits `TCC.db`.

## Full Disk Access (optional)

Without it the app works, but it cannot see folders macOS protects (Mail, Messages, Safari, other
apps' containers). The user grants it in **System Settings → Privacy & Security → Full Disk Access**;
the app only opens that pane.

macOS has no public API to ask whether the permission was granted. The app infers it by listing
`~/Library/Safari`, `~/Library/Mail` or `~/Library/Messages` and says the status is inferred.

## Prompts during a scan

Reading some locations makes macOS stop the read and show a prompt (for example "would like to
access data from other apps" for `~/Library/Containers`). While the prompt is on screen the read
blocks. To avoid surprises:

- without Full Disk Access, other apps' containers (`~/Library/Containers`, `~/Library/Group
  Containers`) are skipped and reported;
- cloud storage (`~/Library/CloudStorage`) is always skipped, since listing it can start a provider
  or download files;
- if a read still blocks for more than 3 s, the progress card tells the user macOS is waiting,
  probably for a permission prompt.

Desktop, Documents and Downloads ask once per app; that prompt is expected.

## Photos (optional)

Only for the "Fotos" source of Similar Images. The app asks through PhotoKit (the system prompt uses
`NSPhotoLibraryUsageDescription`) and reads thumbnails with network access off, so originals in
iCloud are never downloaded. The Photos library package is never read directly. Deleting goes
through PhotoKit: macOS asks for confirmation and the items go to Recently Deleted.
