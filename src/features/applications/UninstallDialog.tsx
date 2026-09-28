import { useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, CheckCircle2, Info, Loader2, ShieldAlert, Trash2 } from "lucide-react";
import { api, errorMessage } from "../../lib/ipc";
import { Badge, Button, ErrorNote } from "../../components/ui";
import { Modal } from "../../components/ReviewDialog";
import { formatBytes, formatCount, shortPath } from "../../lib/format";
import { AppIcon, ConfidenceBadge, kindLabel, ruleLabel } from "./shared";
import type { AppInfo, LeftoverKind, OperationOutcome } from "../../types";

/**
 * Complete preview of an uninstall: the app, every associated file with how it was matched, and
 * a breakdown by location. Safe matches are preselected; the rest needs the user's decision.
 */
export function UninstallDialog({ app, home, onClose, onRemoved }: { app: AppInfo; home?: string; onClose: () => void; onRemoved: (path: string) => void }) {
  const qc = useQueryClient();
  const { data: plan, error, isLoading, refetch } = useQuery({ queryKey: ["uninstallPlan", app.path], queryFn: () => api.uninstallPlan(app.path), gcTime: 0 });
  const [chosen, setChosen] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const [outcomes, setOutcomes] = useState<OperationOutcome[] | null>(null);

  useEffect(() => {
    if (plan) setChosen(new Set(plan.leftovers.filter((l) => l.selected).map((l) => l.path)));
  }, [plan]);

  const breakdown = useMemo(() => {
    const rows = new Map<LeftoverKind, number>();
    for (const l of plan?.leftovers ?? []) if (chosen.has(l.path)) rows.set(l.kind, (rows.get(l.kind) ?? 0) + l.size);
    return [...rows.entries()].sort((a, b) => b[1] - a[1]);
  }, [plan, chosen]);
  const total = (plan?.appSize ?? 0) + breakdown.reduce((s, [, v]) => s + v, 0);

  const run = async () => {
    if (!plan) return;
    setBusy(true);
    setFailure(null);
    try {
      const out = await api.uninstallApp(plan.appPath, [...chosen]);
      setOutcomes(out);
      if (out.find((o) => o.path === plan.appPath)?.ok) onRemoved(plan.appPath);
      qc.invalidateQueries({ queryKey: ["operations"] });
      qc.invalidateQueries({ queryKey: ["metrics"] });
    } catch (e) {
      setFailure(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  if (outcomes) {
    const ok = outcomes.filter((o) => o.ok);
    const failed = outcomes.filter((o) => !o.ok);
    return (
      <Modal onClose={onClose}>
        <div className="p-6">
          <div className="flex items-center gap-2 text-[16px] font-semibold">
            <CheckCircle2 className="size-5 text-safe" /> {app.name} foi para a Lixeira
          </div>
          <p className="mt-1 text-ink-2">
            {formatCount(ok.length)} {ok.length === 1 ? "item" : "itens"} · {formatBytes(ok.reduce((s, o) => s + o.size, 0))}. Dá para recuperar pela Lixeira
            até ela ser esvaziada.
          </p>
          {failed.length > 0 && (
            <ul className="mt-3 max-h-40 space-y-1 overflow-y-auto rounded-lg bg-danger-soft p-3 text-[12px] selectable">
              {failed.map((f) => (
                <li key={f.path}>
                  {shortPath(f.path, home)} — <span className="text-danger">{f.error}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="flex justify-end border-t border-line px-6 py-3">
          <Button variant="primary" onClick={onClose}>
            OK
          </Button>
        </div>
      </Modal>
    );
  }

  return (
    <Modal onClose={busy ? undefined : onClose} width={720}>
      <div className="flex items-center gap-3 border-b border-line px-6 pt-5 pb-4">
        <AppIcon path={app.path} size={40} />
        <div className="min-w-0 flex-1">
          <h2 className="text-[16px] font-semibold">Desinstalar {app.name}</h2>
          <div className="truncate text-[12px] text-ink-3">
            {app.bundleId} {app.version && `· versão ${app.version}`}
          </div>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">
        {isLoading && (
          <div className="flex items-center gap-2 text-ink-2">
            <Loader2 className="size-4 animate-spin" /> Procurando os arquivos do app em ~/Library…
          </div>
        )}
        {error && <ErrorNote>{errorMessage(error)}</ErrorNote>}
        {plan?.protected && (
          <div className="flex gap-2 rounded-lg bg-surface-2 p-3 text-[12.5px] text-ink-2">
            <ShieldAlert className="size-4 shrink-0 text-ink-3" /> Este app vem com o macOS e é protegido pelo sistema. Não pode ser removido.
          </div>
        )}
        {plan?.running && (
          <div className="mb-3 flex items-center gap-2 rounded-lg bg-[color-mix(in_srgb,var(--review)_14%,transparent)] p-3 text-[12.5px]">
            <AlertTriangle className="size-4 shrink-0 text-review" />
            <span className="flex-1">{plan.name} está aberto. Encerre o app antes de desinstalar.</span>
            <Button size="sm" onClick={() => refetch()}>
              Verificar de novo
            </Button>
          </div>
        )}
        {plan && !plan.protected && (
          <>
            <div className="rounded-lg border border-line">
              <div className="grid grid-cols-[minmax(0,1fr)_90px] px-3 py-1.5 text-[12.5px]">
                <span>Aplicativo</span>
                <span className="tabular text-right">{formatBytes(plan.appSize)}</span>
              </div>
              {breakdown.map(([kind, size]) => (
                <div key={kind} className="grid grid-cols-[minmax(0,1fr)_90px] border-t border-line px-3 py-1.5 text-[12.5px]">
                  <span>{kindLabel[kind]}</span>
                  <span className="tabular text-right">{formatBytes(size)}</span>
                </div>
              ))}
              <div className="grid grid-cols-[minmax(0,1fr)_90px] border-t border-line bg-surface-2 px-3 py-1.5 text-[12.5px] font-semibold">
                <span>Total</span>
                <span className="tabular text-right">{formatBytes(total)}</span>
              </div>
            </div>

            <h3 className="mt-5 mb-2 text-[13px] font-semibold">Arquivos do app em ~/Library</h3>
            {plan.leftovers.length === 0 && <p className="text-[12.5px] text-ink-3">Nenhum arquivo associado encontrado.</p>}
            <div className="space-y-1">
              {plan.leftovers.map((l) => (
                <label key={l.path} className="grid cursor-pointer grid-cols-[18px_minmax(0,1fr)_auto_80px] items-center gap-3 rounded-md px-2 py-1.5 hover:bg-surface-2">
                  <input
                    type="checkbox"
                    className="size-3.5 accent-[var(--accent)]"
                    checked={chosen.has(l.path)}
                    onChange={(e) =>
                      setChosen((prev) => {
                        const next = new Set(prev);
                        if (e.target.checked) next.add(l.path);
                        else next.delete(l.path);
                        return next;
                      })
                    }
                  />
                  <div className="min-w-0">
                    <div className="truncate text-[12.5px] selectable" title={l.path}>
                      {shortPath(l.path, home)}
                    </div>
                    <div className="truncate text-[11px] text-ink-3">
                      {kindLabel[l.kind]} · {ruleLabel[l.rule]}
                    </div>
                  </div>
                  <ConfidenceBadge confidence={l.confidence} />
                  <span className="tabular text-right text-[12.5px]">{formatBytes(l.size)}</span>
                </label>
              ))}
            </div>
            {!plan.fullDiskAccess && (
              <p className="mt-3 flex gap-1.5 text-[11.5px] text-ink-3">
                <Info className="mt-px size-3.5 shrink-0" /> Sem Acesso Total ao Disco, os contêineres do app não foram verificados.
              </p>
            )}
          </>
        )}
        {failure && (
          <div className="mt-3">
            <ErrorNote>{failure}</ErrorNote>
          </div>
        )}
      </div>

      <div className="flex items-center gap-3 border-t border-line px-6 py-3">
        <div className="flex flex-1 flex-wrap items-center gap-1.5 text-[11.5px] text-ink-3">
          <Badge tone="safe">Seguro</Badge> pré-selecionado <Badge tone="review">Revisar</Badge> <Badge tone="danger">Risco</Badge> só se você marcar
        </div>
        <Button onClick={onClose} disabled={busy}>
          Cancelar
        </Button>
        <Button variant="danger" busy={busy} disabled={!plan || plan.protected || plan.running} icon={<Trash2 className="size-3.5" />} onClick={run}>
          Mover {formatBytes(total)} para a Lixeira
        </Button>
      </div>
    </Modal>
  );
}
