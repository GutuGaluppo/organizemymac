import type { ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { Activity, Sparkles, AppWindow, ChevronRight, Copy, Map as MapIcon, Download, FileSearch, FolderSearch, HardDrive, Trash2, Usb } from "lucide-react";
import { api } from "../../lib/ipc";
import { Card, PageHeader } from "../../components/ui";
import { formatBytes, formatDateTime, shortPath } from "../../lib/format";
import { useNav, type Section } from "../../stores/nav";
import { useHome } from "../scanner/ScannerPage";
import type { Volume } from "../../types";

function VolumeCard({ v }: { v: Volume }) {
  const used = v.total - v.free;
  const pct = v.total ? used / v.total : 0;
  const tone = pct > 0.9 ? "bg-danger" : pct > 0.75 ? "bg-review" : "bg-accent";
  return (
    <Card className="p-5">
      <div className="flex items-center gap-3">
        <div className="grid size-10 place-items-center rounded-xl bg-surface-2 text-ink-2">
          {v.isRoot ? <HardDrive className="size-5" /> : <Usb className="size-5" />}
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate font-semibold">{v.name}</div>
          <div className="text-[11.5px] text-ink-3">
            {v.fileSystem.toUpperCase()}
            {v.readOnly ? " · somente leitura" : ""}
            {!v.local ? " · rede" : ""}
          </div>
        </div>
        <div className="text-right">
          <div className="tabular text-[18px] font-semibold">{formatBytes(v.free)}</div>
          <div className="text-[11.5px] text-ink-3">livres de {formatBytes(v.total)}</div>
        </div>
      </div>
      <div className="mt-4 h-2.5 overflow-hidden rounded-full bg-surface-3">
        <div className={`h-full rounded-full ${tone}`} style={{ width: `${Math.max(1, pct * 100)}%` }} />
      </div>
      <div className="mt-1.5 flex justify-between text-[11.5px] text-ink-3">
        <span>{formatBytes(used)} em uso</span>
        <span>{Math.round(pct * 100)}%</span>
      </div>
    </Card>
  );
}

function Shortcut({ icon, title, text, to }: { icon: ReactNode; title: string; text: string; to: Section }) {
  const go = useNav((s) => s.go);
  return (
    <button onClick={() => go(to)} className="group flex items-center gap-3 rounded-xl border border-line bg-surface p-4 text-left transition hover:border-accent/50">
      <span className="grid size-9 shrink-0 place-items-center rounded-lg bg-accent-soft text-accent [&_svg]:size-[18px]">{icon}</span>
      <span className="min-w-0 flex-1">
        <span className="block font-medium">{title}</span>
        <span className="block text-[12px] text-ink-3">{text}</span>
      </span>
      <ChevronRight className="size-4 text-ink-3 transition group-hover:translate-x-0.5" />
    </button>
  );
}

const moduleLabel: Record<string, string> = { scanner: "Scanner", largeFiles: "Arquivos grandes", downloads: "Downloads", duplicates: "Duplicados" };

export function OverviewPage() {
  const home = useHome();
  const { data: volumes } = useQuery({ queryKey: ["volumes"], queryFn: api.diskOverview, refetchInterval: 30_000 });
  const { data: scans } = useQuery({ queryKey: ["recentScans"], queryFn: () => api.recentScans(undefined, 6) });
  const { data: metrics } = useQuery({ queryKey: ["metrics"], queryFn: api.localMetrics });

  return (
    <div className="h-full overflow-y-auto">
      <PageHeader
        title="Visão geral"
        subtitle={
          metrics?.bytes_reclaimed_total ? `Você já liberou ${formatBytes(metrics.bytes_reclaimed_total)} com o OrganizeMyMac.` : "Espaço nos discos e atalhos para começar."
        }
      />
      <div className="space-y-6 px-8 pb-10">
        <div className="grid grid-cols-2 gap-4">{volumes?.map((v) => <VolumeCard key={v.mountPoint} v={v} />)}</div>
        <p className="-mt-3 text-[11.5px] text-ink-3">
          O espaço livre não inclui o espaço "purgeable" que o macOS libera sozinho quando precisa (snapshots locais, caches do iCloud), por
          isso o Finder pode mostrar um número maior.
        </p>

        <div>
          <h2 className="mb-2 text-[13px] font-semibold">Por onde começar</h2>
          <div className="grid grid-cols-2 gap-3">
            <Shortcut to="smartCare" icon={<Sparkles />} title="Cuidado inteligente" text="Uma análise completa com recomendações para revisar." />
            <Shortcut to="scanner" icon={<FolderSearch />} title="O que ocupa espaço" text="Analise uma pasta ou o disco inteiro." />
            <Shortcut to="spaceMap" icon={<MapIcon />} title="Mapa de espaço" text="Veja as pastas em proporção e navegue por elas." />
            <Shortcut to="largeFiles" icon={<FileSearch />} title="Arquivos grandes e antigos" text="Os maiores e os esquecidos." />
            <Shortcut to="downloads" icon={<Download />} title="Downloads" text="Instaladores e arquivos que sobraram." />
            <Shortcut to="duplicates" icon={<Copy />} title="Duplicados" text="Cópias idênticas espalhadas pelo Mac." />
            <Shortcut to="apps" icon={<AppWindow />} title="Aplicativos" text="Desinstale apps junto com os arquivos que eles deixam." />
            <Shortcut to="performance" icon={<Activity />} title="Desempenho" text="CPU, memória, bateria e os apps que mais consomem." />
            <Shortcut to="trash" icon={<Trash2 />} title="Lixeira" text="O que já foi removido ainda ocupa espaço." />
          </div>
        </div>

        {!!scans?.length && (
          <div>
            <h2 className="mb-2 text-[13px] font-semibold">Últimas análises</h2>
            <Card className="divide-y divide-line">
              {scans.map((s) => (
                <div key={s.id} className="grid grid-cols-[130px_120px_minmax(0,1fr)_90px] gap-3 px-4 py-2 text-[12.5px]">
                  <span className="text-ink-3">{formatDateTime(s.startedAt)}</span>
                  <span>{moduleLabel[s.module] ?? s.module}</span>
                  <span className="truncate text-ink-2">{shortPath(s.root, home)}</span>
                  <span className="tabular text-right">{formatBytes(s.bytesScanned)}</span>
                </div>
              ))}
            </Card>
          </div>
        )}
      </div>
    </div>
  );
}
