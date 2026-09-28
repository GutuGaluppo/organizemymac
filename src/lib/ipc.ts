// Typed wrappers around the Rust commands. The UI never touches the filesystem or runs commands
// itself: everything goes through these functions.
import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  AppError,
  BuildInfo,
  OperationOutcome,
  TrashSummary,
  Health,
  ProcessList,
  MenuBarSettings,
  UninstallPlan,
  Volume,
  IgnoreEntry,
  JobEvent,
  Location,
  OperationRecord,
  PermissionStatus,
  ScanRecord,
  ScanResult,
  StorageNode,
} from "../types";

export function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) return String((err as AppError).message);
  return String(err);
}

/** Starts a background job; events arrive on `onEvent`. Resolves with the job id. */
export function startJob<T>(command: string, args: Record<string, unknown>, onEvent: (e: JobEvent<T>) => void) {
  const channel = new Channel<JobEvent<T>>();
  channel.onmessage = onEvent;
  return invoke<string>(command, { ...args, onEvent: channel });
}

export const api = {
  cancelJob: (jobId: string) => invoke<boolean>("cancel_job", { jobId }),
  storageNode: (scanId: string, path?: string, depth = 1, limit = 200) =>
    invoke<StorageNode>("storage_node", { scanId, path, depth, limit }),
  permissionStatus: () => invoke<PermissionStatus>("permission_status"),
  openSystemSettings: (pane: "fullDiskAccess" | "loginItems" | "storage" | "photos") =>
    invoke<void>("open_system_settings", { pane }),
  suggestedLocations: () => invoke<Location[]>("suggested_locations"),
  appInfo: () => invoke<BuildInfo>("app_info"),
  recentScans: (module?: string, limit?: number) => invoke<ScanRecord[]>("recent_scans", { module, limit }),
  operationLog: (limit?: number) => invoke<OperationRecord[]>("operation_log", { limit }),
  localMetrics: () => invoke<Record<string, number>>("local_metrics"),
  ignoreList: () => invoke<IgnoreEntry[]>("ignore_list"),
  addToIgnoreList: (path: string, reason?: string) => invoke<void>("add_to_ignore_list", { path, reason }),
  removeFromIgnoreList: (path: string) => invoke<void>("remove_from_ignore_list", { path }),
  diskOverview: () => invoke<Volume[]>("disk_overview"),
  trashSummary: () => invoke<TrashSummary>("trash_summary"),
  emptyTrash: () => invoke<OperationOutcome[]>("empty_trash"),
  emptyTrashWithFinder: () => invoke<void>("empty_trash_with_finder"),
  revealInFinder: (path: string) => invoke<void>("reveal_in_finder", { path }),
  quickLook: (path: string) => invoke<void>("quick_look", { path }),
  moveToTrash: (request: { items: { path: string; size: number }[]; scanRoot?: string }) =>
    invoke<OperationOutcome[]>("move_to_trash", { request }),
  pathExists: (path: string) => invoke<boolean>("path_exists", { path }),
  imageThumbnail: (path: string) => invoke<string | null>("image_thumbnail", { path }),
  photosStatus: (request = false) => invoke<string>("photos_status", { request }),
  photoThumbnail: (id: string) => invoke<string | null>("photo_thumbnail", { id }),
  deletePhotos: (ids: string[]) => invoke<boolean>("delete_photos", { ids }),
  health: () => invoke<Health>("health"),
  processList: () => invoke<ProcessList>("process_list"),
  quitApplication: (appPath: string) => invoke<number>("quit_application", { appPath }),
  menuBarSettings: () => invoke<MenuBarSettings>("menu_bar_settings"),
  setMenuBarSettings: (settings: MenuBarSettings) => invoke<void>("set_menu_bar_settings", { settings }),
  appIcon: (path: string) => invoke<string | null>("app_icon", { path }),
  uninstallPlan: (path: string) => invoke<UninstallPlan>("uninstall_plan", { path }),
  uninstallApp: (path: string, leftovers: string[]) => invoke<OperationOutcome[]>("uninstall_app", { path, leftovers }),
  removeOrphans: (items: { path: string; size: number }[]) => invoke<OperationOutcome[]>("remove_orphans", { items }),
  moveDuplicatesToTrash: (
    groups: { all: string[]; remove: { path: string; size: number; modifiedAt: number | null }[] }[],
    scanRoot?: string,
  ) => invoke<OperationOutcome[]>("move_duplicates_to_trash", { groups, scanRoot }),
};

export type { ScanResult };
