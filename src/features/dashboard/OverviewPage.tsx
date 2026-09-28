import { useQuery } from "@tanstack/react-query";
import { ChevronRight, HardDrive, Usb } from "lucide-react";
import { api } from "../../lib/ipc";
import { Button, Card, PageHeader } from "../../components/ui";
import { formatBytes, formatDateTime, shortPath } from "../../lib/format";
import { useNav } from "../../stores/nav";
import { useJob, useJobs } from "../../stores/jobs";
import { useHome } from "../scanner/ScannerPage";
import { modules, smartCare, tools, type Module } from "../../app/modules";
import type { ScanRecord, Volume } from "../../types";

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

function SmartCareHero() {
  const go = useNav((s) => s.go);
  const { status } = useJob(smartCare.sections[0].job!);
  const action = status === "running" ? "Ver progresso" : status === "done" ? "Ver recomendações" : "Analisar";
  return (
    <div data-tint={smartCare.id}>
      <Card className="flex items-center gap-5 p-5">
        <img src={smartCare.icon} alt="" className="size-20 shrink-0 object-contain" draggable={false} />
        <div className="min-w-0 flex-1">
          <h2 className="text-[17px] font-semibold">Cuidado inteligente</h2>
          <p className="mt-1 text-[13px] text-ink-2">
            Uma análise completa com recomendações para você revisar.
            <br />
            Nada é removido sem a sua confirmação.
          </p>
        </div>
        <Button variant="primary" busy={status === "running"} onClick={() => go("smartCare")} className="h-9! rounded-full! px-5!">
          {action}
        </Button>
      </Card>
    </div>
  );
}

function ModuleCard({ module, lastScan }: { module: Module; lastScan?: ScanRecord }) {
  const go = useNav((s) => s.go);
  const running = useJobs((s) => module.sections.some((x) => x.job && s.jobs[x.job]?.status === "running"));
  const status = running ? "Analisando…" : lastScan ? `Analisado em ${formatDateTime(lastScan.startedAt)}` : "Ainda não analisado";
  return (
    <button
      data-tint={module.id}
      onClick={() => go(module.sections[0].id)}
      className="group flex min-w-0 flex-col items-start rounded-xl border border-line bg-surface p-4 text-left transition hover:border-accent/50"
    >
      <img src={module.icon} alt="" className="size-12 object-contain" draggable={false} />
      <span className="mt-3 font-semibold">{module.label}</span>
      <span className="mt-0.5 flex w-full items-center gap-1 text-[12px] text-ink-3">
        <span className="flex-1 truncate">{status}</span>
        <ChevronRight className="size-3.5 shrink-0 transition group-hover:translate-x-0.5" />
      </span>
    </button>
  );
}

const allModules = [smartCare, ...modules, ...tools];
const moduleLabel = (id: string) => allModules.find((m) => m.sections.some((s) => s.id === id))?.label ?? id;

export function OverviewPage() {
  const home = useHome();
  const go = useNav((s) => s.go);
  const { data: volumes } = useQuery({
    queryKey: ["volumes"],
    queryFn: api.diskOverview,
    refetchInterval: 30_000,
  });
  const { data: scans } = useQuery({
    queryKey: ["recentScans"],
    queryFn: () => api.recentScans(undefined, 20),
  });
  const { data: metrics } = useQuery({
    queryKey: ["metrics"],
    queryFn: api.localMetrics,
  });

  // Scans come newest first, so the first match is the latest one for the module.
  const lastScan = (m: Module) => scans?.find((s) => m.sections.some((x) => x.id === s.module));

  return (
    <div className="h-full overflow-y-auto">
      <PageHeader
        title="Início"
        subtitle={
          metrics?.bytes_reclaimed_total
            ? `Você já liberou ${formatBytes(metrics.bytes_reclaimed_total)} com o OrganizeMyMac.`
            : "Espaço nos discos e atalhos para começar."
        }
      />
      <div className="space-y-6 px-8 pb-10">
        <div className="grid grid-cols-2 gap-4">
          {volumes?.map((v) => (
            <VolumeCard key={v.mountPoint} v={v} />
          ))}
        </div>
        <p className="-mt-3 text-[11.5px] text-ink-3">
          O espaço livre não inclui o espaço "purgeable" que o macOS libera sozinho quando precisa (snapshots locais, caches do iCloud), por isso o
          Finder pode mostrar um número maior.
        </p>

        <SmartCareHero />

        <div className="grid grid-cols-[repeat(auto-fit,minmax(140px,1fr))] gap-3">
          {modules.map((m) => (
            <ModuleCard key={m.id} module={m} lastScan={lastScan(m)} />
          ))}
        </div>

        <div className="flex gap-2">
          {tools.map((t) => (
            <button
              key={t.id}
              data-tint={t.id}
              onClick={() => go(t.sections[0].id)}
              className="flex h-8 items-center gap-2 rounded-full border border-line bg-surface pr-3.5 pl-1.5 text-[12.5px] transition hover:border-accent/50"
            >
              <img src={t.icon} alt="" className="size-[22px] object-contain" draggable={false} />
              {t.label}
            </button>
          ))}
        </div>

        <div>
          <h2 className="mb-2 text-[13px] font-semibold">Últimas análises</h2>
          <Card className="divide-y divide-line">
            {scans?.length ? (
              scans.slice(0, 6).map((s) => (
                <div key={s.id} className="grid grid-cols-[130px_150px_minmax(0,1fr)_90px] gap-3 px-4 py-2 text-[12.5px]">
                  <span className="text-ink-3">{formatDateTime(s.startedAt)}</span>
                  <span className="truncate">{moduleLabel(s.module)}</span>
                  <span className="truncate text-ink-2">{shortPath(s.root, home)}</span>
                  <span className="tabular text-right">{formatBytes(s.bytesScanned)}</span>
                </div>
              ))
            ) : (
              <div className="px-4 py-4 text-center text-[12.5px] text-ink-3">Nenhuma análise concluída ainda.</div>
            )}
          </Card>
        </div>
      </div>
    </div>
  );
}
