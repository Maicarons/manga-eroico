import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useTheme } from "@/lib/theme";
import { LANG_META, SUPPORTED_LANGS } from "@/i18n";
import { api } from "@/lib/tauri";

export default function SettingsPage() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useTheme();
  const [baseUrl, setBaseUrl] = useState("https://api.openai.com/v1");
  const [model, setModel] = useState("gpt-4o-mini");
  const [apiKey, setApiKey] = useState("");
  const [polishEnabled, setPolishEnabled] = useState(false);
  const [polishStyle, setPolishStyle] = useState("");
  const [testResult, setTestResult] = useState<string | null>(null);

  const testConnection = async () => {
    try {
      const res = await api.polishPreview([
        { id: "t1", page: 1, position: 1, source_text: "test", machine_translation: "テスト" },
      ]);
      setTestResult(`✓ ${t("settings.testOk")} — ${res.analysis.slice(0, 60)}`);
    } catch (e) {
      setTestResult(`✗ ${t("settings.testFail", { message: String(e) })}`);
    }
  };

  return (
    <div data-testid="settings-page" className="max-w-2xl">
      <h1 className="text-2xl font-bold">{t("settings.title")}</h1>
      <p className="mb-6 mt-1 text-sm" style={{ color: "var(--text-muted)" }}>
        {t("settings.subtitle")}
      </p>

      <section
        className="mb-6 rounded-xl border p-5"
        style={{ borderColor: "var(--border)", background: "var(--surface)" }}
        data-testid="polish-section"
      >
        <h2 className="mb-4 text-sm font-semibold">{t("settings.polishSection")}</h2>
        <label className="mb-3 block text-xs" style={{ color: "var(--text-muted)" }}>
          {t("settings.baseUrl")}
          <input
            data-testid="polish-baseurl"
            className="mt-1 w-full rounded-lg border px-3 py-2 text-sm"
            style={{ borderColor: "var(--border)", background: "var(--surface-2)", color: "var(--text)" }}
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
          />
        </label>
        <label className="mb-3 block text-xs" style={{ color: "var(--text-muted)" }}>
          {t("settings.model")}
          <input
            className="mt-1 w-full rounded-lg border px-3 py-2 text-sm"
            style={{ borderColor: "var(--border)", background: "var(--surface-2)", color: "var(--text)" }}
            value={model}
            onChange={(e) => setModel(e.target.value)}
          />
        </label>
        <label className="mb-3 block text-xs" style={{ color: "var(--text-muted)" }}>
          {t("settings.apiKey")}
          <input
            type="password"
            className="mt-1 w-full rounded-lg border px-3 py-2 text-sm"
            style={{ borderColor: "var(--border)", background: "var(--surface-2)", color: "var(--text)" }}
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="sk-…"
          />
        </label>
        <div className="mb-4 flex gap-2">
          <button
            data-testid="test-connection"
            onClick={() => void testConnection()}
            className="rounded-lg px-4 py-2 text-sm font-semibold"
            style={{ background: "var(--accent)", color: "var(--accent-contrast)" }}
          >
            {t("settings.testConnection")}
          </button>
          <button
            className="rounded-lg px-4 py-2 text-sm"
            style={{ background: "var(--surface-2)" }}
          >
            {t("settings.saveKey")}
          </button>
        </div>
        {testResult && (
          <p className="text-xs" data-testid="test-result" style={{ color: "var(--text-muted)" }}>
            {testResult}
          </p>
        )}
        <label className="mt-4 block text-xs" style={{ color: "var(--text-muted)" }}>
          {t("settings.polishStyle")}
          <select
            data-testid="polish-style"
            value={polishStyle}
            onChange={(e) => setPolishStyle(e.target.value)}
            className="mt-1 w-full rounded-lg border p-2 text-sm"
            style={{ borderColor: "var(--border)", background: "var(--surface-2)", color: "var(--text)" }}
          >
            <option value="">{t("settings.styleNone")}</option>
            <option value="formal">{t("settings.styleFormal")}</option>
            <option value="casual">{t("settings.styleCasual")}</option>
            <option value="literary">{t("settings.styleLiterary")}</option>
          </select>
        </label>
        <label className="mt-4 flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            data-testid="polish-toggle"
            checked={polishEnabled}
            onChange={(e) => {
              setPolishEnabled(e.target.checked);
              void api.setNodeEnabled("polish", e.target.checked);
            }}
          />
          {t("settings.polishToggle")}
        </label>
      </section>

      <section
        className="rounded-xl border p-5"
        style={{ borderColor: "var(--border)", background: "var(--surface)" }}
      >
        <h2 className="mb-4 text-sm font-semibold">{t("settings.language")}</h2>
        <div className="flex flex-wrap gap-2">
          {SUPPORTED_LANGS.map((l) => (
            <button
              key={l}
              onClick={() => void i18n.changeLanguage(l)}
              className="rounded-lg px-3 py-2 text-sm"
              style={{
                background: i18n.language === l ? "var(--accent)" : "var(--surface-2)",
                color: i18n.language === l ? "var(--accent-contrast)" : "var(--text)",
              }}
            >
              {LANG_META[l].flag} {LANG_META[l].label}
            </button>
          ))}
        </div>
        <h2 className="mb-3 mt-6 text-sm font-semibold">{t("nav.theme.label")}</h2>
        <div className="flex gap-2">
          {(["light", "dark", "system"] as const).map((th) => (
            <button
              key={th}
              onClick={() => setTheme(th)}
              className="rounded-lg px-3 py-2 text-sm"
              style={{
                background: theme === th ? "var(--accent)" : "var(--surface-2)",
                color: theme === th ? "var(--accent-contrast)" : "var(--text)",
              }}
            >
              {t(`nav.theme.${th}`)}
            </button>
          ))}
        </div>
      </section>

      <p className="mt-6 text-xs" style={{ color: "var(--text-muted)" }}>
        {t("settings.about")} · manga-eroico v0.1.0 · AGPL-3.0
      </p>
    </div>
  );
}
