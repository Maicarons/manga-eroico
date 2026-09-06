import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { useProjects } from "./projectsStore";
import { api } from "@/lib/tauri";

interface Overview {
  name: string;
  nodes: string[];
  chapters: Array<{
    title: string;
    pages: Array<{ id: string; file: string; nodes: Record<string, boolean> }>;
  }>;
}

// Source languages: limited by what PP-OCR recognizers can read (script
// families mapped in me-ocr OcrLang::for_source). Languages without a
// dedicated recognizer (km/my/bo) fall back to the mixed model in-engine.
const SOURCE_LANGS = [
  { group: "CJK", items: [
    { code: "ja", label: "日本語" },
    { code: "zh", label: "简体中文" },
    { code: "zh-Hant", label: "繁體中文" },
    { code: "yue", label: "粵語" },
    { code: "ko", label: "한국어" },
    { code: "en", label: "English" },
  ]},
  { group: "Latin / Cyrillic", items: [
    { code: "fr", label: "Français" }, { code: "de", label: "Deutsch" },
    { code: "es", label: "Español" }, { code: "pt", label: "Português" },
    { code: "it", label: "Italiano" }, { code: "nl", label: "Nederlands" },
    { code: "pl", label: "Polski" }, { code: "cs", label: "Čeština" },
    { code: "vi", label: "Tiếng Việt" }, { code: "tr", label: "Türkçe" },
    { code: "id", label: "Bahasa Indonesia" }, { code: "ms", label: "Bahasa Melayu" },
    { code: "tl", label: "Filipino" },
    { code: "ru", label: "Русский" }, { code: "uk", label: "Українська" },
    { code: "kk", label: "Қазақша" }, { code: "mn", label: "Монгол" }, { code: "ug", label: "ئۇيغۇرچە" },
  ]},
  { group: "Other scripts", items: [
    { code: "ar", label: "العربية" }, { code: "fa", label: "فارسی" }, { code: "ur", label: "اردو" }, { code: "he", label: "עברית" },
    { code: "hi", label: "हिन्दी" }, { code: "mr", label: "मराठी" }, { code: "gu", label: "ગુજરાતી" }, { code: "bn", label: "বাংলা" },
    { code: "ta", label: "தமிழ்" }, { code: "te", label: "తెలుగు" }, { code: "th", label: "ไทย" },
  ]},
] as const;

// Target languages: Hy-MT2 translates any pair of these 38 languages.
const TARGET_LANGS = [
  { group: "中文系", items: [
    { code: "zh", label: "简体中文" }, { code: "zh-Hant", label: "繁體中文" }, { code: "yue", label: "粵語" },
  ]},
  { group: "亚洲语言", items: [
    { code: "ja", label: "日本語" }, { code: "ko", label: "한국어" }, { code: "th", label: "ไทย" },
    { code: "vi", label: "Tiếng Việt" }, { code: "id", label: "Indonesia" }, { code: "ms", label: "Melayu" },
    { code: "tl", label: "Filipino" }, { code: "km", label: "ខ្មែរ" }, { code: "my", label: "မြန်မာ" },
  ]},
  { group: "欧洲语言", items: [
    { code: "en", label: "English" }, { code: "fr", label: "Français" }, { code: "de", label: "Deutsch" },
    { code: "es", label: "Español" }, { code: "pt", label: "Português" }, { code: "it", label: "Italiano" },
    { code: "nl", label: "Nederlands" }, { code: "pl", label: "Polski" }, { code: "cs", label: "Čeština" },
    { code: "ru", label: "Русский" }, { code: "uk", label: "Українська" },
  ]},
  { group: "中东 & 南亚", items: [
    { code: "ar", label: "العربية" }, { code: "he", label: "עברית" }, { code: "fa", label: "فارسی" },
    { code: "tr", label: "Türkçe" }, { code: "hi", label: "हिन्दी" }, { code: "bn", label: "বাংলা" },
    { code: "ta", label: "தமிழ்" }, { code: "te", label: "తెలుగు" }, { code: "mr", label: "मराठी" },
    { code: "gu", label: "ગુજરાતી" }, { code: "ur", label: "اردو" },
  ]},
  { group: "民族语言", items: [
    { code: "bo", label: "བོད་སྐད་" }, { code: "kk", label: "Қазақша" },
    { code: "mn", label: "Монгол" }, { code: "ug", label: "ئۇيغۇرچە" },
  ]},
] as const;

function LangSelect({
  value,
  onChange,
  groups,
  testid,
}: {
  value: string;
  onChange: (v: string) => void;
  groups: ReadonlyArray<{ group: string; items: ReadonlyArray<{ code: string; label: string }> }>;
  testid: string;
}) {
  return (
    <select
      data-testid={testid}
      className="ml-1 rounded border bg-transparent px-2 py-1"
      style={{ borderColor: "var(--border)", color: "var(--text)" }}
      value={value}
      onChange={(e) => onChange(e.target.value)}
    >
      {groups.map((g) => (
        <optgroup key={g.group} label={g.group}>
          {g.items.map((l) => (
            <option key={l.code} value={l.code}>
              {l.label}
            </option>
          ))}
        </optgroup>
      ))}
    </select>
  );
}

export default function ProjectsPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { projects, upsert, activate } = useProjects();
  const [creating, setCreating] = useState(false);
  const [overview, setOverview] = useState<Overview | null>(null);
  const [name, setName] = useState("");
  const [source, setSource] = useState("ja");
  const [target, setTarget] = useState("zh");

  const create = async () => {
    if (!name.trim()) return;
    const root = `./${name.trim().replace(/[\\/:*?"<>|]/g, "_")}.mepro`;
    await api.createProject(root, name.trim(), source, target);
    upsert({
      root,
      name: name.trim(),
      sourceLang: source,
      targetLang: target,
      pages: 0,
      chapters: 0,
      lastOpened: new Date().toISOString(),
    });
    await activate(root);
    setCreating(false);
    setName("");
    navigate("/workflow");
  };

  return (
    <div data-testid="projects-page">
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">{t("projects.title")}</h1>
          <p className="mt-1 text-sm" style={{ color: "var(--text-muted)" }}>
            {t("projects.subtitle")}
          </p>
        </div>
        <button
          data-testid="new-project"
          onClick={() => setCreating(true)}
          className="rounded-lg px-4 py-2 text-sm font-semibold"
          style={{ background: "var(--accent)", color: "var(--accent-contrast)" }}
        >
          ＋ {t("projects.newProject")}
        </button>
      </div>

      {creating && (
        <div
          className="mb-6 rounded-xl border p-4"
          style={{ borderColor: "var(--border)", background: "var(--surface)" }}
          data-testid="create-wizard"
        >
          <input
            data-testid="project-name"
            className="mb-3 w-full rounded-lg border px-3 py-2 text-sm"
            style={{ borderColor: "var(--border)", background: "var(--surface-2)", color: "var(--text)" }}
            placeholder={t("projects.namePlaceholder")}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <div className="mb-3 flex gap-4">
            <label className="text-sm">
              {t("projects.sourceLang")}{" "}
              <LangSelect value={source} onChange={setSource} groups={SOURCE_LANGS} testid="source-lang" />
            </label>
            <label className="text-sm">
              {t("projects.targetLang")}{" "}
              <LangSelect value={target} onChange={setTarget} groups={TARGET_LANGS} testid="target-lang" />
            </label>
          </div>
          <p className="mb-3 text-xs" style={{ color: "var(--text-muted)" }}>
            {t("projects.createWizard.autoDetect")}
          </p>
          <div className="flex gap-2">
            <button
              data-testid="create-confirm"
              onClick={() => void create()}
              className="rounded-lg px-4 py-2 text-sm font-semibold"
              style={{ background: "var(--accent)", color: "var(--accent-contrast)" }}
            >
              {t("common.confirm")}
            </button>
            <button
              onClick={() => setCreating(false)}
              className="rounded-lg px-4 py-2 text-sm"
              style={{ background: "var(--surface-2)" }}
            >
              {t("common.cancel")}
            </button>
          </div>
        </div>
      )}

      {projects.length === 0 && !creating ? (
        <div
          className="rounded-xl border border-dashed p-10 text-center text-sm"
          style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}
          data-testid="projects-empty"
        >
          {t("projects.empty")}
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
          {projects.map((p) => (
            <div
              key={p.root}
              data-testid="project-card"
              className="cursor-pointer rounded-xl border p-4 transition-colors duration-200 hover:shadow-md"
              style={{ borderColor: "var(--border)", background: "var(--surface)" }}
              onClick={async () => {
                await activate(p.root);
                const ov = await api.getProjectOverview();
                setOverview(ov ?? null);
              }}
            >
              <div className="text-sm font-semibold">{p.name}</div>
              <div className="mt-1 text-xs" style={{ color: "var(--text-muted)" }}>
                {p.sourceLang.toUpperCase()} → {p.targetLang.toUpperCase()} · {t("projects.pages")} {p.pages} ·{" "}
                {t("projects.chapters")} {p.chapters}
              </div>
            </div>
          ))}
        </div>
      )}

      {overview && (
        <div
          className="mt-6 rounded-xl border p-4"
          style={{ borderColor: "var(--border)", background: "var(--surface)" }}
          data-testid="project-overview"
        >
          <div className="mb-3 flex items-center justify-between">
            <div className="text-sm font-semibold">
              {t("projects.overview")} · {overview.name}
            </div>
            <div className="flex gap-2">
              <button
                onClick={() => navigate("/workflow")}
                className="cursor-pointer rounded-lg border px-3 py-1.5 text-xs transition-colors duration-200"
                style={{ borderColor: "var(--border)" }}
              >
                {t("projects.openWorkflow")}
              </button>
              <button
                onClick={() => setOverview(null)}
                className="cursor-pointer rounded-lg border px-3 py-1.5 text-xs transition-colors duration-200"
                style={{ borderColor: "var(--border)" }}
              >
                <svg className="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
                  <path d="M6 6l12 12M18 6L6 18" strokeLinecap="round" />
                </svg>
              </button>
            </div>
          </div>
          {overview.chapters.map((ch) => (
            <div key={ch.title} className="mb-3">
              <div className="mb-1 text-xs font-semibold">📁 {ch.title}</div>
              <table className="w-full text-left text-xs" data-testid="progress-matrix">
                <thead>
                  <tr style={{ color: "var(--text-muted)" }}>
                    <th className="py-1 pr-2">{t("projects.page")}</th>
                    {overview.nodes.map((n) => (
                      <th key={n} className="py-1 pr-2">
                        {t(`models.role.${n === "detect" ? "detect" : n}`)}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {ch.pages.map((pg) => (
                    <tr key={pg.id} data-testid={`row-${pg.id}`}>
                      <td className="py-1 pr-2 font-mono">{pg.file}</td>
                      {overview.nodes.map((n) => (
                        <td key={n} className="py-1 pr-2">
                          {pg.nodes[n] ? (
                            <span style={{ color: "var(--ok)" }}>●</span>
                          ) : (
                            <span style={{ color: "var(--text-muted)" }}>○</span>
                          )}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
