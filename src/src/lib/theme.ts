import { create } from "zustand";
import { persist } from "zustand/middleware";

export type Theme = "light" | "dark" | "system";

interface ThemeState {
  theme: Theme;
  resolved: "light" | "dark";
  setTheme: (t: Theme) => void;
}

function systemPrefers(): "light" | "dark" {
  if (typeof window === "undefined") return "dark";
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export const useTheme = create<ThemeState>()(
  persist(
    (set) => ({
      theme: "system",
      resolved: systemPrefers(),
      setTheme: (t) =>
        set({ theme: t, resolved: t === "system" ? systemPrefers() : t }),
    }),
    { name: "me-theme" },
  ),
);

// keep resolution in sync when following the OS
if (typeof window !== "undefined") {
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    const s = useTheme.getState();
    if (s.theme === "system") useTheme.setState({ resolved: systemPrefers() });
  });
}
