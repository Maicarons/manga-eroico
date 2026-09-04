import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { useProjects } from "./projectsStore";
import { api } from "@/lib/tauri";

const LANGS = [
  { code: "ja", label: "日本語" },
  { code: "en", label: "English" },
  { code: "zh", label: "简体中文" },
  { code: "ko", label: "한국어" },
] as const;

export default function ProjectsPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { projects, upsert, setActive } = useProjects();
  const [creating, setCreating] = useState(false);
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
    setActive(root);
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
              <select
                className="ml-1 rounded border bg-transparent px-2 py-1"
                style={{ borderColor: "var(--border)", color: "var(--text)" }}
                value={source}
                onChange={(e) => setSource(e.target.value)}
              >
                {LANGS.map((l) => (
                  <option key={l.code} value={l.code}>
                    {l.label}
                  </option>
                ))}
              </select>
            </label>
            <label className="text-sm">
              {t("projects.targetLang")}{" "}
              <select
                className="ml-1 rounded border bg-transparent px-2 py-1"
                style={{ borderColor: "var(--border)", color: "var(--text)" }}
                value={target}
                onChange={(e) => setTarget(e.target.value)}
              >
                {LANGS.map((l) => (
                  <option key={l.code} value={l.code}>
                    {l.label}
                  </option>
                ))}
              </select>
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
              className="cursor-pointer rounded-xl border p-4 transition-transform hover:scale-[1.01]"
              style={{ borderColor: "var(--border)", background: "var(--surface)" }}
              onClick={() => {
                setActive(p.root);
                navigate("/workflow");
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
    </div>
  );
}
