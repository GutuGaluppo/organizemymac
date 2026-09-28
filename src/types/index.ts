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

export type AppInfo = { version: string; dataDir: string; logDir: string };

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
