import { create } from "zustand";

export interface ProjectCard {
  root: string;
  name: string;
  sourceLang: string;
  targetLang: string;
  pages: number;
  chapters: number;
  lastOpened: string;
}

interface ProjectsState {
  projects: ProjectCard[];
  activeRoot: string | null;
  upsert: (p: ProjectCard) => void;
  setActive: (root: string | null) => void;
}

export const useProjects = create<ProjectsState>()((set) => ({
  projects: [],
  activeRoot: null,
  upsert: (p) =>
    set((s) => {
      const rest = s.projects.filter((x) => x.root !== p.root);
      return { projects: [p, ...rest] };
    }),
  setActive: (root) => set({ activeRoot: root }),
}));
