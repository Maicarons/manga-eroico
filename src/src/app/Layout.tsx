import { NavLink, Outlet } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useTheme, type Theme } from "@/lib/theme";
import { LANG_META, SUPPORTED_LANGS, type SupportedLang } from "@/i18n";

const NAV = [
  { to: "/projects", key: "nav.projects", icon: "🗂️" },
  { to: "/workflow", key: "nav.workflow", icon: "🔀" },
  { to: "/editor", key: "nav.editor", icon: "🖌️" },
  { to: "/models", key: "nav.models", icon: "📦" },
  { to: "/settings", key: "nav.settings", icon: "⚙️" },
] as const;

const THEME_OPTIONS: Theme[] = ["light", "dark", "system"];

export default function Layout() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useTheme();

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
        {NAV.map((item) => (
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
            <span className="mr-2">{item.icon}</span>
            {t(item.key)}
          </NavLink>
        ))}

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
