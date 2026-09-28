// Mirrors of the Rust types in src-tauri/src (serde camelCase). Keep in sync.

export type FileCategory =
  | "image"
  | "video"
  | "audio"
  | "document"
  | "archive"
  | "diskImage"
  | "installer"
  | "code"
  | "application"
  | "other";

export type FileEntry = {
  path: string;
  name: string;
  sizeLogical: number;
  sizeAllocated: number;
  /** Unix time in milliseconds. */
  createdAt?: number | null;
  modifiedAt?: number | null;
  extension?: string | null;
  category: FileCategory;
  isDirectory: boolean;
  isSymlink: boolean;
  isCloudPlaceholder: boolean;
};

export type Confidence = "safe" | "review" | "danger";

export type CleanupCandidate = {
  path: string;
  size: number;
  reason: string;
  confidence: Confidence;
  selected: boolean;
};

export type OperationOutcome = {
  path: string;
  size: number;
  ok: boolean;
  error?: string | null;
};

export type ScanProgress = {
  files: number;
  directories: number;
  bytesScanned: number;
  current: string;
  elapsedMs: number;
  warnings: number;
  /** macOS is holding a read, usually while a permission prompt is on screen. */
  waiting: boolean;
};

export type ScanStats = {
  root: string;
  files: number;
  directories: number;
  bytesScanned: number;
  bytesAllocated: number;
  symlinks: number;
  cloudPlaceholders: number;
  warnings: number;
  durationMs: number;
  cancelled: boolean;
};

export type StorageNode = {
  id: number;
  name: string;
  path: string;
  size: number;
  allocated: number;
  files: number;
  isDir: boolean;
  looseFilesSize: number;
  hasChildren: boolean;
  children?: StorageNode[] | null;
};

export type ScanResult = ScanStats & {
  id: string;
  startedAt: number;
  finishedAt: number;
  reclaimableBytes: number;
  largestFiles: FileEntry[];
  tree?: StorageNode | null;
};

export type JobEvent<T> =
  | { event: "start"; jobId: string; root: string }
  | ({ event: "progress" } & ScanProgress)
  | { event: "warning"; path: string; message: string }
  | { event: "stage"; stage: string; done: number; total: number }
  | { event: "complete"; result: T }
  | { event: "cancelled"; result: T }
  | { event: "failed"; message: string };

export type AppError = { kind: string; message: string };

export type AccessStatus = "granted" | "denied" | "unknown";

export type PermissionStatus = { fullDiskAccess: AccessStatus; home: string };

export type Location = { id: string; path: string; exists: boolean };

export type ScanRecord = {
  id: string;
  module: string;
  root: string;
  startedAt: number;
  finishedAt?: number | null;
  files: number;
  directories: number;
  bytesScanned: number;
  bytesAllocated: number;
  reclaimableBytes: number;
  warnings: number;
  durationMs: number;
  cancelled: boolean;
};

export type OperationRecord = {
  id: number;
  ts: number;
  action: string;
  path: string;
  size: number;
  ok: boolean;
  detail?: string | null;
};

export type IgnoreEntry = { path: string; addedAt: number; reason?: string | null };

export type BuildInfo = { version: string; dataDir: string; logDir: string };

export type Volume = {
  name: string;
  mountPoint: string;
  fileSystem: string;
  total: number;
  free: number;
  used: number;
  isRoot: boolean;
  readOnly: boolean;
  local: boolean;
};

export type TrashSummary = {
  path: string;
  readable: boolean;
  files: number;
  folders: number;
  bytes: number;
  items: FileEntry[];
};

export type FileFilter = {
  minSize: number;
  olderThanDays?: number | null;
  dateField: "modified" | "created";
  extensions: string[];
  categories: FileCategory[];
  excludeLibrary: boolean;
  includeHidden: boolean;
};

export type FindResult = ScanResult & {
  matches: FileEntry[];
  matched: number;
  matchedBytes: number;
  truncated: boolean;
};

export type DownloadGroup = "oldInstaller" | "largeFile" | "archive" | "installer" | "screenshot" | "oldFile" | "recent";

export type DownloadItem = FileEntry & {
  group: DownloadGroup;
  ageDays?: number | null;
  extracted: boolean;
  confidence: Confidence;
  selected: boolean;
};

export type DownloadsResult = ScanResult & {
  folder: string;
  items: DownloadItem[];
  totalBytes: number;
};

export type DuplicateFile = FileEntry & { hardLinks: string[]; selected: boolean };

export type DuplicateGroup = { hash: string; size: number; files: DuplicateFile[]; wasted: number };

export type HashStats = {
  candidates: number;
  afterSize: number;
  sampleHashed: number;
  fullHashed: number;
  bytesHashed: number;
  hashMs: number;
};

export type DuplicatesResult = ScanResult & {
  groups: DuplicateGroup[];
  totalGroups: number;
  wastedBytes: number;
  hashStats: HashStats;
};

export type AppInfo = {
  path: string;
  name: string;
  bundleId?: string | null;
  version?: string | null;
  build?: string | null;
  minimumSystem?: string | null;
  size: number;
  modifiedAt?: number | null;
  lastUsedAt?: number | null;
  fromAppStore: boolean;
  systemApp: boolean;
  running: boolean;
  iconFile?: string | null;
};

export type AppsResult = { apps: AppInfo[]; totalSize: number };

export type LeftoverKind =
  | "applicationSupport"
  | "caches"
  | "preferences"
  | "logs"
  | "savedState"
  | "webKit"
  | "httpStorage"
  | "cookies"
  | "container"
  | "groupContainer"
  | "launchAgent"
  | "applicationScripts";

export type MatchRule = "bundleId" | "knownPath" | "vendor" | "appName" | "fuzzy" | "sharedGroup" | "orphan";

export type Leftover = { path: string; kind: LeftoverKind; size: number; rule: MatchRule; confidence: Confidence; selected: boolean };

export type UninstallPlan = {
  appPath: string;
  name: string;
  bundleId?: string | null;
  appSize: number;
  leftovers: Leftover[];
  running: boolean;
  protected: boolean;
  fullDiskAccess: boolean;
};

export type OrphanGroup = { bundleId: string; vendorInstalled: boolean; items: Leftover[]; size: number };

export type OrphansResult = { groups: OrphanGroup[]; totalSize: number; fullDiskAccess: boolean };

export type Battery = { percent: number; charging: boolean; onAc: boolean; minutesRemaining?: number | null; state: string };

export type Health = {
  cpu: number;
  cores: number;
  loadAverage: [number, number, number];
  memoryTotal: number;
  memoryUsed: number;
  memoryAvailable: number;
  swapTotal: number;
  swapUsed: number;
  diskTotal: number;
  diskFree: number;
  uptime: number;
  battery?: Battery | null;
};

export type ProcessCategory = "application" | "background" | "system";

export type ProcessInfo = { pid: number; name: string; memory: number; cpu: number; category: ProcessCategory; appPath?: string | null; user?: string | null };

export type AppUsage = { name: string; appPath?: string | null; memory: number; cpu: number; processes: number; canQuit: boolean };

export type ProcessList = { processes: ProcessInfo[]; apps: AppUsage[] };

export type MenuBarSettings = { enabled: boolean; title: "icon" | "cpu" | "memory" | "both" };

export type Sensitivity = "strict" | "normal";

export type SimilarImage = FileEntry & { width: number; height: number; distance: number; keep: boolean };

export type SimilarGroup = { id: string; images: SimilarImage[]; reclaimable: number };

export type VisionStats = { analyzed: number; candidates: number; featurePrints: number; failed: number; elapsedMs: number };

export type SimilarResult = ScanResult & { groups: SimilarGroup[]; images: number; truncated: boolean; vision: VisionStats; reclaimable: number };

export type PhotoMember = { id: string; width: number; height: number; distance: number; keep: boolean };

export type SimilarPhotosResult = { groups: PhotoMember[][]; vision: VisionStats };

export type Recommendation = { id: string; level: "high" | "suggest"; title: string; detail: string; bytes: number };

export type SmartCareReport = {
  diskTotal: number;
  diskFree: number;
  trash: TrashSummary;
  downloads: DownloadItem[];
  duplicates: DuplicateGroup[];
  largeOld: FileEntry[];
  unusedApps: AppInfo[];
  leftovers: OrphanGroup[];
  recommendations: Recommendation[];
  durationMs: number;
};

export type CarePlan = {
  emptyTrash: string[];
  files: { path: string; size: number }[];
  duplicates: { all: string[]; remove: { path: string; size: number; modifiedAt: number | null }[] }[];
  leftovers: { path: string; size: number }[];
};

export type Risk = "low" | "medium" | "high";

export type CleanupRule = {
  id: string;
  group: "user" | "developer" | "browser" | "mail" | "custom";
  title: string;
  description: string;
  paths: string[];
  risk: Risk;
  requiresClosed: string[];
  app?: string | null;
  perItemRunningCheck: boolean;
  needsFullDiskAccess: boolean;
  selected: boolean;
};

export type RuleItem = { path: string; size: number; selected: boolean; blocked?: string | null; system: boolean };

export type RuleResult = CleanupRule & { items: RuleItem[]; size: number; blockedBy: string[]; unavailable: boolean };

export type CustomRuleDef = { id: string; title: string; folder: string; risk: Risk };

export type RulesReport = { results: RuleResult[]; custom: CustomRuleDef[]; fullDiskAccess: boolean };

export type CloudProvider = "iCloud" | "googleDrive" | "dropbox" | "oneDrive" | "box" | "other";

export type CloudFolder = { provider: CloudProvider; name: string; path: string; canEvict: boolean };

export type CloudUsage = CloudFolder & {
  files: number;
  totalBytes: number;
  localBytes: number;
  cloudOnlyFiles: number;
  largestLocal: FileEntry[];
  cancelled: boolean;
};

export type EvictOutcome = { path: string; ok: boolean; error?: string | null };

export type UpdateSource = "appStore" | "sparkle" | "none";

export type UpdateInfo = {
  path: string;
  name: string;
  bundleId?: string | null;
  installed?: string | null;
  source: UpdateSource;
  feedUrl?: string | null;
  latest?: string | null;
  updateAvailable: boolean;
  url?: string | null;
  error?: string | null;
};

export type UpdatesResult = { apps: UpdateInfo[]; checkedOnline: boolean };

export type SignatureKind = "apple" | "appStore" | "developerId" | "development" | "adHoc" | "unsigned" | "invalid" | "unknown";

export type Signature = { kind: SignatureKind; teamId?: string | null; authority?: string | null };

export type FindingLevel = "info" | "notice" | "attention";

export type Finding = { level: FindingLevel; code: string };

export type PersistenceItem = {
  scope: "user" | "allUsers" | "system";
  plist: string;
  label?: string | null;
  program?: string | null;
  arguments: string[];
  runAtLoad: boolean;
  keepAlive: boolean;
  disabled: boolean;
  programExists: boolean;
  signature?: Signature | null;
  app?: string | null;
  findings: Finding[];
};

export type AppAudit = { path: string; name: string; signature: Signature; gatekeeper?: string | null; gatekeeperChecked: boolean; quarantined: boolean; findings: Finding[] };

export type SecurityAudit = { persistence: PersistenceItem[]; apps: AppAudit[] };
