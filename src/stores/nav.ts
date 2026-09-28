import { create } from "zustand";

export type Section = "overview" | "scanner" | "largeFiles" | "downloads" | "trash" | "settings";

type NavStore = {
  section: Section;
  go: (section: Section) => void;
};

export const useNav = create<NavStore>((set) => ({
  section: "overview",
  go: (section) => set({ section }),
}));
