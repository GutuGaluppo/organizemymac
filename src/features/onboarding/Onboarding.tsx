import { useQuery } from "@tanstack/react-query";
import { ShieldCheck, Trash2, Eye, HardDrive } from "lucide-react";
import { api } from "../../lib/ipc";
import { Button } from "../../components/ui";

const KEY = "organizemymac.onboarded";

export function hasOnboarded(): boolean {
  try {
    return localStorage.getItem(KEY) === "1";
  } catch {
    return false;
  }
}

function markOnboarded() {
  try {
    localStorage.setItem(KEY, "1");
  } catch {
    /* private storage unavailable: show again next time */
  }
}

/** Full Disk Access status. macOS has no API for it: the app infers it and says so. */
export function FullDiskAccessRow({ compact }: { compact?: boolean }) {
  const { data, refetch, isFetching } = useQuery({
    queryKey: ["permissions"],
    queryFn: api.permissionStatus,
    refetchOnWindowFocus: true,
  });
  const status = data?.fullDiskAccess ?? "unknown";
  const label = { granted: "Concedido", denied: "Não concedido", unknown: "Não foi possível verificar" }[status];
  const dot = { granted: "bg-safe", denied: "bg-review", unknown: "bg-ink-3" }[status];
  return (
    <div className={`flex items-center gap-3 ${compact ? "" : "rounded-xl border border-line bg-surface-2 p-4"}`}>
      <HardDrive className="size-5 shrink-0 text-ink-3" />
      <div className="min-w-0 flex-1">
        <div className="font-medium text-ink">Acesso Total ao Disco</div>
        <div className="flex items-center gap-1.5 text-[12px] text-ink-2">
          <span className={`size-1.5 rounded-full ${dot}`} /> {label}
        </div>
      </div>
      <Button size="sm" variant="ghost" onClick={() => refetch()} busy={isFetching}>
        Verificar
      </Button>
      {status !== "granted" && (
        <Button size="sm" onClick={() => api.openSystemSettings("fullDiskAccess")}>
          Abrir Ajustes do Sistema
        </Button>
      )}
    </div>
  );
}

export function Onboarding({ onDone }: { onDone: () => void }) {
  const finish = () => {
    markOnboarded();
    onDone();
  };
  const principles = [
    { icon: <Eye />, title: "Você revisa tudo", text: "Nada é removido sem você ver o caminho e o tamanho de cada item antes." },
    { icon: <Trash2 />, title: "Vai para a Lixeira", text: "Os itens são movidos para a Lixeira, de onde podem ser recuperados. Apagar de vez fica desligado." },
    { icon: <ShieldCheck />, title: "Pastas do sistema protegidas", text: "/System, /usr, /bin, /private e /Library são bloqueados. O app nunca contorna as proteções do macOS." },
  ];
  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/25 backdrop-blur-sm p-6">
      <div className="w-full max-w-[560px] glass-strong rounded-2xl p-8">
        <h1 className="text-[22px] font-semibold tracking-tight">Bem-vindo ao OrganizeMyMac</h1>
        <p className="mt-1.5 text-ink-2">Entenda o que ocupa espaço no seu Mac e remova o que não precisa, com segurança.</p>

        <div className="mt-6 space-y-4">
          {principles.map((p) => (
            <div key={p.title} className="flex gap-3">
              <div className="grid size-8 shrink-0 place-items-center rounded-lg bg-accent-soft text-accent [&_svg]:size-4">{p.icon}</div>
              <div>
                <div className="font-medium">{p.title}</div>
                <div className="text-[12.5px] text-ink-2">{p.text}</div>
              </div>
            </div>
          ))}
        </div>

        <div className="mt-6">
          <FullDiskAccessRow />
          <p className="mt-2 text-[12px] text-ink-3">
            Opcional. Sem ele o app funciona, mas não enxerga pastas protegidas como Mail, Mensagens e Safari. Para conceder, ative o
            OrganizeMyMac em Ajustes do Sistema → Privacidade e Segurança → Acesso Total ao Disco e reabra o app.
          </p>
        </div>

        <div className="mt-7 flex justify-end">
          <Button variant="primary" onClick={finish}>
            Começar
          </Button>
        </div>
      </div>
    </div>
  );
}
