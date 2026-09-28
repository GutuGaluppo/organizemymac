import { useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FileSearch, FolderOpen, Search } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { api } from "../../lib/ipc";
import { useJob, useJobs } from "../../stores/jobs";
import { Button, EmptyState, ErrorNote, PageHeader } from "../../components/ui";
import { FileTable, useSelection } from "../../components/FileTable";
import { ReviewDialog, SelectionBar } from "../../components/ReviewDialog";
import { ScanProgressCard, WarningsNote, useHome } from "../scanner/ScannerPage";
import { formatBytes, formatCount, shortPath } from "../../lib/format";
import type { FileCategory, FileFilter, FindResult } from "../../types";

const MODULE = "largeFiles";

const sizePresets = [
  { label: "> 100 MB", value: 100e6 },
  { label: "> 500 MB", value: 500e6 },
  { label: "> 1 GB", value: 1e9 },
];
const agePresets = [
  { label: "Qualquer data", value: null },
  { label: "> 6 meses", value: 182 },
  { label: "> 1 ano", value: 365 },
  { label: "> 2 anos", value: 730 },
];
const typePresets: { label: string; value: FileCategory[] }[] = [
  { label: "Todos os tipos", value: [] },
  { label: "Vídeos", value: ["video"] },
  { label: "Imagens", value: ["image"] },
  { label: "Áudio", value: ["audio"] },
  { label: "Documentos", value: ["document"] },
  { label: "Compactados", value: ["archive"] },
  { label: "Imagens de disco e instaladores", value: ["diskImage", "installer"] },
];

function Chips<T>({ options, value, onChange }: { options: { label: string; value: T }[]; value: T; onChange: (v: T) => void }) {
  return (
    <div className="inline-flex rounded-lg border border-line bg-surface p-0.5">
      {options.map((o) => {
        const active = JSON.stringify(o.value) === JSON.stringify(value);
        return (
          <button
            key={o.label}
            onClick={() => onChange(o.value)}
            className={`h-7 rounded-md px-2.5 text-[12px] transition ${active ? "bg-accent text-accent-ink font-medium" : "text-ink-2 hover:text-ink"}`}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

export function LargeFilesPage() {
  const home = useHome();
  const job = useJob<FindResult>(MODULE);
  const start = useJobs((s) => s.start);
  const update = useJobs((s) => s.update);
  const { data: locations } = useQuery({ queryKey: ["locations"], queryFn: api.suggestedLocations });
  const [root, setRoot] = useState<string | null>(null);
  const [minSize, setMinSize] = useState(500e6);
  const [olderThan, setOlderThan] = useState<number | null>(null);
  const [categories, setCategories] = useState<FileCategory[]>([]);
  const [includeLibrary, setIncludeLibrary] = useState(false);
  const [review, setReview] = useState(false);
  const { selected, setSelected, onSelect } = useSelection();
  const r = job.result;
  const effectiveRoot = root ?? locations?.find((l) => l.id === "home")?.path ?? home ?? "";

  const run = () => {
    setSelected(new Set());
    const filter: FileFilter = {
      minSize,
      olderThanDays: olderThan,
      dateField: "modified",
      extensions: [],
      categories,
      excludeLibrary: !includeLibrary,
      includeHidden: false,
    };
    start(MODULE, "start_find_files", { root: effectiveRoot, filter });
  };
  const choose = async () => {
    const dir = await open({ directory: true, title: "Onde procurar" });
    if (typeof dir === "string") setRoot(dir);
  };
  const removeRows = (paths: string[]) => {
    const gone = new Set(paths);
    update<FindResult>(MODULE, (res) => ({
      ...res,
      matches: res.matches.filter((f) => !gone.has(f.path)),
      matched: res.matched - paths.length,
    }));
    setSelected((s) => new Set([...s].filter((p) => !gone.has(p))));
  };

  const files = r?.matches ?? [];
  const selectedFiles = useMemo(() => files.filter((f) => selected.has(f.path)), [files, selected]);
  const selectedBytes = selectedFiles.reduce((s, f) => s + f.sizeLogical, 0);
  const running = job.status === "running";

  return (
    <div className="h-full overflow-y-auto">
      <PageHeader
        title="Arquivos grandes e antigos"
        subtitle="Encontre os arquivos que mais ocupam espaço e os que você não abre há muito tempo. Arquivos dentro de apps e bibliotecas (Fotos, Música) não são listados separadamente."
      />
      <div className="px-8 pb-2">
        <div className="space-y-3 rounded-xl border border-line bg-surface p-4">
          <div className="flex flex-wrap items-center gap-3">
            <span className="w-16 text-[12px] text-ink-3">Onde</span>
            <button
              onClick={choose}
              disabled={running}
              className="flex h-8 min-w-0 max-w-md items-center gap-2 rounded-lg border border-line bg-surface-2 px-3 text-[12.5px] hover:border-accent/50"
            >
              <FolderOpen className="size-4 shrink-0 text-accent" />
              <span className="truncate">{effectiveRoot === home ? "Pasta pessoal (~)" : shortPath(effectiveRoot, home) || "…"}</span>
            </button>
            <label className="flex items-center gap-2 text-[12px] text-ink-2">
              <input type="checkbox" className="accent-[var(--accent)]" checked={includeLibrary} onChange={(e) => setIncludeLibrary(e.target.checked)} />
              Incluir ~/Library (dados de apps)
            </label>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <span className="w-16 text-[12px] text-ink-3">Tamanho</span>
            <Chips options={sizePresets} value={minSize} onChange={setMinSize} />
            <span className="ml-3 text-[12px] text-ink-3">Sem modificação há</span>
            <Chips options={agePresets} value={olderThan} onChange={setOlderThan} />
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <span className="w-16 text-[12px] text-ink-3">Tipo</span>
            <Chips options={typePresets} value={categories} onChange={setCategories} />
            <div className="flex-1" />
            <Button variant="primary" icon={<Search className="size-3.5" />} onClick={run} disabled={running || !effectiveRoot}>
              Buscar
            </Button>
          </div>
        </div>
      </div>
      <div className="space-y-4 px-8 pt-3 pb-4">
        {job.status === "failed" && <ErrorNote>{job.error}</ErrorNote>}
        {running && <ScanProgressCard module={MODULE} label="Procurando arquivos" />}
        {job.status === "idle" && (
          <EmptyState icon={<FileSearch className="size-6" />} title="Defina os filtros e busque">
            A data usada é a da última modificação: o macOS não registra de forma confiável quando um arquivo foi aberto.
          </EmptyState>
        )}
        {r && !running && (
          <>
            <div className="flex items-baseline justify-between">
              <div className="text-[13px] text-ink-2">
                <span className="font-semibold text-ink">{formatCount(r.matched)}</span> {r.matched === 1 ? "arquivo" : "arquivos"} ·{" "}
                <span className="tabular font-semibold text-ink">{formatBytes(r.matchedBytes)}</span>
                {r.truncated && <span className="text-ink-3"> · mostrando os {formatCount(files.length)} maiores</span>}
                {r.cancelled && <span className="text-ink-3"> · busca cancelada, resultado parcial</span>}
              </div>
            </div>
            <WarningsNote module={MODULE} />
            {files.length ? (
              <FileTable rows={files} selected={selected} onSelect={onSelect} onIgnored={(p) => removeRows([p])} home={home} />
            ) : (
              <EmptyState icon={<FileSearch className="size-6" />} title="Nada encontrado com esses filtros" />
            )}
          </>
        )}
      </div>
      {r && files.length > 0 && (
        <div className="px-8">
          <SelectionBar count={selectedFiles.length} bytes={selectedBytes} onReview={() => setReview(true)} />
        </div>
      )}
      {review && (
        <ReviewDialog
          items={selectedFiles.map((f) => ({ path: f.path, size: f.sizeLogical, label: f.name }))}
          scanRoot={r?.root}
          home={home}
          onClose={() => setReview(false)}
          onDone={(out) => removeRows(out.filter((o) => o.ok).map((o) => o.path))}
        />
      )}
    </div>
  );
}
