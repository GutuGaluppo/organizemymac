import { useState } from "react";
import { Sidebar } from "./Sidebar";
import { useNav } from "../stores/nav";
import { ScannerPage } from "../features/scanner/ScannerPage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { OverviewPage } from "../features/dashboard/OverviewPage";
import { LargeFilesPage } from "../features/storage/LargeFilesPage";
import { DownloadsPage } from "../features/storage/DownloadsPage";
import { TrashPage } from "../features/storage/TrashPage";
import { Onboarding, hasOnboarded } from "../features/onboarding/Onboarding";

export function App() {
  const section = useNav((s) => s.section);
  const [onboarded, setOnboarded] = useState(hasOnboarded);

  return (
    <div className="flex h-full">
      <Sidebar />
      <main className="min-w-0 flex-1 border-l border-line bg-surface-2">
        {section === "overview" && <OverviewPage />}
        {section === "scanner" && <ScannerPage />}
        {section === "largeFiles" && <LargeFilesPage />}
        {section === "downloads" && <DownloadsPage />}
        {section === "trash" && <TrashPage />}
        {section === "settings" && <SettingsPage />}
      </main>
      {!onboarded && <Onboarding onDone={() => setOnboarded(true)} />}
    </div>
  );
}
