import { create } from "zustand";

export type Section = "overview" | "scanner" | "spaceMap" | "largeFiles" | "downloads" | "duplicates" | "trash" | "apps" | "leftovers" | "settings";

type NavStore = {
  section: Section;
  go: (section: Section) => void;
};

export const useNav = create<NavStore>((set) => ({
  section: "overview",
  go: (section) => set({ section }),
}));
