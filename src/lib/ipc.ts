// Typed wrappers around the Rust commands. The UI never touches the filesystem or runs commands
// itself: everything goes through these functions.
import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  AppError,
  AppInfo,
  OperationOutcome,
  TrashSummary,
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
  storageNode: (scanId: string, path?: string, depth = 1) =>
    invoke<StorageNode>("storage_node", { scanId, path, depth }),
  permissionStatus: () => invoke<PermissionStatus>("permission_status"),
  openSystemSettings: (pane: "fullDiskAccess" | "loginItems" | "storage") =>
    invoke<void>("open_system_settings", { pane }),
  suggestedLocations: () => invoke<Location[]>("suggested_locations"),
  appInfo: () => invoke<AppInfo>("app_info"),
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
};

export type { ScanResult };
