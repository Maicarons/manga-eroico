import { create } from "zustand";
import { api } from "@/lib/tauri";

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
  /** true once the active project has been opened on the backend */
  activated: boolean;
  upsert: (p: ProjectCard) => void;
  setActive: (root: string | null) => void;
  /** Sets the active project AND opens it on the Tauri backend (no-op in
   * browser mock mode, which resolves instantly). */
  activate: (root: string | null) => Promise<void>;
  restore: () => Promise<void>;
}

const STORAGE_KEY = "manga-eroico.projects.v1";

interface Persisted {
  projects: ProjectCard[];
  activeRoot: string | null;
}

function load(): Persisted {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { projects: [], activeRoot: null };
    const parsed = JSON.parse(raw) as Persisted;
    return { projects: parsed.projects ?? [], activeRoot: parsed.activeRoot ?? null };
  } catch {
    return { projects: [], activeRoot: null };
  }
}

function save(state: { projects: ProjectCard[]; activeRoot: string | null }) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // private mode etc. — persistence is best-effort
  }
}

export const useProjects = create<ProjectsState>()((set) => ({
  projects: [],
  activeRoot: null,
  activated: false,
  upsert: (p) =>
    set((s) => {
      const projects = [p, ...s.projects.filter((x) => x.root !== p.root)];
      save({ projects, activeRoot: s.activeRoot });
      return { projects };
    }),
  setActive: (root) =>
    set((s) => {
      save({ projects: s.projects, activeRoot: root });
      return { activeRoot: root, activated: false };
    }),
  activate: async (root) => {
    set((s) => {
      save({ projects: s.projects, activeRoot: root });
      return { activeRoot: root, activated: false };
    });
    if (root) {
      try {
        await api.openProject(root);
      } catch (e) {
        // project may have been deleted on disk — keep UI usable
        console.warn("open_project failed", e);
      }
    }
    set({ activated: true });
  },
  restore: async () => {
    const { projects, activeRoot } = load();
    set({ projects, activeRoot, activated: false });
    if (activeRoot) {
      try {
        await api.openProject(activeRoot);
        set({ activated: true });
      } catch {
        // stale entry — drop it so the UI starts clean
        const filtered = projects.filter((x) => x.root !== activeRoot);
        set({ projects: filtered, activeRoot: null });
        save({ projects: filtered, activeRoot: null });
      }
    }
  },
}));

export const activeProject = (): ProjectCard | undefined => {
  const { projects, activeRoot } = useProjects.getState();
  return projects.find((x) => x.root === activeRoot);
};
