import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, RefreshCw, Trash2 } from "lucide-react";
import { api, errorMessage } from "../../lib/ipc";
import { Button, Card, EmptyState, ErrorNote, PageHeader, Stat } from "../../components/ui";
import { Modal } from "../../components/ReviewDialog";
import { CategoryIcon } from "../../lib/categories";
import { formatBytes, formatCount } from "../../lib/format";
import { FullDiskAccessRow } from "../onboarding/Onboarding";

function ConfirmEmpty({ files, folders, bytes, onClose, onConfirm, busy }: { files: number; folders: number; bytes: number; onClose: () => void; onConfirm: () => void; busy: boolean }) {
  return (
    <Modal onClose={busy ? undefined : onClose} width={460}>
      <div className="p-6">
        <div className="mb-3 grid size-10 place-items-center rounded-xl bg-danger-soft text-danger">
          <AlertTriangle className="size-5" />
        </div>
        <h2 className="text-[16px] font-semibold">Apagar definitivamente o conteúdo da Lixeira?</h2>
        <p className="mt-1.5 text-ink-2">
          {formatCount(files)} {files === 1 ? "arquivo" : "arquivos"} e {formatCount(folders)} {folders === 1 ? "pasta" : "pastas"}, {formatBytes(bytes)}. Depois de
          apagados, não dá para recuperar.
        </p>
      </div>
      <div className="flex justify-end gap-2 border-t border-line px-6 py-3">
        <Button onClick={onClose} disabled={busy}>
          Cancelar
        </Button>
        <Button variant="danger" busy={busy} onClick={onConfirm}>
          Apagar {formatBytes(bytes)}
        </Button>
      </div>
    </Modal>
  );
}

export function TrashPage() {
  const qc = useQueryClient();
  const { data, isFetching, refetch, error } = useQuery({ queryKey: ["trash"], queryFn: api.trashSummary });
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const empty = async () => {
    setBusy(true);
    setFailure(null);
    try {
      const out = await api.emptyTrash();
      const ok = out.filter((o) => o.ok);
      const failed = out.length - ok.length;
      setMessage(`${formatBytes(ok.reduce((s, o) => s + o.size, 0))} liberados.${failed ? ` ${failed} itens não puderam ser apagados.` : ""}`);
      setConfirm(false);
    } catch (e) {
      setFailure(errorMessage(e));
    } finally {
      setBusy(false);
      qc.invalidateQueries({ queryKey: ["trash"] });
      qc.invalidateQueries({ queryKey: ["operations"] });
      qc.invalidateQueries({ queryKey: ["metrics"] });
      qc.invalidateQueries({ queryKey: ["volumes"] });
    }
  };
  const emptyWithFinder = async () => {
    setFailure(null);
    try {
      await api.emptyTrashWithFinder();
      setMessage("O Finder esvaziou a Lixeira.");
      qc.invalidateQueries({ queryKey: ["volumes"] });
    } catch (e) {
      setFailure(errorMessage(e));
    }
  };

  return (
    <div className="h-full overflow-y-auto">
      <PageHeader
        title="Lixeira"
        subtitle="Itens na Lixeira ainda ocupam espaço. Esvaziar é a única ação do app que apaga de vez, e sempre pede confirmação."
        actions={
          <Button variant="ghost" icon={<RefreshCw className="size-3.5" />} busy={isFetching} onClick={() => refetch()}>
            Atualizar
          </Button>
        }
      />
      <div className="space-y-4 px-8 pb-10">
        {error && <ErrorNote>{errorMessage(error)}</ErrorNote>}
        {failure && <ErrorNote>{failure}</ErrorNote>}
        {message && <div className="rounded-lg bg-accent-soft px-3 py-2 text-[12.5px] text-accent">{message}</div>}

        {data && !data.readable && (
          <Card className="space-y-4 p-5">
            <div>
              <div className="font-medium">O macOS não deixa o app ver o conteúdo da Lixeira</div>
              <p className="mt-1 text-[12.5px] text-ink-2">
                Com Acesso Total ao Disco, o app mostra o que há na Lixeira e quanto espaço ocupa antes de esvaziar. Sem ele, você pode pedir
                ao Finder para esvaziar (o macOS pergunta se o OrganizeMyMac pode controlar o Finder).
              </p>
            </div>
            <FullDiskAccessRow />
            <Button icon={<Trash2 className="size-3.5" />} onClick={emptyWithFinder}>
              Esvaziar pelo Finder
            </Button>
          </Card>
        )}

        {data?.readable && data.items.length === 0 && <EmptyState icon={<Trash2 className="size-6" />} title="A Lixeira está vazia" />}

        {data?.readable && data.items.length > 0 && (
          <>
            <Card className="flex items-end justify-between gap-6 p-5">
              <div className="grid flex-1 grid-cols-3 gap-4">
                <Stat label="Arquivos" value={formatCount(data.files)} />
                <Stat label="Pastas" value={formatCount(data.folders)} />
                <Stat label="Espaço" value={formatBytes(data.bytes)} />
              </div>
              <Button variant="danger" icon={<Trash2 className="size-3.5" />} onClick={() => setConfirm(true)}>
                Esvaziar Lixeira…
              </Button>
            </Card>
            <Card className="divide-y divide-line">
              {data.items.slice(0, 200).map((f) => (
                <div key={f.path} className="flex items-center gap-3 px-4 py-2">
                  <CategoryIcon entry={f} />
                  <span className="flex-1 truncate text-[12.5px]">{f.name}</span>
                  <span className="tabular text-[12.5px] text-ink-2">{formatBytes(f.sizeLogical)}</span>
                </div>
              ))}
            </Card>
          </>
        )}
      </div>
      {confirm && data && (
        <ConfirmEmpty files={data.files} folders={data.folders} bytes={data.bytes} busy={busy} onClose={() => setConfirm(false)} onConfirm={empty} />
      )}
    </div>
  );
}
