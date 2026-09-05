import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Background,
  Controls,
  ReactFlow,
  type Edge,
  type Node,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { api, listenPipeline, type PipelineEvent } from "@/lib/tauri";
import { useProjects } from "@/features/projects/projectsStore";

const STEPS = ["detect", "ocr", "inpaint", "translate", "polish", "render"] as const;
type Step = (typeof STEPS)[number];

const STEP_COLORS: Record<Step, string> = {
  detect: "#7c5cff",
  ocr: "#3b82f6",
  inpaint: "#06b6d4",
  translate: "#f59e0b",
  polish: "#ec4899",
  render: "#10b981",
};

interface NodeState {
  enabled: boolean;
  status: PipelineEvent["status"] | null;
}

function buildNodes(states: Record<Step, NodeState>, labels: Record<Step, string>): Node[] {
  return STEPS.map((step, i) => {
    const s = states[step];
    return {
      id: step,
      position: { x: i * 220, y: 0 },
      data: { label: `${labels[step]}\n${s.status ?? ""}` },
      style: {
        background: s.enabled ? STEP_COLORS[step] : "var(--surface-2)",
        color: s.enabled ? "#fff" : "var(--text-muted)",
        border: `2px solid ${s.status === "running" ? "var(--accent)" : "transparent"}`,
        borderRadius: 12,
        width: 170,
        opacity: s.enabled ? 1 : 0.55,
      },
    };
  });
}

const EDGES: Edge[] = STEPS.slice(1).map((step, i) => ({
  id: `${STEPS[i]}-${step}`,
  source: STEPS[i],
  target: step,
  animated: true,
}));

export default function WorkflowPage() {
  const { t } = useTranslation();
  const activeRoot = useProjects((s) => s.activeRoot);
  const [states, setStates] = useState<Record<Step, NodeState>>(() =>
    Object.fromEntries(
      STEPS.map((step) => [step, { enabled: step !== "polish", status: null }]),
    ) as Record<Step, NodeState>,
  );
  const [running, setRunning] = useState(false);
  const [events, setEvents] = useState<PipelineEvent[]>([]);
  const [polishPreview, setPolishPreview] = useState<{ analysis: string; items: Array<{ id: string; polished: string; note: string | null }> } | null>(null);
  const [adopted, setAdopted] = useState<Set<string>>(new Set());
  const [dismissed, setDismissed] = useState<Set<string>>(new Set());

  const labels = {
    detect: t("workflow.detect"),
    ocr: t("workflow.ocr"),
    inpaint: t("workflow.inpaint"),
    translate: t("workflow.translate"),
    polish: t("workflow.polish"),
    render: t("workflow.render"),
  } as Record<Step, string>;

  useEffect(() => {
    const un = listenPipeline((ev) => {
      setEvents((prev) => [...prev.slice(-30), ev]);
      setStates((prev) => {
        const step = ev.step as Step;
        if (!prev[step]) return prev;
        return { ...prev, [step]: { ...prev[step], status: ev.status } };
      });
    });
    return un;
  }, []);

  const toggle = useCallback(
    (step: Step) => {
      setStates((prev) => {
        const enabled = !prev[step].enabled;
        void api.setNodeEnabled(step, enabled);
        return { ...prev, [step]: { ...prev[step], enabled } };
      });
    },
    [],
  );

  const runPage = async () => {
    setRunning(true);
    try {
      await api.runPipelinePage("pg_demo");
    } finally {
      setRunning(false);
    }
  };

  return (
    <div data-testid="workflow-page" className="flex h-full flex-col">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">{t("workflow.title")}</h1>
          <p className="mt-1 text-sm" style={{ color: "var(--text-muted)" }}>
            {t("workflow.subtitle")}
          </p>
        </div>
        <div className="flex gap-2">
          <button
            data-testid="run-page"
            onClick={() => void runPage()}
            disabled={running}
            className="rounded-lg px-4 py-2 text-sm font-semibold disabled:opacity-50"
            style={{ background: "var(--accent)", color: "var(--accent-contrast)" }}
          >
            {running ? t("workflow.running") : `▶ ${t("workflow.runPage")}`}
          </button>
          <button
            data-testid="run-all"
            onClick={() => void api.runPipelineAll()}
            disabled={running || !activeRoot}
            className="cursor-pointer rounded-lg border px-4 py-2 text-sm font-medium disabled:opacity-50 transition-colors duration-200"
            style={{ borderColor: "var(--border)" }}
            title={t("workflow.runAll")}
          >
            {t("workflow.runAll")}
          </button>
        </div>
      </div>

      <div
        className="h-80 rounded-xl border"
        style={{ borderColor: "var(--border)", background: "var(--surface)" }}
        data-testid="workflow-canvas"
      >
        <ReactFlow nodes={buildNodes(states, labels)} edges={EDGES} fitView proOptions={{ hideAttribution: true }}>
          <Background />
          <Controls showInteractive={false} />
        </ReactFlow>
      </div>

      <div className="mt-4 grid grid-cols-2 gap-3 md:grid-cols-6" data-testid="node-toggles">
        {STEPS.map((step) => (
          <button
            key={step}
            data-testid={`toggle-${step}`}
            onClick={() => toggle(step)}
            className="rounded-lg border px-3 py-2 text-xs"
            style={{
              borderColor: "var(--border)",
              background: states[step].enabled ? STEP_COLORS[step] : "var(--surface-2)",
              color: states[step].enabled ? "#fff" : "var(--text-muted)",
            }}
          >
            <svg
              className="h-3 w-3"
              viewBox="0 0 12 12"
              aria-hidden
            >
              <circle cx="6" cy="6" r="4" fill={states[step].enabled ? "currentColor" : "none"} stroke="currentColor" strokeWidth="1.5" />
            </svg>
            {labels[step]}
          </button>
        ))}
      </div>

      <div className="mt-4 text-xs" style={{ color: "var(--text-muted)" }}>
        {activeRoot ?? "(no project)"} · {t("workflow.enableHint")} · {t("workflow.polishHint")}
      </div>

      <div
        className="mt-2 max-h-40 overflow-auto rounded-xl border p-3 font-mono text-xs"
        style={{ borderColor: "var(--border)", background: "var(--surface-2)" }}
        data-testid="event-log"
      >
        {events.length === 0 ? (
          <span style={{ color: "var(--text-muted)" }}>{t("workflow.events")}</span>
        ) : (
          events.map((ev, i) => (
            <div key={i}>
              [{ev.step}] {ev.status}
              {ev.progress != null ? ` ${ev.progress}%` : ""}
            </div>
          ))
        )}
      </div>

      {events.some((ev) => ev.step === "render" && ev.status === "completed") && (
        <div className="mt-3 flex flex-wrap items-center gap-3">
          <a
            href="#/editor"
            data-testid="open-editor"
            className="inline-block cursor-pointer rounded-lg px-4 py-2 text-sm font-semibold transition-opacity duration-200"
            style={{ background: "var(--accent)", color: "var(--accent-contrast)" }}
          >
            {t("workflow.openEditor")}
          </a>
          {states.polish.enabled && (
            <button
              onClick={async () => {
                const res = await api.polishPreview([
                  { id: "b001", page: 1, position: 1, source_text: "こんにちは", machine_translation: "你好" },
                  { id: "b002", page: 1, position: 2, source_text: "またね", machine_translation: "再见" },
                ]);
                setPolishPreview(res);
              }}
              className="cursor-pointer rounded-lg border px-4 py-2 text-sm font-medium transition-colors duration-200"
              style={{ borderColor: "var(--border)" }}
              data-testid="polish-preview"
            >
              {t("workflow.polishPreview")}
            </button>
          )}
        </div>
      )}

      {polishPreview && (
        <div
          className="mt-3 rounded-xl border p-4"
          style={{ borderColor: "var(--border)", background: "var(--surface)" }}
          data-testid="polish-diff"
        >
          <div className="mb-2 text-xs" style={{ color: "var(--text-muted)" }}>
            {polishPreview.analysis}
          </div>
          {polishPreview.items.map((item) => {
            const isAdopted = adopted.has(item.id);
            const isDismissed = dismissed.has(item.id);
            return (
              <div
                key={item.id}
                className="mb-2 flex items-center justify-between gap-4 rounded-lg border p-2 text-sm"
                style={{ borderColor: "var(--border)", opacity: isDismissed ? 0.5 : 1 }}
                data-testid={`diff-${item.id}`}
              >
                <div className="min-w-0 flex-1">
                  <span style={{ color: "var(--text-muted)" }}>{item.id}</span>{" "}
                  <s style={{ opacity: isAdopted ? 0.5 : 1 }}>{item.polished.replace(/^[^\w　-鿿]*/, "")}</s>{" "}
                  <span style={{ color: isAdopted ? "var(--ok)" : "var(--accent)", fontWeight: 600 }}>{item.polished}</span>
                </div>
                <div className="flex shrink-0 gap-2 text-xs">
                  <button
                    onClick={() => {
                      setAdopted((s0) => new Set(s0).add(item.id));
                      setDismissed((s0) => { const n = new Set(s0); n.delete(item.id); return n; });
                    }}
                    className="cursor-pointer rounded-md border px-2 py-1 transition-colors duration-200"
                    style={{ borderColor: isAdopted ? "var(--ok)" : "var(--border)", color: isAdopted ? "var(--ok)" : "var(--text)" }}
                    data-testid={`adopt-${item.id}`}
                  >
                    {t("workflow.adopt")}
                  </button>
                  <button
                    onClick={() => {
                      setDismissed((s0) => new Set(s0).add(item.id));
                      setAdopted((s0) => { const n = new Set(s0); n.delete(item.id); return n; });
                    }}
                    className="cursor-pointer rounded-md border px-2 py-1 transition-colors duration-200"
                    style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}
                    data-testid={`dismiss-${item.id}`}
                  >
                    {t("workflow.dismiss")}
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
