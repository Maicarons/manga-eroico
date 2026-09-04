import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, listenModelDownload, type HardwareReport, type ModelSpec } from "@/lib/tauri";

export default function ModelsPage() {
  const { t } = useTranslation();
  const [hw, setHw] = useState<HardwareReport | null>(null);
  const [models, setModels] = useState<ModelSpec[]>([]);
  const [progress, setProgress] = useState<Record<string, number>>({});

  useEffect(() => {
    void api.getHardwareInfo().then(setHw);
    void api.listModels().then(setModels);
    const un = listenModelDownload((ev) => {
      setProgress((p) => ({ ...p, [ev.id]: ev.percent }));
    });
    return un;
  }, []);

  const download = (id: string) => {
    void api.downloadModel(id);
  };

  const tierLabel = (tier: string) => t(`models.tier.${tier}`);

  return (
    <div data-testid="models-page">
      <h1 className="text-2xl font-bold">{t("models.title")}</h1>
      <p className="mb-6 mt-1 text-sm" style={{ color: "var(--text-muted)" }}>
        {t("models.subtitle")}
      </p>

      {hw && (
        <div
          className="mb-6 grid grid-cols-2 gap-4 rounded-xl border p-4 md:grid-cols-4"
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
      )}

      <div className="grid grid-cols-1 gap-3 md:grid-cols-2" data-testid="model-list">
        {models.map((m) => {
          const pct = progress[m.id];
          return (
            <div
              key={m.id}
              className="flex items-center gap-4 rounded-xl border p-4"
              style={{ borderColor: "var(--border)", background: "var(--surface)" }}
              data-testid={`model-${m.id}`}
            >
              <div className="flex-1">
                <div className="text-sm font-semibold">
                  {m.file}{" "}
                  <span className="ml-1 rounded px-1.5 py-0.5 text-[10px]" style={{ background: "var(--surface-2)" }}>
                    {t(`models.role.${m.role}`)}
                  </span>
                  <span className="ml-1 text-[10px]" style={{ color: "var(--text-muted)" }}>
                    {t(`models.lang.${m.lang}`)}
                  </span>
                </div>
                <div className="mt-1 text-xs" style={{ color: "var(--text-muted)" }}>
                  ModelScope: {m.modelscope_repo} · {t("models.size")} ~{m.size_mib} MiB
                </div>
                {pct != null && pct < 100 && (
                  <div className="mt-2 h-1.5 w-full overflow-hidden rounded-full" style={{ background: "var(--surface-2)" }}>
                    <div className="h-full rounded-full" style={{ width: `${pct}%`, background: "var(--accent)" }} />
                  </div>
                )}
              </div>
              <button
                onClick={() => download(m.id)}
                disabled={pct != null && pct < 100}
                className="rounded-lg px-3 py-2 text-xs font-semibold disabled:opacity-50"
                style={{ background: "var(--accent)", color: "var(--accent-contrast)" }}
                data-testid={`download-${m.id}`}
              >
                {pct != null && pct < 100
                  ? t("models.downloading", { percent: pct })
                  : pct === 100
                    ? t("models.verified")
                    : t("models.download")}
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function Stat({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div>
      <div className="text-xs" style={{ color: "var(--text-muted)" }}>
        {label}
      </div>
      <div className="mt-1 text-sm font-semibold" style={accent ? { color: "var(--accent)" } : undefined}>
        {value}
      </div>
    </div>
  );
}
