import { useEffect, useMemo, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { ChevronRight, ChevronDown, Eye, File, Folder, FolderOpen, House, Map as MapIcon, RotateCcw } from "lucide-react";
import { api } from "../../lib/ipc";
import { useJob, useJobs } from "../../stores/jobs";
import { Button, EmptyState, ErrorNote, PageHeader } from "../../components/ui";
import { ReviewDialog, SelectionBar } from "../../components/ReviewDialog";
import { LocationPicker, ScanProgressCard, useHome } from "../scanner/ScannerPage";
import { squarify, type Tile } from "../../lib/treemap";
import { formatBytes, formatCount, shortPath } from "../../lib/format";
import type { ScanResult, StorageNode } from "../../types";

const MODULE = "scanner"; // the Space Map shows the Scanner's last scan
const PALETTE = ["#0f9d8a", "#3b82f6", "#8b5cf6", "#e0803a", "#d4a017", "#db2777", "#0ea5e9", "#65a30d", "#6366f1", "#14b8a6"];

type Picked = { path: string; size: number; name: string };

function useSize<T extends HTMLElement>() {
  const ref = useRef<T>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  useEffect(() => {
    if (!ref.current) return;
    const ro = new ResizeObserver(([e]) => setSize({ w: e.contentRect.width, h: e.contentRect.height }));
    ro.observe(ref.current);
    return () => ro.disconnect();
  }, []);
  return { ref, ...size };
}

/** Children plus a "smaller files" pseudo-item for the loose files of a folder. */
function tileItems(node: StorageNode): { value: number; item: StorageNode | null }[] {
  const items: { value: number; item: StorageNode | null }[] = (node.children ?? []).map((c) => ({ value: c.size, item: c }));
  const listed = items.reduce((s, i) => s + i.value, 0);
  const rest = node.size - listed;
  if (rest > node.size * 0.002) items.push({ value: rest, item: null });
  return items;
}

function Treemap({
  node,
  selected,
  onOpen,
  onToggle,
}: {
  node: StorageNode;
  selected: Set<string>;
  onOpen: (path: string) => void;
  onToggle: (n: StorageNode) => void;
}) {
  const { ref, w, h } = useSize<HTMLDivElement>();
  const [hover, setHover] = useState<{ name: string; size: number; x: number; y: number } | null>(null);
  const tiles = useMemo(() => squarify(tileItems(node), { x: 0, y: 0, w, h }), [node, w, h]);

  const renderTile = (t: Tile<StorageNode | null>, color: string, depth: number) => {
    const n = t.item;
    const pad = depth === 0 ? 2 : 1;
    const label = t.w > 70 && t.h > 30;
    const isSel = n ? selected.has(n.path) : false;
    const inner = n?.isDir && n.children?.length && depth === 0 && t.w > 90 && t.h > 70
      ? squarify(tileItems(n), { x: 0, y: 18, w: t.w - pad * 2, h: t.h - pad * 2 - 18 })
      : null;
    return (
      <div
        key={n?.path ?? `rest-${depth}-${t.x}-${t.y}`}
        className={`absolute overflow-hidden rounded-[5px] transition-[filter] ${n ? "cursor-pointer hover:brightness-110" : ""} ${isSel ? "ring-2 ring-white ring-inset outline-2 outline-danger" : ""}`}
        style={{
          left: t.x + pad,
          top: t.y + pad,
          width: Math.max(0, t.w - pad * 2),
          height: Math.max(0, t.h - pad * 2),
          background: n ? color : depth === 0 ? "var(--surface-3)" : "rgba(255,255,255,.16)",
          // Inner tiles: a lighter outline and a slightly darker fill so their edges read inside the parent.
          boxShadow: depth > 0 ? "inset 0 0 0 1px rgba(255,255,255,.35)" : undefined,
          filter: depth > 0 && n ? "brightness(0.9)" : undefined,
        }}
        onMouseMove={(e) => {
          e.stopPropagation();
          const box = ref.current!.getBoundingClientRect();
          setHover({ name: n ? n.name : "Arquivos menores", size: t.value, x: e.clientX - box.left, y: e.clientY - box.top });
        }}
        onMouseLeave={() => setHover(null)}
        onClick={(e) => {
          e.stopPropagation();
          if (!n) return;
          if (n.isDir && !e.metaKey) onOpen(n.path);
          else onToggle(n);
        }}
      >
        {label && (
          <div className={`pointer-events-none truncate px-1.5 pt-1 text-[11px] font-medium ${n || depth > 0 ? "text-white" : "text-ink-2"}`} style={{ textShadow: n ? "0 1px 1px rgba(0,0,0,.25)" : undefined }}>
            {n ? n.name : "Arquivos menores"} <span className="font-normal opacity-80">{formatBytes(t.value)}</span>
          </div>
        )}
        {inner?.map((it) => renderTile(it, color, depth + 1))}
      </div>
    );
  };

  return (
    <div ref={ref} className="relative h-full w-full select-none" onMouseLeave={() => setHover(null)}>
      {tiles.map((t, i) => renderTile(t, t.item?.isDir ? PALETTE[i % PALETTE.length] : "#8e8e93", 0))}
      {hover && (
        <div
          className="pointer-events-none absolute z-10 rounded-md bg-black/80 px-2 py-1 text-[11.5px] text-white shadow"
          style={{ left: Math.min(hover.x + 12, w - 200), top: Math.min(hover.y + 12, h - 40) }}
        >
          <div className="max-w-[220px] truncate font-medium">{hover.name}</div>
          <div className="tabular opacity-80">{formatBytes(hover.size)}</div>
        </div>
      )}
    </div>
  );
}

function OutlineRow({
  scanId,
  node,
  depth,
  max,
  selected,
  onOpen,
  onToggle,
}: {
  scanId: string;
  node: StorageNode;
  depth: number;
  max: number;
  selected: Set<string>;
  onOpen: (path: string) => void;
  onToggle: (n: StorageNode) => void;
}) {
  const [open, setOpen] = useState(false);
  const { data } = useQuery({
    queryKey: ["node", scanId, node.path, 1],
    queryFn: () => api.storageNode(scanId, node.path, 1, 100),
    enabled: open && node.hasChildren,
  });
  return (
    <>
      <div className="group flex h-7 items-center gap-1 rounded-md pr-2 hover:bg-surface-2" style={{ paddingLeft: 4 + depth * 14 }}>
        <button className="grid size-4 place-items-center text-ink-3" onClick={() => setOpen(!open)} disabled={!node.hasChildren || !node.isDir}>
          {node.isDir && node.hasChildren ? open ? <ChevronDown className="size-3.5" /> : <ChevronRight className="size-3.5" /> : null}
        </button>
        <input
          type="checkbox"
          className="size-3 accent-[var(--accent)] opacity-0 group-hover:opacity-100 checked:opacity-100"
          checked={selected.has(node.path)}
          onChange={() => onToggle(node)}
          aria-label={`Selecionar ${node.name}`}
        />
        {node.isDir ? <Folder className="size-3.5 shrink-0 text-accent" /> : <File className="size-3.5 shrink-0 text-ink-3" />}
        <button className="min-w-0 flex-1 truncate text-left text-[12px]" onDoubleClick={() => node.isDir && onOpen(node.path)} title={node.name}>
          {node.name}
        </button>
        <div className="relative h-1 w-10 overflow-hidden rounded-full bg-surface-3">
          <div className="h-full bg-accent/70" style={{ width: `${(node.size / Math.max(max, 1)) * 100}%` }} />
        </div>
        <span className="tabular w-16 text-right text-[11.5px] text-ink-2">{formatBytes(node.size)}</span>
      </div>
      {open &&
        data?.children?.map((c) => (
          <OutlineRow key={c.path} scanId={scanId} node={c} depth={depth + 1} max={max} selected={selected} onOpen={onOpen} onToggle={onToggle} />
        ))}
    </>
  );
}

export function SpaceMapPage() {
  const home = useHome();
  const qc = useQueryClient();
  const job = useJob<ScanResult>(MODULE);
  const start = useJobs((s) => s.start);
  const reset = useJobs((s) => s.reset);
  const r = job.status === "running" ? undefined : job.result;
  const [path, setPath] = useState<string | undefined>(undefined);
  const [picked, setPicked] = useState<Map<string, Picked>>(new Map());
  const [review, setReview] = useState(false);

  useEffect(() => {
    setPath(undefined);
    setPicked(new Map());
  }, [r?.id]);

  const { data: node, error, isFetching } = useQuery({
    queryKey: ["node", r?.id, path ?? "", 2],
    queryFn: () => api.storageNode(r!.id, path, 2, 60),
    enabled: !!r?.id,
    placeholderData: (prev) => prev,
  });

  const crumbs = useMemo(() => {
    if (!r || !node) return [];
    const root = r.root.replace(/\/$/, "") || "/";
    const rel = node.path === root ? "" : node.path.slice(root.length).replace(/^\//, "");
    const parts = rel ? rel.split("/") : [];
    return [{ name: root === home ? "Pasta pessoal" : shortPath(root, home), path: root }, ...parts.map((p, i) => ({ name: p, path: `${root === "/" ? "" : root}/${parts.slice(0, i + 1).join("/")}` }))];
  }, [r, node, home]);

  const toggle = (n: StorageNode) =>
    setPicked((prev) => {
      const next = new Map(prev);
      if (next.has(n.path)) next.delete(n.path);
      else {
        // Selecting a folder replaces selections inside it; a file inside a selected folder is ignored.
        if ([...next.keys()].some((p) => n.path.startsWith(p + "/"))) return prev;
        for (const p of [...next.keys()]) if (p.startsWith(n.path + "/")) next.delete(p);
        next.set(n.path, { path: n.path, size: n.size, name: n.name });
      }
      return next;
    });

  const items = [...picked.values()];
  const bytes = items.reduce((s, i) => s + i.size, 0);

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Mapa de espaço"
        subtitle="Cada retângulo tem a área proporcional ao espaço que ocupa. Clique numa pasta para entrar; ⌘-clique ou a caixa na lista seleciona."
        actions={
          r && (
            <Button icon={<RotateCcw className="size-3.5" />} onClick={() => reset(MODULE)}>
              Outra pasta
            </Button>
          )
        }
      />
      {!r && (
        <div className="space-y-4 overflow-y-auto px-8 pb-8">
          {job.status === "running" ? (
            <ScanProgressCard module={MODULE} />
          ) : (
            <>
              {job.status === "failed" && <ErrorNote>{job.error}</ErrorNote>}
              <LocationPicker onPick={(root) => start(MODULE, "start_scan", { root })} />
              <EmptyState icon={<MapIcon className="size-6" />} title="Escolha uma pasta para mapear">
                O mapa usa a mesma análise do Scanner: se você já analisou uma pasta lá, ela aparece aqui.
              </EmptyState>
            </>
          )}
        </div>
      )}
      {r && (
        <>
          <div className="flex items-center gap-1 px-8 pb-3 text-[12.5px]">
            {crumbs.map((c, i) => (
              <span key={c.path} className="flex min-w-0 items-center gap-1">
                {i > 0 && <ChevronRight className="size-3.5 shrink-0 text-ink-3" />}
                <button
                  className={`flex items-center gap-1 truncate rounded px-1.5 py-0.5 hover:bg-surface-3 ${i === crumbs.length - 1 ? "font-semibold text-ink" : "text-ink-2"}`}
                  onClick={() => setPath(i === 0 ? undefined : c.path)}
                >
                  {i === 0 && <House className="size-3.5" />}
                  {c.name}
                </button>
              </span>
            ))}
            <span className="ml-auto shrink-0 text-ink-3">
              {node && `${formatBytes(node.size)} · ${formatCount(node.files)} arquivos`}
              {isFetching && " · carregando…"}
            </span>
          </div>
          {error && (
            <div className="px-8">
              <ErrorNote>O mapa desta análise não está mais na memória. Analise a pasta de novo.</ErrorNote>
            </div>
          )}
          <div className="flex min-h-0 flex-1 gap-4 px-8 pb-4">
            <div className="w-[300px] shrink-0 overflow-y-auto rounded-xl glass p-1.5">
              {node?.children?.map((c) => (
                <OutlineRow key={c.path} scanId={r.id} node={c} depth={0} max={node.children![0].size} selected={new Set(picked.keys())} onOpen={setPath} onToggle={toggle} />
              ))}
              {node && node.looseFilesSize > 0 && (
                <div className="flex h-7 items-center gap-2 px-2 text-[12px] text-ink-3">
                  <Eye className="size-3.5 opacity-0" />
                  <span className="flex-1">Arquivos menores</span>
                  <span className="tabular">{formatBytes(node.looseFilesSize)}</span>
                </div>
              )}
            </div>
            <div className="min-w-0 flex-1 rounded-xl glass p-1">
              {node && <Treemap node={node} selected={new Set(picked.keys())} onOpen={setPath} onToggle={toggle} />}
            </div>
          </div>
          <div className="px-8">
            <SelectionBar
              count={items.length}
              bytes={bytes}
              onReview={() => setReview(true)}
              extra={
                items.length > 0 && (
                  <Button variant="ghost" size="sm" icon={<FolderOpen className="size-3.5" />} onClick={() => api.revealInFinder(items[items.length - 1].path)}>
                    Mostrar no Finder
                  </Button>
                )
              }
            />
          </div>
        </>
      )}
      {review && r && (
        <ReviewDialog
          items={items.map((i) => ({ path: i.path, size: i.size, label: i.name }))}
          scanRoot={r.root}
          home={home}
          onClose={() => setReview(false)}
          onDone={(out) => {
            const gone = new Set(out.filter((o) => o.ok).map((o) => o.path));
            setPicked((prev) => new Map([...prev].filter(([p]) => !gone.has(p))));
            qc.invalidateQueries({ queryKey: ["node", r.id] });
          }}
        />
      )}
    </div>
  );
}
