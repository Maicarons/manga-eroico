/**
 * Thin Tauri IPC wrapper. When the app runs in a plain browser (web
 * preview / Playwright), every call falls back to deterministic mocks so
 * the full UI is testable without the Rust host.
 */

interface TauriGlobal {
  __TAURI_INTERNALS__?: unknown;
}

export function isTauri(): boolean {
  return typeof window !== "undefined" && !!(window as unknown as TauriGlobal).__TAURI_INTERNALS__;
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const mod = await import("@tauri-apps/api/core");
    return mod.invoke<T>(cmd, args);
  }
  return mockInvoke<T>(cmd, args);
}

// ---- mocks (web preview) ----

const mockHardware = {
  info: {
    cpu_cores: 8,
    total_ram_mib: 32768,
    gpus: [{ name: "Mock GPU (web preview)", vram_mib: 8192 }],
    free_disk_mib: 102400,
  },
  tier: "standard",
};

const mockModels = [
  { id: "ppocrv5_det", role: "detect", lang: "any", modelscope_repo: "RapidAI/RapidOCR", file: "ch_PP-OCRv5_mobile_det.onnx", size_mib: 5, sha256: "" },
  { id: "rec_ja", role: "rec", lang: "ja", modelscope_repo: "RapidAI/RapidOCR", file: "japan_PP-OCRv5_rec_mobile_infer.onnx", size_mib: 11, sha256: "" },
  { id: "hymt2_1.8b_q4", role: "llm", lang: "any", modelscope_repo: "Tencent-Hunyuan/Hy-MT2", file: "hy-mt2-1.8b-q4_0.gguf", size_mib: 1100, sha256: "" },
];

const mockProject = {
  schema_version: 1,
  name: "Mock Project",
  source_lang: { ja: "ja" },
  target_lang: "zh",
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString(),
  pipeline: { configs: [] },
  pages: [],
  chapters: [],
  glossary: {},
};

let mockProjectName = "Mock Project";

async function mockInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  switch (cmd) {
    case "greet":
      return `Hello, ${args?.name}! (mock)` as T;
    case "get_hardware_info":
      return mockHardware as T;
    case "decide_tier":
      return "standard" as T;
    case "list_models":
      return mockModels as T;
    case "get_llm_for_tier":
      return "hymt2_7b_q4" as T;
    case "download_model":
      // fake a download with progress
      for (const pct of [25, 50, 75, 100]) {
        window.dispatchEvent(
          new CustomEvent("mock-model-download", { detail: { id: args?.spec_id, percent: pct } }),
        );
        await new Promise((r) => setTimeout(r, 150));
      }
      return `C:\\mock\\models\\${args?.spec_id}.onnx` as T;
    case "create_project":
      mockProjectName = String(args?.name ?? "Mock Project");
      return { ...mockProject, name: mockProjectName } as T;
    case "open_project":
      return { ...mockProject, name: mockProjectName } as T;
    case "save_project":
      return undefined as T;
    case "add_page":
      return "pg_mock0001" as T;
    case "add_chapter":
      return "ch_mock0001" as T;
    case "set_glossary_term":
    case "set_node_enabled":
      return undefined as T;
    case "run_pipeline_page": {
      // emit the same event sequence the Rust host would
      const steps = ["detect", "ocr", "inpaint", "translate", "polish", "render"];
      for (const step of steps) {
        emitPipeline({ page: { 0: args?.page_id }, step, status: "running", progress: null, message: null });
        await new Promise((r) => setTimeout(r, 120));
        emitPipeline({ page: { 0: args?.page_id }, step, status: "completed", progress: 100, message: null });
      }
      return true as T;
    }
    case "polish_preview":
      return {
        analysis: "Mock analysis: tone is light-hearted, names consistent.",
        items: (args?.bubbles as Array<{ id: string; machine_translation: string }>)?.map((b) => ({
          id: b.id,
          polished: `✨ ${b.machine_translation}`,
          note: null,
        })),
      } as T;
    default:
      throw new Error(`mock: unknown command ${cmd}`);
  }
}

function emitPipeline(ev: unknown) {
  window.dispatchEvent(new CustomEvent("mock-pipeline-event", { detail: ev }));
}

// ---- typed API surface ----

export interface HardwareReport {
  info: {
    cpu_cores: number;
    total_ram_mib: number;
    gpus: Array<{ name: string; vram_mib: number | null }>;
    free_disk_mib: number;
  };
  tier: "lite" | "standard" | "pro";
}

export interface ModelSpec {
  id: string;
  role: "detect" | "cls" | "rec" | "dict" | "inpaint" | "llm";
  lang: "zh" | "en" | "ja" | "ko" | "any";
  modelscope_repo: string;
  file: string;
  size_mib: number;
  sha256: string;
}

export interface PipelineEvent {
  page: { 0: string };
  step: "detect" | "ocr" | "inpaint" | "translate" | "polish" | "render";
  status: "pending" | "running" | "completed" | "skipped" | "failed";
  progress: number | null;
  message: string | null;
}

export interface PolishResult {
  analysis: string;
  items: Array<{ id: string; polished: string; note: string | null }>;
}

export const api = {
  greet: (name: string) => invoke<string>("greet", { name }),
  getHardwareInfo: () => invoke<HardwareReport>("get_hardware_info"),
  listModels: () => invoke<ModelSpec[]>("list_models"),
  downloadModel: (specId: string, destDir = "models") =>
    invoke<string>("download_model", { specId, destDir }),
  createProject: (root: string, name: string, sourceLang: string, targetLang: string) =>
    invoke("create_project", { root, name, sourceLang, targetLang }),
  setNodeEnabled: (node: string, enabled: boolean) =>
    invoke<void>("set_node_enabled", { node, enabled }),
  runPipelinePage: (pageId: string) => invoke<boolean>("run_pipeline_page", { pageId }),
  polishPreview: (
    bubbles: Array<{
      id: string;
      page: number;
      position: number;
      source_text: string;
      machine_translation: string;
    }>,
  ) => invoke<PolishResult>("polish_preview", { bubbles }),
};

/** Unified pipeline/model event subscription for both real & mock backends. */
export function listenPipeline(cb: (ev: PipelineEvent) => void): () => void {
  if (isTauri()) {
    let unlisten: (() => void) | null = null;
    import("@tauri-apps/api/event").then(({ listen }) =>
      listen<PipelineEvent>("pipeline-event", (e) => cb(e.payload)).then((u) => (unlisten = u)),
    );
    return () => unlisten?.();
  }
  const handler = (e: Event) => cb((e as CustomEvent).detail);
  window.addEventListener("mock-pipeline-event", handler);
  return () => window.removeEventListener("mock-pipeline-event", handler);
}

export function listenModelDownload(cb: (ev: { id: string; percent: number }) => void): () => void {
  if (isTauri()) {
    let unlisten: (() => void) | null = null;
    import("@tauri-apps/api/event").then(({ listen }) =>
      listen<{ id: string; percent: number }>("model-download", (e) => cb(e.payload)).then(
        (u) => (unlisten = u),
      ),
    );
    return () => unlisten?.();
  }
  const handler = (e: Event) => cb((e as CustomEvent).detail);
  window.addEventListener("mock-model-download", handler);
  return () => window.removeEventListener("mock-model-download", handler);
}
