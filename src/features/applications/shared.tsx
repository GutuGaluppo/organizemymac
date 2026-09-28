import { useQuery } from "@tanstack/react-query";
import { AppWindow } from "lucide-react";
import { api } from "../../lib/ipc";
import { Badge } from "../../components/ui";
import type { Confidence, LeftoverKind, MatchRule } from "../../types";

export const kindLabel: Record<LeftoverKind, string> = {
  applicationSupport: "Application Support",
  caches: "Caches",
  preferences: "Preferências",
  logs: "Registros",
  savedState: "Estado salvo das janelas",
  webKit: "Dados da web (WebKit)",
  httpStorage: "Armazenamento HTTP",
  cookies: "Cookies",
  container: "Contêiner",
  groupContainer: "Contêiner compartilhado",
  launchAgent: "Item de início (LaunchAgent)",
  applicationScripts: "Scripts do app",
};

export const ruleLabel: Record<MatchRule, string> = {
  bundleId: "Nome igual ao identificador do app",
  knownPath: "Pasta conhecida deste app",
  vendor: "Pasta do fabricante: pode ter dados de outros apps dele",
  appName: "Pasta com o mesmo nome do app",
  fuzzy: "Nome parecido com o do app: confira antes",
  sharedGroup: "Compartilhado com outros apps do fabricante",
  orphan: "Nenhum app instalado tem este identificador",
};

export function ConfidenceBadge({ confidence }: { confidence: Confidence }) {
  if (confidence === "safe") return <Badge tone="safe">Seguro</Badge>;
  if (confidence === "danger") return <Badge tone="danger">Risco</Badge>;
  return <Badge tone="review">Revisar</Badge>;
}

export function AppIcon({ path, size = 32 }: { path: string; size?: number }) {
  const { data } = useQuery({ queryKey: ["appIcon", path], queryFn: () => api.appIcon(path), staleTime: Infinity });
  return data ? (
    <img src={data} alt="" width={size} height={size} className="shrink-0" draggable={false} />
  ) : (
    <div className="grid shrink-0 place-items-center rounded-lg bg-surface-3 text-ink-3" style={{ width: size, height: size }}>
      <AppWindow className="size-4" />
    </div>
  );
}
