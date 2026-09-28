import { useEffect, useMemo, useState } from "react";
import { Download } from "lucide-react";
import { useJob, useJobs } from "../../stores/jobs";
import { Badge, Button, EmptyState, ErrorNote, PageHeader } from "../../components/ui";
import { FileTable, useSelection, type Row } from "../../components/FileTable";
import { ReviewDialog, SelectionBar } from "../../components/ReviewDialog";
import { ScanProgressCard, WarningsNote, useHome } from "../scanner/ScannerPage";
import { formatBytes, formatCount } from "../../lib/format";
import type { DownloadGroup, DownloadItem, DownloadsResult } from "../../types";

const MODULE = "downloads";

const groups: { id: DownloadGroup; title: string; description: string }[] = [
  { id: "oldInstaller", title: "Instaladores antigos", description: "Imagens de disco e instaladores baixados há mais de 30 dias. O app provavelmente já foi instalado." },
  { id: "largeFile", title: "Arquivos grandes", description: "Itens com mais de 500 MB." },
  { id: "archive", title: "Arquivos compactados", description: "ZIP e similares. Os que já foram extraídos ao lado aparecem como seguros." },
  { id: "installer", title: "Instaladores recentes", description: "Baixados nos últimos 30 dias: talvez ainda não tenham sido usados." },
  { id: "screenshot", title: "Capturas de tela", description: "Imagens salvas pelo macOS com o nome padrão de captura." },
  { id: "oldFile", title: "Arquivos antigos", description: "Chegaram há mais de 6 meses." },
  { id: "recent", title: "Recentes", description: "O resto, baixado nos últimos meses." },
];

function note(item: DownloadItem): string | undefined {
  const age = item.ageDays != null ? (item.ageDays === 0 ? "hoje" : `há ${formatCount(item.ageDays)} ${item.ageDays === 1 ? "dia" : "dias"}`) : null;
  if (item.group === "archive" && item.extracted) return "Já extraído: a pasta com o conteúdo está ao lado";
  return age ? `Chegou ${age}` : undefined;
}

export function DownloadsPage() {
  const home = useHome();
  const job = useJob<DownloadsResult>(MODULE);
  const start = useJobs((s) => s.start);
  const update = useJobs((s) => s.update);
  const { selected, setSelected, onSelect } = useSelection();
  const [review, setReview] = useState(false);
  const r = job.result;
  const running = job.status === "running";

  // Pre-select what the classifier marked (old installers) when a new result arrives.
  useEffect(() => {
    if (r) setSelected(new Set(r.items.filter((i) => i.selected).map((i) => i.path)));
  }, [r?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const items = r?.items ?? [];
  const byPath = useMemo(() => new Map(items.map((i) => [i.path, i])), [items]);
  const chosen = useMemo(() => items.filter((i) => selected.has(i.path)), [items, selected]);
  const chosenBytes = chosen.reduce((s, i) => s + i.sizeLogical, 0);

  const removeRows = (paths: string[]) => {
    const gone = new Set(paths);
    update<DownloadsResult>(MODULE, (res) => {
      const left = res.items.filter((i) => !gone.has(i.path));
      return { ...res, items: left, totalBytes: left.reduce((s, i) => s + i.sizeLogical, 0) };
    });
    setSelected((s) => new Set([...s].filter((p) => !gone.has(p))));
  };

  return (
    <div className="h-full overflow-y-auto">
      <PageHeader
        title="Downloads"
        subtitle="Instaladores, arquivos compactados e capturas que ficaram para trás na pasta Downloads, do mais seguro de remover para o que merece uma olhada."
        actions={
          <Button variant={r ? "secondary" : "primary"} busy={running} onClick={() => start(MODULE, "start_downloads_scan", {})}>
            {r ? "Analisar de novo" : "Analisar Downloads"}
          </Button>
        }
      />
      <div className="space-y-5 px-8 pb-4">
        {job.status === "failed" && <ErrorNote>{job.error}</ErrorNote>}
        {running && <ScanProgressCard module={MODULE} label="Analisando Downloads" />}
        {job.status === "idle" && (
          <EmptyState icon={<Download className="size-6" />} title="Organize a pasta Downloads">
            Nada é removido na análise. Depois, você escolhe o que vai para a Lixeira.
          </EmptyState>
        )}
        {r && !running && (
          <>
            <div className="text-[13px] text-ink-2">
              <span className="font-semibold text-ink">{formatCount(items.length)}</span> itens ·{" "}
              <span className="tabular font-semibold text-ink">{formatBytes(r.totalBytes)}</span> na pasta Downloads
            </div>
            <WarningsNote module={MODULE} />
            {items.length === 0 && <EmptyState icon={<Download className="size-6" />} title="A pasta Downloads está vazia" />}
            {groups.map((g) => {
              const rows: Row[] = items
                .filter((i) => i.group === g.id)
                .map((i) => ({ ...i, note: note(i) }));
              if (!rows.length) return null;
              const size = rows.reduce((s, i) => s + i.sizeLogical, 0);
              const safe = rows.every((row) => byPath.get(row.path)?.confidence === "safe");
              return (
                <section key={g.id}>
                  <div className="mb-2 flex items-baseline gap-2">
                    <h2 className="text-[13.5px] font-semibold">{g.title}</h2>
                    {safe && <Badge tone="safe">Seguro</Badge>}
                    <span className="text-[12px] text-ink-3">
                      {formatCount(rows.length)} · {formatBytes(size)}
                    </span>
                  </div>
                  <p className="-mt-1 mb-2 text-[12px] text-ink-3">{g.description}</p>
                  <FileTable rows={rows} selected={selected} onSelect={onSelect} onIgnored={(p) => removeRows([p])} home={home} />
                </section>
              );
            })}
            {items.length > 0 && <SelectionBar count={chosen.length} bytes={chosenBytes} onReview={() => setReview(true)} />}
          </>
        )}
      </div>
      {review && (
        <ReviewDialog
          items={chosen.map((i) => ({ path: i.path, size: i.sizeLogical, label: i.name, confidence: i.confidence }))}
          scanRoot={r?.folder}
          home={home}
          onClose={() => setReview(false)}
          onDone={(out) => removeRows(out.filter((o) => o.ok).map((o) => o.path))}
        />
      )}
    </div>
  );
}
