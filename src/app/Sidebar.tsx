import type { ReactNode } from "react";
import { Activity, ArrowUpCircle, ShieldQuestion, Brush, Cloud, Images, Sparkles, AppWindow, PackageX, Copy, Download, FileSearch, FolderSearch, LayoutGrid, Map as MapIcon, Settings, Trash2 } from "lucide-react";
import { useNav, type Section } from "../stores/nav";
import { useJobs } from "../stores/jobs";

type Item = { id: Section; label: string; icon: ReactNode; job?: string };
type Group = { label?: string; items: Item[] };

const groups: Group[] = [
  {
    items: [
      { id: "overview", label: "Visão geral", icon: <LayoutGrid /> },
      { id: "smartCare", label: "Cuidado inteligente", icon: <Sparkles />, job: "smartCare" },
    ],
  },
  {
    label: "Armazenamento",
    items: [
      { id: "scanner", label: "Scanner", icon: <FolderSearch />, job: "scanner" },
      { id: "spaceMap", label: "Mapa de espaço", icon: <MapIcon /> },
      { id: "largeFiles", label: "Grandes e antigos", icon: <FileSearch />, job: "largeFiles" },
      { id: "downloads", label: "Downloads", icon: <Download />, job: "downloads" },
      { id: "duplicates", label: "Duplicados", icon: <Copy />, job: "duplicates" },
      { id: "similarImages", label: "Imagens parecidas", icon: <Images />, job: "similarImages" },
      { id: "cleanup", label: "Limpeza avançada", icon: <Brush /> },
      { id: "cloud", label: "Nuvem", icon: <Cloud />, job: "cloud" },
      { id: "trash", label: "Lixeira", icon: <Trash2 /> },
    ],
  },
  {
    label: "Aplicativos",
    items: [
      { id: "apps", label: "Aplicativos", icon: <AppWindow />, job: "apps" },
      { id: "leftovers", label: "Restos de apps", icon: <PackageX />, job: "leftovers" },
      { id: "updates", label: "Atualizações", icon: <ArrowUpCircle />, job: "updates" },
    ],
  },
  {
    label: "Mac",
    items: [
      { id: "performance", label: "Desempenho", icon: <Activity /> },
      { id: "security", label: "Auditoria de segurança", icon: <ShieldQuestion />, job: "security" },
    ],
  },
];

function NavItem({ item }: { item: Item }) {
  const { section, go } = useNav();
  const running = useJobs((s) => (item.job ? s.jobs[item.job]?.status === "running" : false));
  const active = section === item.id;
  return (
    <button
      onClick={() => go(item.id)}
      className={`flex h-7 w-full items-center gap-2.5 rounded-md px-2.5 text-left text-[13px] transition-colors [&_svg]:size-4 [&_svg]:shrink-0 ${
        active ? "bg-black/8 dark:bg-white/12 text-ink font-medium" : "text-ink-2 hover:bg-black/4 dark:hover:bg-white/6"
      }`}
    >
      <span className={active ? "text-accent" : "text-ink-3"}>{item.icon}</span>
      <span className="flex-1 truncate">{item.label}</span>
      {running && <span className="size-1.5 rounded-full bg-accent animate-pulse" aria-label="em andamento" />}
    </button>
  );
}

export function Sidebar() {
  return (
    <nav className="drag flex h-full w-[212px] shrink-0 flex-col gap-4 px-3 pt-14 pb-3">
      <div className="flex-1 space-y-4 overflow-y-auto [-webkit-app-region:no-drag]">
        {groups.map((g, i) => (
          <div key={i}>
            {g.label && <div className="px-2.5 pb-1 text-[11px] font-semibold text-ink-3">{g.label}</div>}
            <div className="space-y-0.5">
              {g.items.map((item) => (
                <NavItem key={item.id} item={item} />
              ))}
            </div>
          </div>
        ))}
      </div>
      <div className="[-webkit-app-region:no-drag]">
        <NavItem item={{ id: "settings", label: "Ajustes", icon: <Settings /> }} />
      </div>
    </nav>
  );
}
