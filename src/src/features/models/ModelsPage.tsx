import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, listenModelDownload, type HardwareReport, type ModelSpec } from "@/lib/tauri";

const STEPS = ["detect", "recommend", "select", "download"] as const;
type Step = (typeof STEPS)[number];

/** Per-tier recommended LLM spec id (mirrors model-manager::registry). */
const TIER_LLM: Record<string, string> = {
  lite: "hymt2_1.8b_iq2_m",
  standard: "hymt2_7b_q4",
  pro: "hymt2_30b_a3b_q4",
};
/** Core OCR set every user needs. */
const CORE_IDS = ["ppocrv5_det", "ppocrv5_cls", "rec_mixed", "dict_mixed"];

export default function ModelsPage() {
  const { t } = useTranslation();
  const [step, setStep] = useState<Step>("detect");
  const [hw, setHw] = useState<HardwareReport | null>(null);
  const [models, setModels] = useState<ModelSpec[]>([]);
  const [progress, setProgress] = useState<Record<string, number>>({});
  const [selected, setSelected] = useState<Set<string>>(new Set());

  useEffect(() => {
    void api.getHardwareInfo().then(setHw);
    void api.listModels().then(setModels);
    const un = listenModelDownload((ev) => {
      setProgress((p) => ({ ...p, [ev.id]: ev.percent }));
    });
    return un;
  }, []);

  // recommended selection once hardware is known
  useEffect(() => {
    if (!hw || models.length === 0) return;
    setSelected(new Set([...CORE_IDS, TIER_LLM[hw.tier]].filter(Boolean)));
  }, [hw, models]);

  const recommendedLlm = hw ? TIER_LLM[hw.tier] : undefined;
  const tierLabel = (tier: string) => t(`models.tier.${tier}`);

  const chosen = useMemo(
    () => models.filter((m) => selected.has(m.id)),
    [models, selected],
  );
  const allDone = chosen.length > 0 && chosen.every((m) => progress[m.id] === 100);
  const anyActive = chosen.some((m) => progress[m.id] != null && progress[m.id] < 100);

  const toggle = (id: string) =>
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const startDownloads = () => {
    chosen.forEach((m) => {
      if (progress[m.id] == null || progress[m.id] < 100) void api.downloadModel(m.id);
    });
  };

  const stepIndex = STEPS.indexOf(step);

  return (
    <div data-testid="models-page">
      <h1 className="text-2xl font-bold">{t("models.title")}</h1>
      <p className="mb-4 mt-1 text-sm" style={{ color: "var(--text-muted)" }}>
        {t("models.subtitle")}
      </p>

      {/* stepper: "Step N of M" + clickable history */}
      <ol className="mb-6 flex flex-wrap items-center gap-2" aria-label="wizard steps">
        {STEPS.map((s, i) => (
          <li key={s} className="flex items-center gap-2">
            <button
              onClick={() => i < stepIndex && setStep(s)}
              disabled={i > stepIndex}
              className={`flex cursor-pointer items-center gap-1.5 rounded-full border px-3 py-1 text-xs font-medium transition-colors duration-200 disabled:cursor-default ${
                i === stepIndex
                  ? "border-transparent"
                  : i < stepIndex
                    ? ""
                    : "opacity-50"
              }`}
              style={{
                background: i === stepIndex ? "var(--accent)" : i < stepIndex ? "var(--ok)" : "var(--surface-2)",
                color: i === stepIndex ? "var(--accent-contrast)" : "var(--text)",
                borderColor: i === stepIndex ? "transparent" : "var(--border)",
              }}
              aria-current={i === stepIndex ? "step" : undefined}
            >
              <StepIcon name={s} done={i < stepIndex} />
              {t(`models.wizard.step.${s}`)}
            </button>
            {i < STEPS.length - 1 && <span aria-hidden className="text-xs" style={{ color: "var(--text-muted)" }}>→</span>}
          </li>
        ))}
      </ol>

      {step === "detect" && (
        <section aria-label={t("models.wizard.step.detect")}>
          {hw ? (
            <>
              <div
                className="mb-4 grid grid-cols-2 gap-4 rounded-xl border p-4 md:grid-cols-4"
                style={{ borderColor: "var(--border)", background: "var(--surface)" }}
                data-testid="hardware-card"
              >
                <Stat label={t("models.cpuCores")} value={String(hw.info.cpu_cores)} />
                <Stat label={t("models.ram")} value={`${(hw.info.total_ram_mib / 1024).toFixed(0)} GB`} />
                <Stat
                  label={t("models.vram")}
                  value={
                    hw.info.gpus.length > 0 && hw.info.gpus[0].vram_mib
                      ? `${(hw.info.gpus[0].vram_mib / 1024).toFixed(0)} GB · ${hw.info.gpus[0].name}`
                      : "—"
                  }
                />
                <Stat label={t("models.recommendedTier")} value={tierLabel(hw.tier)} accent />
              </div>
              <WizardNext onClick={() => setStep("recommend")} label={t("models.wizard.next")} />
            </>
          ) : (
            <p className="text-sm" style={{ color: "var(--text-muted)" }}>
              {t("models.detecting")}
            </p>
          )}
        </section>
      )}

      {step === "recommend" && hw && (
        <section aria-label={t("models.wizard.step.recommend")}>
          <div className="mb-4 rounded-xl border p-4" style={{ borderColor: "var(--border)", background: "var(--surface)" }}>
            <p className="text-sm">
              {t("models.wizard.recommendIntro", { tier: tierLabel(hw.tier) })}
            </p>
            <ul className="mt-3 space-y-1 text-sm">
              {CORE_IDS.map((id) => (
                <li key={id} className="flex items-center gap-2">
                  <CheckIcon /> {t(`models.role.${models.find((m) => m.id === id)?.role ?? "detect"}`)}
                  <span className="text-xs" style={{ color: "var(--text-muted)" }}>
                    {models.find((m) => m.id === id)?.file}
                  </span>
                </li>
              ))}
              {recommendedLlm && (
                <li className="flex items-center gap-2">
                  <CheckIcon /> {t("models.role.llm")}
                  <span className="text-xs" style={{ color: "var(--text-muted)" }}>
                    {models.find((m) => m.id === recommendedLlm)?.file}
                  </span>
                </li>
              )}
            </ul>
          </div>
          <div className="flex gap-3">
            <button
              onClick={() => setStep("detect")}
              className="cursor-pointer rounded-lg border px-4 py-2 text-sm font-medium transition-colors duration-200"
              style={{ borderColor: "var(--border)", color: "var(--text)" }}
            >
              {t("models.wizard.back")}
            </button>
            <WizardNext onClick={() => setStep("select")} label={t("models.wizard.next")} />
          </div>
        </section>
      )}

      {step === "select" && (
        <section aria-label={t("models.wizard.step.select")}>
          <div className="grid grid-cols-1 gap-3 md:grid-cols-2" data-testid="model-list">
            {models.map((m) => {
              const pct = progress[m.id];
              const done = pct === 100;
              const core = CORE_IDS.includes(m.id);
              return (
                <label
                  key={m.id}
                  className={`flex cursor-pointer items-center gap-4 rounded-xl border p-4 transition-colors duration-200 ${done ? "opacity-70" : ""}`}
                  style={{
                    borderColor: selected.has(m.id) ? "var(--accent)" : "var(--border)",
                    background: "var(--surface)",
                  }}
                  data-testid={`model-${m.id}`}
                >
                  <input
                    type="checkbox"
                    checked={selected.has(m.id)}
                    onChange={() => toggle(m.id)}
                    disabled={core || done || (pct != null && pct < 100)}
                    className="h-4 w-4"
                    aria-label={`${m.id}`}
                  />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm font-semibold">
                      {m.file}{" "}
                      <span className="ml-1 rounded px-1.5 py-0.5 text-[10px]" style={{ background: "var(--surface-2)" }}>
                        {t(`models.role.${m.role}`)}
                      </span>
                      <span className="ml-1 text-[10px]" style={{ color: "var(--text-muted)" }}>
                        {t(`models.lang.${m.lang}`)}
                      </span>
                      {core && (
                        <span className="ml-1 rounded px-1.5 py-0.5 text-[10px]" style={{ background: "var(--surface-2)" }}>
                          {t("models.wizard.core")}
                        </span>
                      )}
                    </div>
                    <div className="mt-1 text-xs" style={{ color: "var(--text-muted)" }}>
                      {m.modelscope_repo} · {t("models.size")} ~{m.size_mib} MiB
                    </div>
                    {pct != null && pct < 100 && (
                      <div className="mt-2 h-1.5 w-full overflow-hidden rounded-full" style={{ background: "var(--surface-2)" }} role="progressbar" aria-valuenow={pct}>
                        <div className="h-full rounded-full transition-all duration-200" style={{ width: `${pct}%`, background: "var(--accent)" }} />
                      </div>
                    )}
                  </div>
                </label>
              );
            })}
          </div>
          <div className="mt-4 flex gap-3">
            <button
              onClick={() => setStep("recommend")}
              className="cursor-pointer rounded-lg border px-4 py-2 text-sm font-medium transition-colors duration-200"
              style={{ borderColor: "var(--border)", color: "var(--text)" }}
            >
              {t("models.wizard.back")}
            </button>
            <button
              onClick={() => {
                setStep("download");
                startDownloads();
              }}
              disabled={selected.size === 0}
              className="cursor-pointer rounded-lg px-4 py-2 text-sm font-semibold transition-opacity duration-200 disabled:cursor-default disabled:opacity-50"
              style={{ background: "var(--accent)", color: "var(--accent-contrast)" }}
              data-testid="start-download"
            >
              {t("models.wizard.startDownload", { count: selected.size })}
            </button>
          </div>
        </section>
      )}

      {step === "download" && (
        <section aria-label={t("models.wizard.step.download")}>
          {allDone ? (
            <div
              className="mb-4 flex items-center gap-3 rounded-xl border p-4"
              style={{ borderColor: "var(--ok)", background: "var(--surface)" }}
              data-testid="download-complete"
            >
              <CheckIcon big />
              <div>
                <div className="text-sm font-semibold">{t("models.wizard.allVerified")}</div>
                <div className="text-xs" style={{ color: "var(--text-muted)" }}>
                  {t("models.wizard.allVerifiedHint")}
                </div>
              </div>
            </div>
          ) : (
            !anyActive && (
              <div className="mb-4 flex gap-3">
                <button
                  onClick={startDownloads}
                  className="cursor-pointer rounded-lg px-4 py-2 text-sm font-semibold"
                  style={{ background: "var(--accent)", color: "var(--accent-contrast)" }}
                >
                  {t("models.wizard.resume")}
                </button>
              </div>
            )
          )}
          <ul className="space-y-2">
            {chosen.map((m) => {
              const pct = progress[m.id] ?? 0;
              return (
                <li
                  key={m.id}
                  className="flex items-center gap-4 rounded-xl border p-4"
                  style={{ borderColor: "var(--border)", background: "var(--surface)" }}
                  data-testid={`dl-${m.id}`}
                >
                  <StatusIcon pct={pct} />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm font-semibold">{m.file}</div>
                    <div className="mt-1.5 h-1.5 w-full overflow-hidden rounded-full" style={{ background: "var(--surface-2)" }} role="progressbar" aria-valuenow={pct}>
                      <div className="h-full rounded-full transition-all duration-200" style={{ width: `${pct}%`, background: pct === 100 ? "var(--ok)" : "var(--accent)" }} />
                    </div>
                  </div>
                  <div className="text-xs tabular-nums" style={{ color: "var(--text-muted)" }}>
                    {pct === 100 ? t("models.verified") : t("models.downloading", { percent: pct })}
                  </div>
                </li>
              );
            })}
          </ul>
        </section>
      )}
    </div>
  );
}

function WizardNext({ onClick, label }: { onClick: () => void; label: string }) {
  return (
    <button
      onClick={onClick}
      className="cursor-pointer rounded-lg px-4 py-2 text-sm font-semibold transition-opacity duration-200"
      style={{ background: "var(--accent)", color: "var(--accent-contrast)" }}
    >
      {label}
    </button>
  );
}

function Stat({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div>
      <div className="text-xs" style={{ color: "var(--text-muted)" }}>{label}</div>
      <div className="mt-1 text-sm font-semibold" style={accent ? { color: "var(--accent)" } : undefined}>
        {value}
      </div>
    </div>
  );
}

function StepIcon({ name, done }: { name: Step; done: boolean }) {
  const cls = "h-3.5 w-3.5";
  if (done)
    return (
      <svg className={cls} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" aria-hidden>
        <path d="M5 13l4 4L19 7" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    );
  switch (name) {
    case "detect":
      return (
        <svg className={cls} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
          <circle cx="11" cy="11" r="7" />
          <path d="M21 21l-4.3-4.3" strokeLinecap="round" />
        </svg>
      );
    case "recommend":
      return (
        <svg className={cls} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
          <path d="M13 2L3 14h7l-1 8 10-12h-7l1-8z" strokeLinejoin="round" />
        </svg>
      );
    case "select":
      return (
        <svg className={cls} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
          <rect x="3" y="3" width="18" height="18" rx="4" />
          <path d="M8 12l3 3 5-6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      );
    case "download":
      return (
        <svg className={cls} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
          <path d="M12 3v12m0 0l-4-4m4 4l4-4" strokeLinecap="round" strokeLinejoin="round" />
          <path d="M4 17v2a2 2 0 002 2h12a2 2 0 002-2v-2" strokeLinecap="round" />
        </svg>
      );
  }
}

function CheckIcon({ big }: { big?: boolean }) {
  return (
    <svg
      className={big ? "h-8 w-8" : "h-4 w-4"}
      viewBox="0 0 24 24"
      fill="none"
      stroke="var(--ok)"
      strokeWidth="3"
      aria-hidden
    >
      <path d="M5 13l4 4L19 7" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function StatusIcon({ pct }: { pct: number }) {
  if (pct === 100) return <CheckIcon />;
  return (
    <svg className="h-5 w-5 animate-spin" viewBox="0 0 24 24" fill="none" aria-hidden>
      <circle cx="12" cy="12" r="9" stroke="var(--surface-2)" strokeWidth="3" />
      <path d="M21 12a9 9 0 00-9-9" stroke="var(--accent)" strokeWidth="3" strokeLinecap="round" />
    </svg>
  );
}
