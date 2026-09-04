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
