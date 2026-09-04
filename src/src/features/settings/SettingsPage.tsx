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
