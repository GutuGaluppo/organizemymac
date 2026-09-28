// Sizes use base 10, like Finder and "About This Mac" (1 GB = 1,000,000,000 bytes).
const units = ["bytes", "KB", "MB", "GB", "TB"];

const numberFormat = (digits: number) =>
  new Intl.NumberFormat("pt-BR", { maximumFractionDigits: digits, minimumFractionDigits: 0 });

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 KB";
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit++;
  }
  if (unit === 0) return bytes < 1000 ? `${bytes} bytes` : "1 KB";
  const digits = value >= 100 ? 0 : 1;
  return `${numberFormat(digits).format(value)} ${units[unit]}`;
}

export function formatCount(n: number): string {
  return new Intl.NumberFormat("pt-BR").format(n);
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  const s = ms / 1000;
  if (s < 60) return `${numberFormat(1).format(s)} s`;
  const m = Math.floor(s / 60);
  return `${m} min ${Math.round(s % 60)} s`;
}

const dateFormat = new Intl.DateTimeFormat("pt-BR", { day: "2-digit", month: "short", year: "numeric" });
const dateTimeFormat = new Intl.DateTimeFormat("pt-BR", { dateStyle: "short", timeStyle: "short" });

export function formatDate(ms?: number | null): string {
  return ms ? dateFormat.format(new Date(ms)) : "—";
}

export function formatDateTime(ms?: number | null): string {
  return ms ? dateTimeFormat.format(new Date(ms)) : "—";
}

const relative = new Intl.RelativeTimeFormat("pt-BR", { numeric: "auto" });

/** "há 3 meses", "ontem"… */
export function formatAge(ms?: number | null): string {
  if (!ms) return "—";
  const days = Math.round((ms - Date.now()) / 86_400_000);
  if (Math.abs(days) < 1) return "hoje";
  if (Math.abs(days) < 31) return relative.format(days, "day");
  if (Math.abs(days) < 365) return relative.format(Math.round(days / 30), "month");
  return relative.format(Math.round(days / 365), "year");
}

/** Replaces the home folder with "~". */
export function shortPath(path: string, home?: string): string {
  return home && path.startsWith(home) ? "~" + path.slice(home.length) : path;
}

export function parentFolder(path: string): string {
  const i = path.lastIndexOf("/");
  return i > 0 ? path.slice(0, i) : "/";
}
