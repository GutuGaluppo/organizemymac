// Development-only preview mode: when the UI runs in a plain browser (`npm run dev`), commands are
// answered with fictitious data so screens can be designed and checked without the Rust core.
// Never bundled into the app (imported only when `import.meta.env.DEV` and outside Tauri).
import { mockIPC } from "@tauri-apps/api/mocks";
import type { SmartCareReport, SimilarResult, AppInfo, AppsResult, Leftover, OrphansResult, UninstallPlan, DuplicateGroup, DuplicatesResult, DownloadItem, DownloadsResult, FileCategory, FileEntry, FindResult, ScanResult, StorageNode } from "../types";

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

const baseScan = (root: string): ScanResult => ({
  id: crypto.randomUUID(),
  root,
  startedAt: now - 1500,
  finishedAt: now,
  files: 412_318,
  directories: 61_204,
  bytesScanned: 318e9,
  bytesAllocated: 301e9,
  symlinks: 1_830,
  cloudPlaceholders: 214,
  warnings: 0,
  durationMs: 1490,
  cancelled: false,
  reclaimableBytes: 0,
  largestFiles: [],
  tree: null,
});

registerMock("disk_overview", () => [
  { name: "Macintosh HD", mountPoint: "/", fileSystem: "apfs", total: 994.66e9, free: 212.4e9, used: 782.2e9, isRoot: true, readOnly: false, local: true },
  { name: "Backup", mountPoint: "/Volumes/Backup", fileSystem: "apfs", total: 2e12, free: 1.31e12, used: 690e9, isRoot: false, readOnly: false, local: true },
]);

registerMock("start_find_files", ({ root, filter, onEvent }) =>
  streamJob<FindResult>(onEvent, root as string, () => {
    const f = filter as { minSize: number };
    const matches = demoLargest().filter((x) => x.sizeLogical >= f.minSize && !x.path.includes("/Library/"));
    return { ...baseScan(root as string), matches, matched: matches.length, matchedBytes: matches.reduce((a, b) => a + b.sizeLogical, 0), truncated: false };
  }, 1400),
);

const dl = (name: string, size: number, age: number, category: FileCategory, group: DownloadItem["group"], extra: Partial<DownloadItem> = {}): DownloadItem => ({
  ...fakeFile(`${HOME}/Downloads/${name}`, size, age, category),
  group,
  ageDays: age,
  extracted: false,
  confidence: group === "oldInstaller" ? "safe" : "review",
  selected: group === "oldInstaller",
  ...extra,
});

registerMock("start_downloads_scan", ({ onEvent }) =>
  streamJob<DownloadsResult>(onEvent, `${HOME}/Downloads`, () => {
    const items: DownloadItem[] = [
      dl("Figma-126.1.dmg", 540e6, 60, "diskImage", "oldInstaller"),
      dl("Docker.dmg", 610e6, 120, "diskImage", "oldInstaller"),
      dl("Zoom.pkg", 98e6, 210, "installer", "oldInstaller"),
      dl("GoogleChrome.dmg", 230e6, 45, "diskImage", "oldInstaller"),
      dl("Xcode_26.6.xip", 11.2e9, 95, "other", "largeFile"),
      dl("ubuntu-24.04-desktop-arm64.iso", 3.4e9, 300, "diskImage", "largeFile"),
      dl("fotos-viagem.zip", 1.2e9, 40, "archive", "largeFile"),
      dl("brand-assets.zip", 86e6, 22, "archive", "archive", { extracted: true, confidence: "safe" }),
      dl("relatorio-dados.tar.gz", 12e6, 16, "archive", "archive"),
      dl("Raycast.dmg", 72e6, 4, "diskImage", "installer"),
      dl("Captura de Tela 2026-09-12 às 10.41.22.png", 1.8e6, 16, "image", "screenshot"),
      dl("Captura de Tela 2026-09-20 às 18.02.03.png", 2.3e6, 8, "image", "screenshot"),
      dl("contrato-2025.pdf", 1.1e6, 290, "document", "oldFile"),
      dl("apresentacao.key", 44e6, 12, "document", "recent"),
    ];
    return { ...baseScan(`${HOME}/Downloads`), folder: `${HOME}/Downloads`, items, totalBytes: items.reduce((a, b) => a + b.sizeLogical, 0) };
  }, 1200),
);

registerMock("trash_summary", () => ({
  path: `${HOME}/.Trash`,
  readable: true,
  files: 1_204,
  folders: 88,
  bytes: 6.1e9,
  items: [
    { ...fakeFile(`${HOME}/.Trash/old-project`, 3.9e9, 30), isDirectory: true },
    fakeFile(`${HOME}/.Trash/Figma-125.dmg`, 530e6, 60, "diskImage"),
    fakeFile(`${HOME}/.Trash/gravação.mov`, 1.4e9, 10, "video"),
    fakeFile(`${HOME}/.Trash/notas.txt`, 12e3, 5, "document"),
  ],
}));
registerMock("empty_trash", () => [{ path: `${HOME}/.Trash/old-project`, size: 6.1e9, ok: true }]);
registerMock("empty_trash_with_finder", () => null);
registerMock("reveal_in_finder", () => null);
registerMock("quick_look", () => null);
registerMock("path_exists", () => true);
registerMock("move_to_trash", ({ request }) =>
  (request as { items: { path: string; size: number }[] }).items.map((i) => ({ ...i, ok: !i.path.includes("/Library/"), error: i.path.includes("/Library/") ? "blocked by safety rule" : null })),
);

const dupGroup = (name: string, size: number, paths: string[], category: FileCategory): DuplicateGroup => ({
  hash: crypto.randomUUID().replace(/-/g, ""),
  size,
  wasted: size * (paths.length - 1),
  files: paths.map((p, i) => ({ ...fakeFile(`${HOME}/${p}/${name}`, size, 200 - i * 30, category), hardLinks: [], selected: i > 0 })),
});

registerMock("start_duplicate_scan", ({ root, onEvent }) =>
  streamJob<DuplicatesResult>(onEvent, root as string, () => {
    const groups = [
      dupGroup("Viagem Patagônia 4K.mov", 18.4e9, ["Movies", "Desktop/Backup vídeos"], "video"),
      dupGroup("Backup iPhone 2024.zip", 6.3e9, ["Documents", "Downloads"], "archive"),
      dupGroup("ubuntu-24.04-desktop-arm64.iso", 3.4e9, ["Downloads", "Projects/vm-images", "Desktop"], "diskImage"),
      dupGroup("IMG_4021.HEIC", 3.1e6, ["Pictures/2024", "Desktop/fotos", "Downloads"], "image"),
      dupGroup("contrato-assinado.pdf", 1.2e6, ["Documents/Contratos", "Downloads"], "document"),
    ];
    return {
      ...baseScan(root as string),
      groups,
      totalGroups: groups.length,
      wastedBytes: groups.reduce((s, g) => s + g.wasted, 0),
      hashStats: { candidates: 8307, afterSize: 2890, sampleHashed: 2890, fullHashed: 1616, bytesHashed: 8.9e9, hashMs: 4940 },
    };
  }, 1600),
);
registerMock("move_duplicates_to_trash", ({ groups }) =>
  (groups as { remove: { path: string; size: number }[] }[]).flatMap((g) => g.remove.map((r) => ({ path: r.path, size: r.size, ok: true, error: null }))),
);

function nested(path: string, size: number, depth: number): StorageNode {
  const names = ["Library", "Developer", "Caches", "Application Support", "Movies", "Projetos", "node_modules", "Fotos", "Backups", "Xcode", "DerivedData", "Docker", "Music", "Arquivos"];
  let seed = [...path].reduce((a, c) => (a * 31 + c.charCodeAt(0)) % 9973, 7);
  const rnd = () => ((seed = (seed * 1103515245 + 12345) % 2147483648) / 2147483648);
  const count = depth > 0 ? 6 + Math.floor(rnd() * 8) : 0;
  let left = size * 0.93;
  const children: StorageNode[] = [];
  for (let i = 0; i < count && left > 1e6; i++) {
    const share = i === count - 1 ? left : left * (0.18 + rnd() * 0.35);
    left -= share;
    const isDir = rnd() > 0.2;
    const name = isDir ? `${names[Math.floor(rnd() * names.length)]} ${i + 1}` : `arquivo-${i + 1}.${["mov", "zip", "dmg", "raw"][i % 4]}`;
    children.push({ ...nested(`${path}/${name}`, share, isDir ? depth - 1 : 0), isDir, hasChildren: isDir });
  }
  return {
    id: nodeId++,
    name: path.split("/").pop() || path,
    path,
    size,
    allocated: size,
    files: Math.round(size / 2e6),
    isDir: true,
    looseFilesSize: size - children.reduce((s, c) => s + c.size, 0),
    hasChildren: children.length > 0,
    children: depth > 0 ? children : null,
  };
}
const nodeSizes = new Map<string, number>([[HOME, 318e9]]);
registerMock("storage_node", ({ path, depth }) => {
  const p = (path as string | undefined) ?? HOME;
  const n = nested(p, nodeSizes.get(p) ?? 40e9, (depth as number) ?? 1);
  const remember = (x: StorageNode) => {
    nodeSizes.set(x.path, x.size);
    x.children?.forEach(remember);
  };
  remember(n);
  return n;
});

const app = (name: string, id: string, size: number, lastUsedDays: number | null, extra: Partial<AppInfo> = {}): AppInfo => ({
  path: `/Applications/${name}.app`,
  name,
  bundleId: id,
  version: "1." + (name.length % 9) + ".2",
  build: "100",
  minimumSystem: "13.0",
  size,
  modifiedAt: now - 40 * DAY,
  lastUsedAt: lastUsedDays == null ? null : now - lastUsedDays * DAY,
  fromAppStore: false,
  systemApp: false,
  running: false,
  iconFile: null,
  ...extra,
});
const demoApps = [
  app("Xcode", "com.apple.dt.Xcode", 9.45e9, 1, { fromAppStore: true, running: true }),
  app("Android Studio", "com.google.android.studio", 2.91e9, 240),
  app("Docker", "com.docker.docker", 2.56e9, 12),
  app("Figma", "com.figma.Desktop", 612e6, 3),
  app("Google Chrome", "com.google.Chrome", 1.5e9, 0, { running: true }),
  app("Visual Studio Code", "com.microsoft.VSCode", 1.46e9, 0),
  app("Zoom", "us.zoom.xos", 382e6, 410),
  app("GarageBand", "com.apple.garageband10", 1.1e9, 900, { fromAppStore: true }),
  app("Spotify", "com.spotify.client", 410e6, 30),
  app("Safari", "com.apple.Safari", 18e6, 0, { systemApp: true }),
];
registerMock("start_app_list", ({ onEvent }) => streamJob<AppsResult>(onEvent, "/Applications", () => ({ apps: demoApps, totalSize: demoApps.reduce((s, a) => s + a.size, 0) }), 900));
registerMock("app_icon", () => null);
const lf = (path: string, kind: Leftover["kind"], size: number, rule: Leftover["rule"], confidence: Leftover["confidence"]): Leftover => ({
  path: `${HOME}/Library/${path}`,
  kind,
  size,
  rule,
  confidence,
  selected: confidence === "safe",
});
registerMock("uninstall_plan", ({ path }): UninstallPlan => {
  const a = demoApps.find((x) => x.path === path)!;
  return {
    appPath: a.path,
    name: a.name,
    bundleId: a.bundleId,
    appSize: a.size,
    running: a.running,
    protected: a.systemApp,
    fullDiskAccess: false,
    leftovers: [
      lf(`Application Support/${a.name}`, "applicationSupport", 250e6, "knownPath", "safe"),
      lf(`Caches/${a.bundleId}`, "caches", 94e6, "bundleId", "safe"),
      lf(`Preferences/${a.bundleId}.plist`, "preferences", 80e3, "bundleId", "safe"),
      lf(`Logs/${a.name}`, "logs", 18e6, "appName", "review"),
      lf(`Saved Application State/${a.bundleId}.savedState`, "savedState", 1.2e6, "bundleId", "safe"),
      lf(`Group Containers/ABCDE12345.${a.bundleId?.split(".").slice(0, 2).join(".")}.shared`, "groupContainer", 40e6, "sharedGroup", "danger"),
    ],
  };
});
registerMock("uninstall_app", ({ path, leftovers }) => [
  { path, size: 612e6, ok: true, error: null },
  ...(leftovers as string[]).map((p) => ({ path: p, size: 10e6, ok: true, error: null })),
]);
registerMock("start_orphan_scan", ({ onEvent }) =>
  streamJob<OrphansResult>(onEvent, `${HOME}/Library`, () => {
    const groups = [
      { bundleId: "com.tinyspeck.slackmacgap", vendorInstalled: false, items: [lf("Application Support/com.tinyspeck.slackmacgap", "applicationSupport", 275e6, "orphan", "review"), lf("Caches/com.tinyspeck.slackmacgap", "caches", 88e6, "orphan", "safe"), lf("Preferences/com.tinyspeck.slackmacgap.plist", "preferences", 12e3, "orphan", "safe")], size: 363e6 },
      { bundleId: "com.adobe.dunamis", vendorInstalled: true, items: [lf("Caches/com.adobe.dunamis", "caches", 104e6, "orphan", "review")], size: 104e6 },
      { bundleId: "com.duckduckgo.macos.browser", vendorInstalled: false, items: [lf("HTTPStorages/com.duckduckgo.macos.browser", "httpStorage", 20e6, "orphan", "safe"), lf("Preferences/com.duckduckgo.macos.browser.plist", "preferences", 40e3, "orphan", "safe")], size: 20e6 },
    ];
    return { groups, totalSize: groups.reduce((s, g) => s + g.size, 0), fullDiskAccess: false };
  }, 900),
);
registerMock("remove_orphans", ({ items }) => (items as { path: string; size: number }[]).map((i) => ({ ...i, ok: true, error: null })));

let menuBar = { enabled: true, title: "cpu" };
registerMock("menu_bar_settings", () => menuBar);
registerMock("set_menu_bar_settings", ({ settings }) => {
  menuBar = settings as typeof menuBar;
  return null;
});
registerMock("health", () => ({
  cpu: 8 + Math.random() * 22,
  cores: 10,
  loadAverage: [2.41, 2.2, 2.05],
  memoryTotal: 24e9,
  memoryUsed: 16.8e9 + Math.random() * 0.4e9,
  memoryAvailable: 7e9,
  swapTotal: 3.2e9,
  swapUsed: 1.1e9,
  diskTotal: 994.66e9,
  diskFree: 212.4e9,
  uptime: 3 * 86400 + 5 * 3600 + 12 * 60,
  battery: { percent: 87, charging: false, onAc: false, minutesRemaining: 312, state: "discharging" },
}));
registerMock("process_list", () => {
  const apps = [
    { name: "Google Chrome", appPath: "/Applications/Google Chrome.app", memory: 3.9e9, cpu: 12.4, processes: 23, canQuit: true },
    { name: "Xcode", appPath: "/Applications/Xcode.app", memory: 2.7e9, cpu: 4.1, processes: 6, canQuit: true },
    { name: "Visual Studio Code", appPath: "/Applications/Visual Studio Code.app", memory: 1.6e9, cpu: 2.3, processes: 11, canQuit: true },
    { name: "Figma", appPath: "/Applications/Figma.app", memory: 980e6, cpu: 1.1, processes: 5, canQuit: true },
    { name: "Spotify", appPath: "/Applications/Spotify.app", memory: 410e6, cpu: 0.6, processes: 4, canQuit: true },
  ];
  const processes = [
    ...apps.map((a, i) => ({ pid: 500 + i, name: a.name, memory: a.memory, cpu: a.cpu, category: "application", appPath: a.appPath, user: "demo" })),
    { pid: 900, name: "node", memory: 620e6, cpu: 3.2, category: "background", appPath: null, user: "demo" },
    { pid: 901, name: "rust-analyzer", memory: 1.1e9, cpu: 0.4, category: "background", appPath: null, user: "demo" },
    { pid: 1, name: "launchd", memory: 30e6, cpu: 0.1, category: "system", appPath: null, user: "root" },
    { pid: 120, name: "WindowServer", memory: 890e6, cpu: 6.2, category: "system", appPath: null, user: "_windowserver" },
    { pid: 140, name: "mds_stores", memory: 210e6, cpu: 1.4, category: "system", appPath: null, user: "root" },
  ];
  return { processes, apps };
});
registerMock("quit_application", () => 1);

const PICS = ["1e3a8a,3b82f6", "7c2d12,f97316", "14532d,22c55e", "581c87,a855f7"];
const svgThumb = (i: number, v: number) => {
  const [a, b] = PICS[i % PICS.length].split(",");
  const svg = `<svg xmlns='http://www.w3.org/2000/svg' width='320' height='240'><defs><linearGradient id='g' x1='0' y1='0' x2='1' y2='1'><stop offset='0' stop-color='#${a}'/><stop offset='1' stop-color='#${b}'/></linearGradient></defs><rect width='320' height='240' fill='url(#g)'/><circle cx='${210 + v * 6}' cy='${90 - v * 4}' r='${38 - v * 3}' fill='rgba(255,255,255,.75)'/><path d='M0 200 L90 120 L160 180 L230 110 L320 190 L320 240 L0 240Z' fill='rgba(0,0,0,.35)'/></svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
};
registerMock("image_thumbnail", ({ path }) => {
  const m = /g(\d)-(\d)/.exec(path as string);
  return m ? svgThumb(+m[1], +m[2]) : null;
});
registerMock("photos_status", () => "notDetermined");
registerMock("start_similar_images", ({ root, onEvent }) =>
  streamJob<SimilarResult>(onEvent, root as string, () => {
    const groups = [0, 1, 2].map((gi) => {
      const images = [0, 1, 2, 3].slice(0, gi === 1 ? 2 : gi === 2 ? 4 : 3).map((v) => ({
        ...fakeFile(`${HOME}/Pictures/Viagem/g${gi}-${v} IMG_${4020 + gi * 10 + v}.jpg`, [6.2e6, 2.1e6, 890e3, 4.4e6][v], 300 - v, "image"),
        width: [4032, 2016, 1200, 4032][v],
        height: [3024, 1512, 900, 3024][v],
        distance: v === 0 ? 0.08 : 0.1 + v * 0.05,
        keep: v === 0,
      }));
      return { id: images[0].path, images, reclaimable: images.filter((x) => !x.keep).reduce((s, x) => s + x.sizeLogical, 0) };
    });
    return { ...baseScan(root as string), groups, images: 1840, truncated: false, vision: { analyzed: 1840, candidates: 212, featurePrints: 212, failed: 3, elapsedMs: 9400 }, reclaimable: groups.reduce((s, g) => s + g.reclaimable, 0) };
  }, 1500),
);

registerMock("start_smart_care", ({ onEvent }) => {
  const id = (onEvent as { id: number }).id;
  let index = 0;
  const send = (message: unknown) =>
    (window as unknown as { __TAURI_INTERNALS__: { runCallback: (id: number, d: unknown) => void } }).__TAURI_INTERNALS__.runCallback(id, { index: index++, message });
  const stages = ["trash", "downloads", "home", "duplicates", "apps", "leftovers"];
  stages.forEach((s, i) => setTimeout(() => send({ event: "stage", stage: s, done: i, total: 6 }), 250 * i));
  setTimeout(() => {
    const dl = (name: string, size: number, age: number, sel: boolean): DownloadItem => ({ ...fakeFile(`${HOME}/Downloads/${name}`, size, age, "diskImage"), group: sel ? "oldInstaller" : "recent", ageDays: age, extracted: false, confidence: sel ? "safe" : "review", selected: sel });
    const result: SmartCareReport = {
      diskTotal: 994.66e9,
      diskFree: 82e9,
      trash: { path: `${HOME}/.Trash`, readable: true, files: 412, folders: 20, bytes: 2.5e9, items: [fakeFile(`${HOME}/.Trash/old-project.zip`, 2.5e9, 12, "archive")] },
      downloads: [dl("Figma-126.1.dmg", 540e6, 60, true), dl("Docker.dmg", 610e6, 120, true), dl("Zoom.pkg", 98e6, 210, true), dl("apresentacao.key", 44e6, 12, false)],
      duplicates: [dupGroup("Viagem Patagônia 4K.mov", 1.4e9, ["Movies", "Desktop/Backup vídeos"], "video"), dupGroup("Backup iPhone 2024.zip", 1.7e9, ["Documents", "Downloads"], "archive")],
      largeOld: [fakeFile(`${HOME}/Documents/VM/Windows 11.vhdx`, 5.2e9, 700, "diskImage")],
      unusedApps: demoApps.filter((a) => (a.lastUsedAt ?? 0) < Date.now() - 180 * DAY),
      leftovers: [{ bundleId: "com.tinyspeck.slackmacgap", vendorInstalled: false, items: [lf("Caches/com.tinyspeck.slackmacgap", "caches", 88e6, "orphan", "safe")], size: 88e6 }],
      recommendations: [
        { id: "lowDisk", level: "high", title: "Pouco espaço livre", detail: "Restam 82,0 GB (8% do disco). Abaixo de 10% o macOS pode ficar lento e falhar em atualizações.", bytes: 0 },
        { id: "duplicates", level: "suggest", title: "Remover cópias duplicadas", detail: "Arquivos idênticos de mais de 10 MB ocupam 3,1 GB a mais.", bytes: 3.1e9 },
        { id: "emptyTrash", level: "suggest", title: "Esvaziar a Lixeira", detail: "A Lixeira ocupa 2,5 GB.", bytes: 2.5e9 },
        { id: "installers", level: "suggest", title: "Revisar instaladores em Downloads", detail: "3 instaladores; os antigos somam 1,2 GB.", bytes: 1.2e9 },
      ],
      durationMs: 15300,
    };
    send({ event: "complete", result });
    send({ end: true, index });
  }, 1700);
  return crypto.randomUUID();
});
registerMock("run_smart_care", ({ plan }) => {
  const p = plan as { files: { path: string; size: number }[]; leftovers: { path: string; size: number }[] };
  return [...p.files, ...p.leftovers].map((i) => ({ ...i, ok: true, error: null }));
});

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
