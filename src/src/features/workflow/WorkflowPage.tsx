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
