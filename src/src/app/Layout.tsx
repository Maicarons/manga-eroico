import { NavLink, Outlet } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useTheme, type Theme } from "@/lib/theme";
import { useProjects, activeProject } from "@/features/projects/projectsStore";
import { LANG_META, SUPPORTED_LANGS, type SupportedLang } from "@/i18n";

const GLOBAL_NAV = [
  {
    to: "/projects",
    key: "nav.projects",
    path: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
  },
  {
    to: "/models",
    key: "nav.models",
    path: "M12 3 3 8l9 5 9-5zM3 8v8l9 5 9-5V8",
  },
  {
    to: "/settings",
    key: "nav.settings",
    path: "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM12 2v3m0 14v3M2 12h3m14 0h3M5 5l2 2m10 10 2 2M19 5l-2 2M7 17l-2 2",
  },
] as const;

const PROJECT_NAV = [
  {
    to: "/workflow",
    key: "nav.workflow",
    path: "M12 5v4m0 0-5.5 6.5M12 9l5.5 6.5M5 19h.01M19 19h.01M12 3.5h.01",
  },
  {
    to: "/editor",
    key: "nav.editor",
    path: "M4 20h16M6 16l10-10 2 2-10 10H6z",
  },
] as const;

const THEME_OPTIONS: Theme[] = ["light", "dark", "system"];

export default function Layout() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useTheme();
  const activeRoot = useProjects((st) => st.activeRoot);

  return (
    <div className="flex h-screen" style={{ background: "var(--bg)", color: "var(--text)" }}>
      <aside
        className="flex w-56 flex-col shrink-0 border-r p-4 gap-1"
        style={{ borderColor: "var(--border)", background: "var(--surface)" }}
        data-testid="sidebar"
      >
        <div className="mb-4 px-2">
          <div className="text-lg font-bold tracking-tight">manga-eroico</div>
          <div className="text-xs" style={{ color: "var(--text-muted)" }}>
            {t("common.tagline")}
          </div>
        </div>
        {GLOBAL_NAV.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            className={({ isActive }) =>
              `rounded-lg px-3 py-2 text-sm transition-colors hover:opacity-80 ${
                isActive ? "font-semibold" : ""
              }`
            }
            style={({ isActive }) =>
              isActive ? { background: "var(--accent)", color: "var(--accent-contrast)" } : {}
            }
          >
            <svg
              className="mr-2 h-4 w-4 shrink-0"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.8"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden
            >
              <path d={item.path} />
            </svg>
            <span data-testid={`nav-${item.to.slice(1)}`}>{t(item.key)}</span>
          </NavLink>
        ))}

        {activeRoot && (
          <>
            <div
              className="mt-3 mb-1 px-3 text-[11px] font-semibold uppercase tracking-wide"
              style={{ color: "var(--text-muted)" }}
            >
              {t("nav.currentProject")}
            </div>
            <div
              className="mx-3 mb-1 truncate rounded-lg px-3 py-2 text-xs"
              style={{ background: "var(--surface-2)", color: "var(--text-muted)" }}
              data-testid="sidebar-active-project"
            >
              {activeProject()?.name ?? activeRoot}
            </div>
            {PROJECT_NAV.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                className={({ isActive }) =>
                  `rounded-lg px-3 py-2 text-sm transition-colors hover:opacity-80 ${
                    isActive ? "font-semibold" : ""
                  }`
                }
                style={({ isActive }) =>
                  isActive ? { background: "var(--accent)", color: "var(--accent-contrast)" } : {}
                }
              >
                <svg
                  className="mr-2 h-4 w-4 shrink-0"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.8"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  aria-hidden
                >
                  <path d={item.path} />
                </svg>
                <span data-testid={`nav-${item.to.slice(1)}`}>{t(item.key)}</span>
              </NavLink>
            ))}
          </>
        )}

        <div className="mt-auto flex flex-col gap-3 pt-4" style={{ borderTop: "1px solid var(--border)" }}>
          <label className="flex items-center gap-2 text-xs" style={{ color: "var(--text-muted)" }}>
            🌐
            <select
              aria-label={t("settings.language")}
              className="flex-1 rounded border bg-transparent px-1 py-1 text-xs"
              style={{ borderColor: "var(--border)", color: "var(--text)" }}
              value={i18n.language}
              onChange={(e) => void i18n.changeLanguage(e.target.value as SupportedLang)}
            >
              {SUPPORTED_LANGS.map((l) => (
                <option key={l} value={l}>
                  {LANG_META[l].flag} {LANG_META[l].label}
                </option>
              ))}
            </select>
          </label>
          <div className="flex items-center gap-1 text-xs" style={{ color: "var(--text-muted)" }}>
            {t("nav.theme.label")}
            <div className="ml-auto flex gap-1">
              {THEME_OPTIONS.map((th) => (
                <button
                  key={th}
                  onClick={() => setTheme(th)}
                  aria-pressed={theme === th}
                  data-testid={`theme-${th}`}
                  title={t(`nav.theme.${th}`)}
                  className="rounded px-2 py-1"
                  style={{
                    background: theme === th ? "var(--accent)" : "var(--surface-2)",
                    color: theme === th ? "var(--accent-contrast)" : "var(--text)",
                  }}
                >
                  {th === "light" ? "☀️" : th === "dark" ? "🌙" : "💻"}
                </button>
              ))}
            </div>
          </div>
        </div>
      </aside>
      <main className="flex-1 overflow-auto p-6">
        <Outlet />
      </main>
    </div>
  );
}
