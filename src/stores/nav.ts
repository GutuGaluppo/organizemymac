import { create } from "zustand";

export type Section = "scanner" | "settings";

type NavStore = {
  section: Section;
  go: (section: Section) => void;
};

export const useNav = create<NavStore>((set) => ({
  section: "scanner",
  go: (section) => set({ section }),
}));
