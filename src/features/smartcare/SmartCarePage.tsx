import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, AppWindow, Check, ChevronDown, ChevronRight, Copy, Download, FileSearch, Loader2, PackageX, Sparkles, Trash2 } from "lucide-react";
import { api, errorMessage } from "../../lib/ipc";
import { useJob, useJobs } from "../../stores/jobs";
import { useNav } from "../../stores/nav";
import { Badge, Button, Card, ErrorNote, PageHeader } from "../../components/ui";
import { Modal } from "../../components/ReviewDialog";
import { useHome } from "../scanner/ScannerPage";
import { ConfidenceBadge, kindLabel } from "../applications/shared";
import { formatAge, formatBytes, formatCount, shortPath } from "../../lib/format";
import type { CarePlan, OperationOutcome, SmartCareReport } from "../../types";

const MODULE = "smartCare";

const stages = [
  { id: "trash", label: "Lixeira" },
  { id: "downloads", label: "Downloads" },
  { id: "home", label: "Pasta pessoal" },
  { id: "duplicates", label: "Duplicados" },
  { id: "apps", label: "Aplicativos" },
  { id: "leftovers", label: "Restos de apps" },
];

type Row = { key: string; label: ReactNode; detail?: ReactNode; size: number; badge?: ReactNode; locked?: boolean };

function Section({
  icon,
  title,
  note,
  rows,
  selected,
  onToggle,
  onAll,
  footer,
}: {
  icon: ReactNode;
  title: string;
  note?: ReactNode;
  rows: Row[];
  selected: Set<string>;
  onToggle: (key: string) => void;
  onAll?: (value: boolean) => void;
  footer?: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const chosen = rows.filter((r) => selected.has(r.key));
  const bytes = chosen.reduce((s, r) => s + r.size, 0);
  if (!rows.length) return null;
  return (
    <Card className="overflow-hidden">
      <button className="flex w-full items-center gap-3 px-4 py-3 text-left hover:bg-surface-2" onClick={() => setOpen(!open)}>
        <span className="grid size-8 place-items-center rounded-lg bg-accent-soft text-accent [&_svg]:size-4">{icon}</span>
        <span className="min-w-0 flex-1">
          <span className="block font-medium">{title}</span>
          <span className="block text-[12px] text-ink-3">
            {formatCount(chosen.length)} de {formatCount(rows.length)} selecionados · {formatBytes(bytes)}
          </span>
        </span>
        {open ? <ChevronDown className="size-4 text-ink-3" /> : <ChevronRight className="size-4 text-ink-3" />}
      </button>
      {open && (
        <div className="border-t border-line">
          {note && <div className="bg-surface-2 px-4 py-2 text-[12px] text-ink-2">{note}</div>}
          {onAll && (
            <div className="flex gap-2 px-4 pt-2">
              <Button size="sm" variant="ghost" onClick={() => onAll(true)}>
                Marcar todos
              </Button>
              <Button size="sm" variant="ghost" onClick={() => onAll(false)}>
                Desmarcar
              </Button>
            </div>
          )}
          <div className="max-h-80 overflow-y-auto px-2 py-1">
            {rows.map((r) => (
              <label key={r.key} className={`grid grid-cols-[18px_minmax(0,1fr)_auto_80px] items-center gap-3 rounded-md px-2 py-1.5 ${r.locked ? "opacity-60" : "cursor-pointer hover:bg-surface-2"}`}>
                <input type="checkbox" className="size-3.5 accent-[var(--accent)]" disabled={r.locked} checked={selected.has(r.key)} onChange={() => onToggle(r.key)} />
                <div className="min-w-0">
                  <div className="truncate text-[12.5px]">{r.label}</div>
                  {r.detail && <div className="truncate text-[11px] text-ink-3">{r.detail}</div>}
                </div>
                <div>{r.badge}</div>
                <span className="tabular text-right text-[12.5px]">{formatBytes(r.size)}</span>
              </label>
            ))}
          </div>
          {footer && <div className="border-t border-line px-4 py-2">{footer}</div>}
        </div>
      )}
    </Card>
  );
}

function Summary({ icon, label, value, hint }: { icon: ReactNode; label: string; value: string; hint?: string }) {
  return (
    <div className="rounded-xl border border-line bg-surface p-4">
      <div className="flex items-center gap-2 text-[12px] text-ink-3 [&_svg]:size-4">
        {icon} {label}
      </div>
      <div className="tabular mt-1.5 text-[20px] font-semibold">{value}</div>
      {hint && <div className="text-[11.5px] text-ink-3">{hint}</div>}
    </div>
  );
}

export function SmartCarePage() {
  const home = useHome();
  const qc = useQueryClient();
  const go = useNav((s) => s.go);
  const job = useJob<SmartCareReport | null>(MODULE);
  const start = useJobs((s) => s.start);
  const reset = useJobs((s) => s.reset);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [emptyTrash, setEmptyTrash] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [outcomes, setOutcomes] = useState<OperationOutcome[] | null>(null);
  const r = job.result ?? undefined;
  const running = job.status === "running";

  // Defaults: old installers, the smart duplicate selection and safe leftovers. Never the Trash,
  // large files or anything ambiguous.
  useEffect(() => {
    if (!r) return;
    setEmptyTrash(false);
    setSelected(
      new Set([
        ...r.downloads.filter((d) => d.selected).map((d) => `dl:${d.path}`),
        ...r.duplicates.flatMap((g) => g.files.filter((f) => f.selected).map((f) => `dup:${f.path}`)),
        ...r.leftovers.flatMap((g) => g.items.filter((i) => i.selected).map((i) => `lo:${i.path}`)),
      ]),
    );
  }, [r]);

  const toggle = (key: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  const setMany = (keys: string[], value: boolean) =>
    setSelected((prev) => {
      const next = new Set(prev);
      for (const k of keys) {
        if (value) next.add(k);
        else next.delete(k);
      }
      return next;
    });

  const rows = useMemo(() => {
    if (!r) return null;
    const downloads: Row[] = r.downloads.map((d) => ({
      key: `dl:${d.path}`,
      label: d.name,
      detail: d.ageDays != null ? `Chegou há ${formatCount(d.ageDays)} dias` : undefined,
      size: d.sizeLogical,
      badge: <ConfidenceBadge confidence={d.confidence} />,
    }));
    const duplicates: Row[] = r.duplicates.flatMap((g) =>
      g.files.map((f) => {
        const others = g.files.filter((x) => x.path !== f.path);
        const lastKept = !selected.has(`dup:${f.path}`) && others.every((x) => selected.has(`dup:${x.path}`));
        return { key: `dup:${f.path}`, label: shortPath(f.path, home), detail: `${g.files.length} cópias idênticas`, size: f.sizeLogical, badge: selected.has(`dup:${f.path}`) ? undefined : <Badge tone="safe">Manter</Badge>, locked: lastKept };
      }),
    );
    const largeOld: Row[] = r.largeOld.map((f) => ({ key: `lg:${f.path}`, label: shortPath(f.path, home), detail: `Modificado ${formatAge(f.modifiedAt)}`, size: f.sizeLogical, badge: <ConfidenceBadge confidence="review" /> }));
    const leftovers: Row[] = r.leftovers.flatMap((g) =>
      g.items.map((i) => ({ key: `lo:${i.path}`, label: g.bundleId, detail: kindLabel[i.kind], size: i.size, badge: <ConfidenceBadge confidence={i.confidence} /> })),
    );
    return { downloads, duplicates, largeOld, leftovers };
  }, [r, selected, home]);

  const all = rows ? [...rows.downloads, ...rows.duplicates, ...rows.largeOld, ...rows.leftovers] : [];
  const chosenBytes = all.filter((x) => selected.has(x.key)).reduce((s, x) => s + x.size, 0) + (emptyTrash && r ? r.trash.bytes : 0);
  const chosenCount = all.filter((x) => selected.has(x.key)).length;
  const reclaimable = r
    ? r.downloads.filter((d) => d.selected).reduce((s, d) => s + d.sizeLogical, 0) +
      r.duplicates.reduce((s, g) => s + g.wasted, 0) +
      r.leftovers.reduce((s, g) => s + g.items.filter((i) => i.selected).reduce((a, i) => a + i.size, 0), 0) +
      (r.trash.readable ? r.trash.bytes : 0)
    : 0;

  const plan = (): CarePlan => {
    const has = (k: string) => selected.has(k);
    return {
      emptyTrash: emptyTrash && r ? r.trash.items.map((i) => i.path) : [],
      files: [
        ...(r?.downloads ?? []).filter((d) => has(`dl:${d.path}`)).map((d) => ({ path: d.path, size: d.sizeLogical })),
        ...(r?.largeOld ?? []).filter((f) => has(`lg:${f.path}`)).map((f) => ({ path: f.path, size: f.sizeLogical })),
      ],
      duplicates: (r?.duplicates ?? [])
        .map((g) => ({
          all: g.files.map((f) => f.path),
          remove: g.files.filter((f) => has(`dup:${f.path}`)).map((f) => ({ path: f.path, size: f.sizeLogical, modifiedAt: f.modifiedAt ?? null })),
        }))
        .filter((g) => g.remove.length),
      leftovers: (r?.leftovers ?? []).flatMap((g) => g.items.filter((i) => has(`lo:${i.path}`)).map((i) => ({ path: i.path, size: i.size }))),
    };
  };

  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      const out = await api.runSmartCare(plan());
      setOutcomes(out);
      setConfirm(false);
      for (const k of ["operations", "metrics", "volumes", "trash"]) qc.invalidateQueries({ queryKey: [k] });
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const stageIndex = stages.findIndex((s) => s.id === job.stage?.stage);

  return (
    <div className="h-full overflow-y-auto">
      <PageHeader
        title="Cuidado inteligente"
        subtitle="Uma análise que junta Lixeira, Downloads, duplicados, arquivos esquecidos, apps sem uso e restos de apps. Você revisa tudo antes de qualquer remoção."
        actions={
          r &&
          !running && (
            <Button variant="ghost" onClick={() => reset(MODULE)}>
              Nova análise
            </Button>
          )
        }
      />
      <div className="space-y-5 px-8 pb-4">
        {job.status === "failed" && <ErrorNote>{job.error}</ErrorNote>}
        {error && <ErrorNote>{error}</ErrorNote>}

        {(job.status === "idle" || job.status === "cancelled" || (job.status === "done" && !r)) && (
          <Card className="flex flex-col items-center px-8 py-12 text-center">
            <div className="grid size-16 place-items-center rounded-2xl bg-accent-soft text-accent">
              <Sparkles className="size-7" />
            </div>
            <h2 className="mt-4 text-[17px] font-semibold">Analise o Mac de uma vez</h2>
            <p className="mt-1.5 max-w-md text-ink-2">Leva de alguns segundos a poucos minutos. Nada é alterado na análise.</p>
            <Button variant="primary" className="mt-5" onClick={() => start(MODULE, "start_smart_care", {})}>
              Analisar
            </Button>
          </Card>
        )}

        {running && (
          <Card className="p-6">
            <div className="mb-4 flex items-center justify-between">
              <div className="text-[15px] font-semibold">Analisando</div>
              <Button size="sm" onClick={() => useJobs.getState().cancel(MODULE)}>
                Cancelar
              </Button>
            </div>
            <div className="grid grid-cols-3 gap-2">
              {stages.map((s, i) => (
                <div key={s.id} className={`flex items-center gap-2 rounded-lg px-3 py-2 text-[12.5px] ${i === stageIndex ? "bg-accent-soft text-accent" : i < stageIndex ? "text-ink-2" : "text-ink-3"}`}>
                  {i < stageIndex ? <Check className="size-4 text-safe" /> : i === stageIndex ? <Loader2 className="size-4 animate-spin" /> : <span className="size-4" />}
                  {s.label}
                </div>
              ))}
            </div>
            {job.progress && stageIndex === 2 && <div className="mt-3 text-[11.5px] text-ink-3">{formatCount(job.progress.files)} arquivos lidos</div>}
          </Card>
        )}

        {r && rows && !running && (
          <>
            <div className="grid grid-cols-5 gap-3">
              <Summary icon={<Sparkles />} label="Pode liberar" value={formatBytes(reclaimable)} hint={`em ${(r.durationMs / 1000).toFixed(0)} s de análise`} />
              <Summary icon={<Download />} label="Downloads" value={formatBytes(r.downloads.filter((d) => d.selected).reduce((s, d) => s + d.sizeLogical, 0))} hint="instaladores antigos" />
              <Summary icon={<Copy />} label="Duplicados" value={formatBytes(r.duplicates.reduce((s, g) => s + g.wasted, 0))} hint={`${formatCount(r.duplicates.length)} grupos`} />
              <Summary icon={<Trash2 />} label="Lixeira" value={r.trash.readable ? formatBytes(r.trash.bytes) : "—"} hint={r.trash.readable ? undefined : "sem acesso"} />
              <Summary icon={<AppWindow />} label="Apps" value={`${formatCount(r.unusedApps.length)} sem uso`} hint="há mais de 6 meses" />
            </div>

            {r.recommendations.length > 0 && (
              <div className="space-y-2">
                <h2 className="text-[13px] font-semibold">Recomendações</h2>
                {r.recommendations.map((rec) => (
                  <div key={rec.id} className={`flex items-start gap-3 rounded-xl border px-4 py-3 ${rec.level === "high" ? "border-review/40 bg-[color-mix(in_srgb,var(--review)_10%,transparent)]" : "border-line bg-surface"}`}>
                    {rec.level === "high" ? <AlertTriangle className="mt-0.5 size-4 text-review" /> : <Sparkles className="mt-0.5 size-4 text-accent" />}
                    <div>
                      <div className="font-medium">{rec.title}</div>
                      <div className="text-[12.5px] text-ink-2">{rec.detail}</div>
                    </div>
                  </div>
                ))}
              </div>
            )}

            <div className="space-y-3">
              <h2 className="text-[13px] font-semibold">Revisão</h2>
              {r.trash.readable && r.trash.items.length > 0 && (
                <Card className="flex items-center gap-3 px-4 py-3">
                  <span className="grid size-8 place-items-center rounded-lg bg-danger-soft text-danger">
                    <Trash2 className="size-4" />
                  </span>
                  <label className="flex flex-1 cursor-pointer items-center gap-3">
                    <input type="checkbox" className="size-3.5 accent-[var(--accent)]" checked={emptyTrash} onChange={(e) => setEmptyTrash(e.target.checked)} />
                    <span>
                      <span className="block font-medium">Esvaziar a Lixeira</span>
                      <span className="block text-[12px] text-ink-3">
                        Apaga de vez {r.trash.items.length === 1 ? "o item que está" : `os ${formatCount(r.trash.items.length)} itens que estão`} lá agora (
                        {formatBytes(r.trash.bytes)}). O que esta limpeza mover para a Lixeira não é apagado.
                      </span>
                    </span>
                  </label>
                </Card>
              )}
              <Section icon={<Download />} title="Downloads" rows={rows.downloads} selected={selected} onToggle={toggle} onAll={(v) => setMany(rows.downloads.map((x) => x.key), v)} note="Instaladores antigos já vêm marcados; o resto fica para você decidir." />
              <Section icon={<Copy />} title="Duplicados (acima de 10 MB)" rows={rows.duplicates} selected={selected} onToggle={toggle} note="Uma cópia de cada grupo sempre fica. Duplicados menores estão na seção Duplicados." />
              <Section icon={<FileSearch />} title="Arquivos grandes e esquecidos" rows={rows.largeOld} selected={selected} onToggle={toggle} note="Mais de 500 MB e sem modificação há mais de um ano. Nenhum vem marcado." />
              <Section icon={<PackageX />} title="Restos de apps desinstalados" rows={rows.leftovers} selected={selected} onToggle={toggle} onAll={(v) => setMany(rows.leftovers.map((x) => x.key), v)} />
              {r.unusedApps.length > 0 && (
                <Card className="flex items-center gap-3 px-4 py-3">
                  <span className="grid size-8 place-items-center rounded-lg bg-accent-soft text-accent">
                    <AppWindow className="size-4" />
                  </span>
                  <div className="flex-1">
                    <div className="font-medium">{formatCount(r.unusedApps.length)} apps sem uso há mais de 6 meses</div>
                    <div className="truncate text-[12px] text-ink-3">{r.unusedApps.map((a) => a.name).join(", ")}</div>
                  </div>
                  <Button size="sm" onClick={() => go("apps")}>
                    Ver em Aplicativos
                  </Button>
                </Card>
              )}
            </div>

            <div className="sticky bottom-0 -mx-8 flex items-center gap-3 border-t border-line bg-surface-2/90 px-8 py-3 backdrop-blur">
              <div className="flex-1 text-[12.5px] text-ink-2">
                <span className="font-medium text-ink">{formatCount(chosenCount + (emptyTrash ? 1 : 0))}</span> ações ·{" "}
                <span className="tabular font-medium text-ink">{formatBytes(chosenBytes)}</span>
              </div>
              <Button variant="primary" disabled={!chosenCount && !emptyTrash} onClick={() => setConfirm(true)}>
                Limpar…
              </Button>
            </div>
          </>
        )}
      </div>

      {confirm && r && (
        <Modal onClose={busy ? undefined : () => setConfirm(false)} width={480}>
          <div className="p-6">
            <h2 className="text-[16px] font-semibold">Confirmar limpeza</h2>
            <ul className="mt-3 space-y-1 text-[12.5px] text-ink-2">
              {emptyTrash && (
                <li className="text-danger">
                  Apagar de vez {formatCount(r.trash.items.length)} itens da Lixeira ({formatBytes(r.trash.bytes)})
                </li>
              )}
              {chosenCount > 0 && (
                <li>
                  Mover {formatCount(chosenCount)} itens para a Lixeira ({formatBytes(chosenBytes - (emptyTrash ? r.trash.bytes : 0))}); dá para recuperar pelo Finder
                </li>
              )}
            </ul>
            <p className="mt-3 text-[11.5px] text-ink-3">Cada item é validado de novo antes: pastas do sistema, arquivos alterados desde a análise e a última cópia de um duplicado são sempre preservados.</p>
          </div>
          <div className="flex justify-end gap-2 border-t border-line px-6 py-3">
            <Button onClick={() => setConfirm(false)} disabled={busy}>
              Cancelar
            </Button>
            <Button variant={emptyTrash ? "danger" : "primary"} busy={busy} onClick={run}>
              Limpar {formatBytes(chosenBytes)}
            </Button>
          </div>
        </Modal>
      )}

      {outcomes && (
        <Modal onClose={() => { setOutcomes(null); start(MODULE, "start_smart_care", {}); }} width={480}>
          <div className="p-6">
            <h2 className="flex items-center gap-2 text-[16px] font-semibold">
              <Check className="size-5 text-safe" /> Limpeza concluída
            </h2>
            <p className="mt-1.5 text-ink-2">
              {formatCount(outcomes.filter((o) => o.ok).length)} itens · {formatBytes(outcomes.filter((o) => o.ok).reduce((s, o) => s + o.size, 0))}
              {outcomes.some((o) => !o.ok) && `. ${outcomes.filter((o) => !o.ok).length} não foram processados (detalhes em Ajustes → Registro de operações).`}
            </p>
          </div>
          <div className="flex justify-end border-t border-line px-6 py-3">
            <Button variant="primary" onClick={() => { setOutcomes(null); start(MODULE, "start_smart_care", {}); }}>
              Analisar de novo
            </Button>
          </div>
        </Modal>
      )}
    </div>
  );
}
