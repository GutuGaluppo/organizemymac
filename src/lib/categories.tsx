import { AppWindow, Archive, Disc, File, FileCode, FileText, Folder, Image, Music, Package, Video } from "lucide-react";
import type { FileCategory, FileEntry } from "../types";

export const categoryLabel: Record<FileCategory, string> = {
  image: "Imagem",
  video: "Vídeo",
  audio: "Áudio",
  document: "Documento",
  archive: "Compactado",
  diskImage: "Imagem de disco",
  installer: "Instalador",
  code: "Código",
  application: "Aplicativo",
  other: "Outro",
};

export function CategoryIcon({ entry, className = "size-4" }: { entry: Pick<FileEntry, "category" | "isDirectory">; className?: string }) {
  if (entry.isDirectory && entry.category !== "application") return <Folder className={`${className} text-accent`} />;
  const icons: Record<FileCategory, React.ReactNode> = {
    image: <Image className={className} />,
    video: <Video className={className} />,
    audio: <Music className={className} />,
    document: <FileText className={className} />,
    archive: <Archive className={className} />,
    diskImage: <Disc className={className} />,
    installer: <Package className={className} />,
    code: <FileCode className={className} />,
    application: <AppWindow className={className} />,
    other: <File className={className} />,
  };
  return <span className="text-ink-3">{icons[entry.category]}</span>;
}

export function typeLabel(entry: FileEntry): string {
  if (entry.isDirectory && entry.category !== "application") return "Pasta";
  if (entry.category === "other" && entry.extension) return entry.extension.toUpperCase();
  return categoryLabel[entry.category];
}
