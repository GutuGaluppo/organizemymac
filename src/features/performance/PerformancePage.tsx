import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { BatteryCharging, BatteryMedium, Clock, Cpu, HardDrive, MemoryStick, Power } from "lucide-react";
import { api, errorMessage } from "../../lib/ipc";
import { Button, Card, ErrorNote, PageHeader } from "../../components/ui";
import { Modal } from "../../components/ReviewDialog";
import { formatBytes, formatCount } from "../../lib/format";
import { AppIcon } from "../applications/shared";
import type { AppUsage, ProcessCategory } from "../../types";

const HISTORY = 60;

function duration(secs: number): string {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return d ? `${d} d ${h} h` : h ? `${h} h ${m} min` : `${m} min`;
}

function Sparkline({ values, max = 100 }: { values: number[]; max?: number }) {
  if (values.length < 2) return <div className="h-10" />;
  const w = 220;
  const h = 40;
  const pts = values.map((v, i) => `${(i / (HISTORY - 1)) * w},${h - (Math.min(v, max) / max) * (h - 2) - 1}`).join(" ");
  return (
    <svg viewBox={`0 0 ${w} ${h}`} className="h-10 w-full" preserveAspectRatio="none" aria-hidden>
      <polyline points={`0,${h} ${pts} ${((values.length - 1) / (HISTORY - 1)) * w},${h}`} fill="var(--accent-soft)" stroke="none" />
      <polyline points={pts} fill="none" stroke="var(--accent)" strokeWidth="1.5" vectorEffect="non-scaling-stroke" />
    </svg>
  );
}

function Meter({ value, tone }: { value: number; tone?: string }) {
  const color = tone ?? (value > 0.9 ? "bg-danger" : value > 0.75 ? "bg-review" : "bg-accent");
  return (
    <div className="h-2 overflow-hidden rounded-full bg-surface-3">
      <div className={`h-full rounded-full ${color} transition-[width] duration-500`} style={{ width: `${Math.max(1, Math.min(1, value) * 100)}%` }} />
    </div>
  );
}

function Tile({ icon, label, value, detail, children }: { icon: ReactNode; label: string; value: ReactNode; detail?: ReactNode; children?: ReactNode }) {
  return (
    <Card className="p-4">
      <div className="flex items-center gap-2 text-[12px] font-medium text-ink-3 [&_svg]:size-4">
        {icon} {label}
      </div>
      <div className="tabular mt-2 text-[22px] font-semibold">{value}</div>
      {detail && <div className="mt-0.5 text-[12px] text-ink-3">{detail}</div>}
      {children && <div className="mt-3">{children}</div>}
    </Card>
  );
}

const categories: { id: ProcessCategory; label: string; note: string }[] = [
  { id: "application", label: "Aplicativos", note: "Apps que você abriu, com os processos auxiliares de cada um somados." },
  { id: "background", label: "Em segundo plano", note: "Outros processos da sua conta: agentes, ajudantes e ferramentas de linha de comando." },
  { id: "system", label: "Sistema", note: "Processos do macOS e de outras contas. Não podem ser encerrados por aqui." },
];

export function PerformancePage() {
  const qc = useQueryClient();
  const { data: h, error } = useQuery({ queryKey: ["health"], queryFn: api.health, refetchInterval: 2000 });
  const { data: procs } = useQuery({ queryKey: ["processes"], queryFn: api.processList, refetchInterval: 5000 });
  const [cpuHistory, setCpuHistory] = useState<number[]>([]);
  const [tab, setTab] = useState<ProcessCategory>("application");
  const [quitting, setQuitting] = useState<AppUsage | null>(null);
  const [quitError, setQuitError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (h) setCpuHistory((prev) => [...prev, h.cpu].slice(-HISTORY));
  }, [h]);

  const rows: (AppUsage & { key: string; user?: string | null })[] = useMemo(() => {
    if (!procs) return [];
    if (tab === "application")
      return procs.apps
        .filter((a) => a.appPath && procs.processes.some((p) => p.appPath === a.appPath && p.category === "application"))
        .map((a) => ({ ...a, key: a.appPath ?? a.name }));
    return procs.processes
      .filter((p) => p.category === tab)
      .sort((a, b) => b.memory - a.memory)
      .slice(0, 150)
      .map((p) => ({ key: String(p.pid), name: p.name, appPath: null, memory: p.memory, cpu: p.cpu, processes: 1, canQuit: false, user: p.user }));
  }, [procs, tab]);

  const quit = async () => {
    if (!quitting?.appPath) return;
    setBusy(true);
    setQuitError(null);
    try {
      await api.quitApplication(quitting.appPath);
      setQuitting(null);
      setTimeout(() => qc.invalidateQueries({ queryKey: ["processes"] }), 1500);
    } catch (e) {
      setQuitError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const mem = h ? h.memoryUsed / h.memoryTotal : 0;
  const disk = h ? (h.diskTotal - h.diskFree) / Math.max(h.diskTotal, 1) : 0;

  return (
    <div className="h-full overflow-y-auto">
      <PageHeader title="Desempenho" subtitle="Atualiza a cada 2 segundos enquanto a janela está visível, e para quando ela está escondida." />
      <div className="space-y-5 px-8 pb-10">
        {error && <ErrorNote>{errorMessage(error)}</ErrorNote>}
        <div className="grid grid-cols-3 gap-4">
          <Tile icon={<Cpu />} label="CPU" value={h ? `${Math.round(h.cpu)}%` : "—"} detail={h && `${h.cores} núcleos · carga ${h.loadAverage[0].toFixed(2).replace(".", ",")}`}>
            <Sparkline values={cpuHistory} />
          </Tile>
          <Tile
            icon={<MemoryStick />}
            label="Memória"
            value={h ? formatBytes(h.memoryUsed) : "—"}
            detail={h && `de ${formatBytes(h.memoryTotal)} · ${formatBytes(h.memoryAvailable)} disponíveis`}
          >
            <Meter value={mem} />
            {h && h.swapTotal > 0 && (
              <div className="mt-2 text-[11.5px] text-ink-3">
                Swap: {formatBytes(h.swapUsed)} de {formatBytes(h.swapTotal)}
                {h.swapUsed > 2e9 ? " · o Mac está usando o disco como memória" : ""}
              </div>
            )}
          </Tile>
          <Tile icon={<HardDrive />} label="Disco" value={h ? formatBytes(h.diskFree) : "—"} detail={h && `livres de ${formatBytes(h.diskTotal)}`}>
            <Meter value={disk} />
          </Tile>
          <Tile
            icon={h?.battery?.charging ? <BatteryCharging /> : <BatteryMedium />}
            label="Bateria"
            value={h?.battery ? `${h.battery.percent}%` : "—"}
            detail={
              h?.battery
                ? [h.battery.charging ? "carregando" : h.battery.onAc ? "na tomada" : "na bateria", h.battery.minutesRemaining ? duration(h.battery.minutesRemaining * 60) + (h.battery.charging ? " para carregar" : " restantes") : null]
                    .filter(Boolean)
                    .join(" · ")
                : "Este Mac não tem bateria"
            }
          >
            {h?.battery && <Meter value={h.battery.percent / 100} tone={h.battery.percent < 15 ? "bg-danger" : "bg-safe"} />}
          </Tile>
          <Tile icon={<Clock />} label="Ligado há" value={h ? duration(h.uptime) : "—"} detail="desde o último reinício" />
          <Tile icon={<Power />} label="Apps abertos" value={procs ? formatCount(procs.apps.filter((a) => a.canQuit).length) : "—"} detail={procs && `${formatCount(procs.processes.length)} processos no total`} />
        </div>

        <div>
          <div className="mb-2 flex items-center gap-3">
            <h2 className="text-[13px] font-semibold">Processos por memória</h2>
            <div className="inline-flex rounded-lg border border-line bg-surface p-0.5">
              {categories.map((c) => (
                <button
                  key={c.id}
                  onClick={() => setTab(c.id)}
                  className={`h-7 rounded-md px-2.5 text-[12px] ${tab === c.id ? "bg-accent font-medium text-accent-ink" : "text-ink-2 hover:text-ink"}`}
                >
                  {c.label}
                </button>
              ))}
            </div>
          </div>
          <p className="mb-2 text-[12px] text-ink-3">{categories.find((c) => c.id === tab)?.note}</p>
          <Card className="divide-y divide-line">
            {rows.map((r) => (
              <div key={r.key} className="group grid grid-cols-[28px_minmax(0,1fr)_70px_90px_80px] items-center gap-3 px-4 py-1.5 text-[12.5px]">
                {r.appPath ? <AppIcon path={r.appPath} size={22} /> : <span className="size-[22px]" />}
                <div className="min-w-0 truncate">
                  {r.name}
                  {r.processes > 1 && <span className="text-ink-3"> · {r.processes} processos</span>}
                  {r.user && tab === "system" && <span className="text-ink-3"> · {r.user}</span>}
                </div>
                <span className="tabular text-right text-ink-2">{r.cpu.toFixed(1).replace(".", ",")}%</span>
                <span className="tabular text-right">{formatBytes(r.memory)}</span>
                <div className="text-right">
                  {r.canQuit && (
                    <Button size="sm" variant="ghost" className="opacity-0 group-hover:opacity-100" onClick={() => setQuitting(r)}>
                      Encerrar…
                    </Button>
                  )}
                </div>
              </div>
            ))}
          </Card>
        </div>
      </div>
      {quitting && (
        <Modal onClose={busy ? undefined : () => setQuitting(null)} width={440}>
          <div className="p-6">
            <h2 className="text-[16px] font-semibold">Encerrar {quitting.name}?</h2>
            <p className="mt-1.5 text-ink-2">É o mesmo que escolher Encerrar no menu do app: ele pode pedir para você salvar documentos abertos.</p>
            {quitError && (
              <div className="mt-3">
                <ErrorNote>{quitError}</ErrorNote>
              </div>
            )}
          </div>
          <div className="flex justify-end gap-2 border-t border-line px-6 py-3">
            <Button onClick={() => setQuitting(null)} disabled={busy}>
              Cancelar
            </Button>
            <Button variant="primary" busy={busy} onClick={quit}>
              Encerrar
            </Button>
          </div>
        </Modal>
      )}
    </div>
  );
}
