import { useMemo, useState, type ReactNode } from "react";
import { ArrowDown, ArrowUp, Ban, Eye, FolderOpen } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import type { FileEntry } from "../types";
import { formatAge, formatBytes, formatDate, parentFolder, shortPath } from "../lib/format";
import { CategoryIcon, typeLabel } from "../lib/categories";
import { api } from "../lib/ipc";

type SortKey = "name" | "size" | "modified" | "type" | "location";

export type Row = FileEntry & { note?: ReactNode };

/** Quick Look, Reveal in Finder and Ignore for one file. */
export function RowActions({ entry, onIgnored }: { entry: FileEntry; onIgnored?: (path: string) => void }) {
  const qc = useQueryClient();
  const ignore = async () => {
    await api.addToIgnoreList(entry.path, "ignorado em uma lista de resultados");
    qc.invalidateQueries({ queryKey: ["ignore"] });
    onIgnored?.(entry.path);
  };
  const btn = "grid size-6 place-items-center rounded text-ink-3 hover:bg-surface-3 hover:text-ink";
  return (
    <div className="flex items-center gap-0.5 opacity-0 transition group-hover:opacity-100 focus-within:opacity-100">
      {!entry.isDirectory && (
        <button className={btn} title="Visualização Rápida" onClick={() => api.quickLook(entry.path)}>
          <Eye className="size-3.5" />
        </button>
      )}
      <button className={btn} title="Mostrar no Finder" onClick={() => api.revealInFinder(entry.path)}>
        <FolderOpen className="size-3.5" />
      </button>
      <button className={btn} title="Ignorar (não sugerir mais)" onClick={ignore}>
        <Ban className="size-3.5" />
      </button>
    </div>
  );
}

function Checkbox({ checked, indeterminate, onChange, label }: { checked: boolean; indeterminate?: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <input
      type="checkbox"
      aria-label={label}
      className="size-3.5 accent-[var(--accent)]"
      checked={checked}
      ref={(el) => {
        if (el) el.indeterminate = !!indeterminate && !checked;
      }}
      onChange={(e) => onChange(e.target.checked)}
    />
  );
}

/** Name | Size | Modified | Type | Location, with selection, sorting and row actions. */
export function FileTable({
  rows,
  selected,
  onSelect,
  onIgnored,
  home,
  initialSort = "size",
  maxHeight,
}: {
  rows: Row[];
  selected: Set<string>;
  onSelect: (paths: string[], value: boolean) => void;
  onIgnored?: (path: string) => void;
  home?: string;
  initialSort?: SortKey;
  maxHeight?: number;
}) {
  const [sort, setSort] = useState<{ key: SortKey; desc: boolean }>({ key: initialSort, desc: initialSort === "size" });
  const sorted = useMemo(() => {
    const val = (r: Row): string | number => {
      switch (sort.key) {
        case "name":
          return r.name.toLowerCase();
        case "size":
          return r.sizeLogical;
        case "modified":
          return r.modifiedAt ?? 0;
        case "type":
          return typeLabel(r);
        case "location":
          return r.path.toLowerCase();
      }
    };
    return [...rows].sort((a, b) => {
      const x = val(a);
      const y = val(b);
      const c = x < y ? -1 : x > y ? 1 : 0;
      return sort.desc ? -c : c;
    });
  }, [rows, sort]);

  const allSelected = rows.length > 0 && rows.every((r) => selected.has(r.path));
  const someSelected = rows.some((r) => selected.has(r.path));
  const header = (key: SortKey, label: string, align = "text-left") => (
    <button
      className={`flex items-center gap-1 ${align === "text-right" ? "justify-end" : ""} hover:text-ink`}
      onClick={() => setSort((s) => ({ key, desc: s.key === key ? !s.desc : key === "size" || key === "modified" }))}
    >
      {label}
      {sort.key === key && (sort.desc ? <ArrowDown className="size-3" /> : <ArrowUp className="size-3" />)}
    </button>
  );
  const grid = "grid grid-cols-[22px_minmax(0,2.2fr)_88px_110px_110px_minmax(0,1.6fr)_76px] items-center gap-3 px-4";

  return (
    <div className="overflow-hidden rounded-xl glass">
      <div className={`${grid} h-8 border-b border-line bg-surface-2 text-[11px] font-medium text-ink-3`}>
        <Checkbox
          label="Selecionar todos"
          checked={allSelected}
          indeterminate={someSelected}
          onChange={(v) => onSelect(rows.map((r) => r.path), v)}
        />
        {header("name", "Nome")}
        <div className="text-right">{header("size", "Tamanho", "text-right")}</div>
        {header("modified", "Modificado")}
        {header("type", "Tipo")}
        {header("location", "Local")}
        <span />
      </div>
      <div className="overflow-y-auto" style={{ maxHeight: maxHeight ?? undefined }}>
        {sorted.map((r) => (
          <div
            key={r.path}
            className={`group ${grid} min-h-10 border-b border-line py-1.5 last:border-0 ${selected.has(r.path) ? "bg-accent-soft/60" : "hover:bg-surface-2"}`}
          >
            <Checkbox label={`Selecionar ${r.name}`} checked={selected.has(r.path)} onChange={(v) => onSelect([r.path], v)} />
            <div className="flex min-w-0 items-center gap-2">
              <CategoryIcon entry={r} />
              <div className="min-w-0">
                <div className="truncate text-[12.5px]" title={r.name}>
                  {r.name}
                </div>
                {r.note && <div className="truncate text-[11px] text-ink-3">{r.note}</div>}
              </div>
            </div>
            <div className="tabular text-right text-[12.5px]">{formatBytes(r.sizeLogical)}</div>
            <div className="text-[12px] text-ink-2" title={formatDate(r.modifiedAt)}>
              {formatAge(r.modifiedAt)}
            </div>
            <div className="truncate text-[12px] text-ink-2">{typeLabel(r)}</div>
            <div className="truncate text-[12px] text-ink-3 selectable" title={r.path}>
              {shortPath(parentFolder(r.path), home)}
            </div>
            <RowActions entry={r} onIgnored={onIgnored} />
          </div>
        ))}
      </div>
    </div>
  );
}

/** Selection helpers shared by result screens. */
export function useSelection(initial: Iterable<string> = []) {
  const [selected, setSelected] = useState<Set<string>>(() => new Set(initial));
  const onSelect = (paths: string[], value: boolean) =>
    setSelected((prev) => {
      const next = new Set(prev);
      for (const p of paths) {
        if (value) next.add(p);
        else next.delete(p);
      }
      return next;
    });
  return { selected, setSelected, onSelect };
}
