// Development-only preview mode: when the UI runs in a plain browser (`npm run dev`), commands are
// answered with fictitious data so screens can be designed and checked without the Rust core.
// Never bundled into the app (imported only when `import.meta.env.DEV` and outside Tauri).
import { mockIPC } from "@tauri-apps/api/mocks";
import type { FileCategory, FileEntry, ScanResult, StorageNode } from "../types";

const HOME = "/Users/demo";
const now = Date.now();
const DAY = 86_400_000;

type Handler = (args: Record<string, unknown>) => unknown;
const handlers: Record<string, Handler> = {};
export function registerMock(command: string, handler: Handler) {
  handlers[command] = handler;
}

/** Streams job events to a Channel like the Rust side does. */
export function streamJob<T>(channelArg: unknown, root: string, result: () => T, durationMs = 1800) {
  const id = (channelArg as { id: number }).id;
  let index = 0;
  const send = (message: unknown) =>
    (window as unknown as { __TAURI_INTERNALS__: { runCallback: (id: number, d: unknown) => void } }).__TAURI_INTERNALS__.runCallback(id, {
      index: index++,
      message,
    });
  const jobId = crypto.randomUUID();
  setTimeout(() => send({ event: "start", jobId, root }), 10);
  const steps = 12;
  for (let i = 1; i <= steps; i++) {
    setTimeout(() => {
      const f = i / steps;
      send({
        event: "progress",
        files: Math.round(412_000 * f),
        directories: Math.round(61_000 * f),
        bytesScanned: Math.round(318e9 * f),
        current: `${root}/Library/Developer/CoreSimulator/Devices/${i}`,
        elapsedMs: Math.round(durationMs * f),
        warnings: i > 6 ? 2 : 0,
        waiting: false,
      });
      if (i === 7) {
        send({ event: "warning", path: `${HOME}/Library/Containers`, message: "not scanned: needs Full Disk Access" });
        send({ event: "warning", path: `${HOME}/Library/CloudStorage`, message: "not scanned: cloud storage" });
      }
    }, (durationMs / steps) * i);
  }
  setTimeout(() => {
    send({ event: "complete", result: result() });
    send({ end: true, index });
  }, durationMs + 50);
  return jobId;
}

export function fakeFile(path: string, size: number, ageDays: number, category: FileCategory = "other"): FileEntry {
  const name = path.split("/").pop() ?? path;
  const ext = name.includes(".") ? name.split(".").pop()!.toLowerCase() : null;
  return {
    path,
    name,
    sizeLogical: size,
    sizeAllocated: size,
    createdAt: now - (ageDays + 3) * DAY,
    modifiedAt: now - ageDays * DAY,
    extension: ext,
    category,
    isDirectory: false,
    isSymlink: false,
    isCloudPlaceholder: false,
  };
}

let nodeId = 1;
function node(path: string, size: number, children?: StorageNode[], isDir = true): StorageNode {
  return {
    id: nodeId++,
    name: path.split("/").pop() || path,
    path,
    size,
    allocated: Math.round(size * 0.96),
    files: Math.round(size / 900_000),
    isDir,
    looseFilesSize: children ? Math.max(0, size - children.reduce((s, c) => s + c.size, 0)) : 0,
    hasChildren: isDir,
    children: children ?? null,
  };
}

export const demoTree = (): StorageNode =>
  node(HOME, 318e9, [
    node(`${HOME}/Library`, 124e9),
    node(`${HOME}/Movies`, 61e9),
    node(`${HOME}/Projects`, 48e9),
    node(`${HOME}/Pictures`, 33e9),
    node(`${HOME}/Downloads`, 21.4e9),
    node(`${HOME}/Documents`, 14.2e9),
    node(`${HOME}/Music`, 7.9e9),
    node(`${HOME}/Desktop`, 3.1e9),
    node(`${HOME}/.docker`, 2.6e9),
    node(`${HOME}/.npm`, 1.4e9),
    node(`${HOME}/Parallels.pvm`, 900e6, undefined, false),
  ]);

export const demoLargest = (): FileEntry[] => [
  fakeFile(`${HOME}/Movies/Viagem Patagônia 4K.mov`, 18.4e9, 420, "video"),
  fakeFile(`${HOME}/Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw`, 14.1e9, 2, "diskImage"),
  fakeFile(`${HOME}/Downloads/Xcode_26.6.xip`, 11.2e9, 95, "archive"),
  fakeFile(`${HOME}/Movies/Casamento — edição final.mp4`, 7.8e9, 610, "video"),
  fakeFile(`${HOME}/Documents/Backup iPhone 2024.zip`, 6.3e9, 520, "archive"),
  fakeFile(`${HOME}/Downloads/ubuntu-24.04-desktop-arm64.iso`, 3.4e9, 300, "diskImage"),
  fakeFile(`${HOME}/Projects/ml-sandbox/models/llama-8b-q4.gguf`, 4.9e9, 140, "other"),
  fakeFile(`${HOME}/Downloads/Figma-126.1.dmg`, 540e6, 60, "diskImage"),
  fakeFile(`${HOME}/Pictures/Export Lightroom/panorama-final.tif`, 1.9e9, 800, "image"),
  fakeFile(`${HOME}/Music/Logic/Sessão 12/Audio Files/take-03.wav`, 1.2e9, 900, "audio"),
];

registerMock("start_scan", ({ root, onEvent }) =>
  streamJob<ScanResult>(onEvent, root as string, () => ({
    id: "demo-scan",
    root: root as string,
    startedAt: now - 1800,
    finishedAt: now,
    files: 412_318,
    directories: 61_204,
    bytesScanned: 318e9,
    bytesAllocated: 301e9,
    symlinks: 1_830,
    cloudPlaceholders: 214,
    warnings: 2,
    durationMs: 1790,
    cancelled: false,
    reclaimableBytes: 0,
    largestFiles: demoLargest(),
    tree: demoTree(),
  })),
);
registerMock("cancel_job", () => true);
registerMock("permission_status", () => ({ fullDiskAccess: "denied", home: HOME }));
registerMock("open_system_settings", () => null);
registerMock("suggested_locations", () =>
  ["home", "downloads", "documents", "desktop", "applications", "disk"].map((id) => ({
    id,
    exists: true,
    path: { home: HOME, downloads: `${HOME}/Downloads`, documents: `${HOME}/Documents`, desktop: `${HOME}/Desktop`, applications: "/Applications", disk: "/" }[id],
  })),
);
registerMock("app_info", () => ({ version: "0.1.0", dataDir: `${HOME}/Library/Application Support/dev.galuppo.OrganizaMyMac`, logDir: `${HOME}/Library/Logs/dev.galuppo.OrganizaMyMac` }));
registerMock("recent_scans", () => []);
registerMock("operation_log", () => [
  { id: 2, ts: now - 3_600_000, action: "trash", path: `${HOME}/Downloads/Figma-125.dmg`, size: 530e6, ok: true },
  { id: 1, ts: now - 7_200_000, action: "trash", path: `${HOME}/Downloads/old-installer.pkg`, size: 210e6, ok: true },
]);
registerMock("local_metrics", () => ({ scans_total: 14, files_scanned_total: 5_120_004, bytes_reclaimed_total: 23.4e9, items_removed_total: 187, operation_errors_total: 1 }));
registerMock("ignore_list", () => [{ path: `${HOME}/Projects/keep-forever`, addedAt: now - 30 * DAY, reason: null }]);
registerMock("add_to_ignore_list", () => null);
registerMock("remove_from_ignore_list", () => null);
registerMock("plugin:dialog|open", () => `${HOME}/Projects`);

export function install() {
  mockIPC((cmd, args) => {
    const handler = handlers[cmd];
    if (!handler) {
      console.warn("[mock] no handler for", cmd, args);
      return null;
    }
    return handler((args ?? {}) as Record<string, unknown>);
  });
  document.documentElement.dataset.mock = "true";
}
