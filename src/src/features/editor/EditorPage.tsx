import { useEffect, useRef, useState } from "react";
import { Stage, Layer, Rect, Text, Image as KonvaImage, Group } from "react-konva";
import { useTranslation } from "react-i18next";
import { api } from "@/lib/tauri";

interface Bubble {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  text: string;
  vertical: boolean;
  fontSize: number;
}

const INITIAL_BUBBLES: Bubble[] = [
  { id: "b1", x: 60, y: 40, w: 220, h: 90, text: "こんにちは、世界！", vertical: false, fontSize: 18 },
  { id: "b2", x: 320, y: 160, w: 90, h: 200, text: "またね、またね", vertical: true, fontSize: 18 },
];

const CANVAS_W = 620;
const CANVAS_H = 420;
const STORAGE_KEY = "manga-eroico.editor.bubbles.v1";

export default function EditorPage() {
  const { t } = useTranslation();
  const stageRef = useRef<any>(null);
  const [bubbles, setBubbles] = useState<Bubble[]>(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) return JSON.parse(saved) as Bubble[];
    } catch {
      /* ignore */
    }
    return INITIAL_BUBBLES;
  });
  const [selected, setSelected] = useState<string | null>("b1");
  const [pageImage, setPageImage] = useState<HTMLImageElement | null>(null);
  const [pageInfo, setPageInfo] = useState<string>("");
  const [exported, setExported] = useState(false);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(bubbles));
  }, [bubbles]);

  // load the latest translated page bitmap (real project under Tauri,
  // synthetic page in the browser preview)
  const loadPage = async () => {
    const src = await api.getTranslatedPage();
    if (!src) return;
    const img = new window.Image();
    img.onload = () => {
      setPageImage(img);
      setPageInfo(t("editor.pageLoaded"));
    };
    img.src = src;
  };

  const current = bubbles.find((b) => b.id === selected) ?? null;

  const updateSelected = (patch: Partial<Bubble>) => {
    if (!selected) return;
    setBubbles((prev) => prev.map((b) => (b.id === selected ? { ...b, ...patch } : b)));
  };

  const exportPng = () => {
    const stage = stageRef.current;
    if (!stage) return;
    const uri = stage.toDataURL({ pixelRatio: 2 });
    // data: URLs do not trigger real downloads in Chromium — convert to blob
    fetch(uri)
      .then((r) => r.blob())
      .then((blob) => {
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "manga-eroico-page.png";
        a.click();
        URL.revokeObjectURL(url);
      })
      .catch(() => {
        // fallback: plain data-url anchor (browser-dependent)
        const a = document.createElement("a");
        a.href = uri;
        a.download = "manga-eroico-page.png";
        a.click();
      });
    setExported(true);
    setTimeout(() => setExported(false), 2500);
  };

  return (
    <div data-testid="editor-page" className="flex h-full gap-6">
      <div className="flex-1">
        <div className="mb-4 flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold">{t("editor.title")}</h1>
            <p className="mt-1 text-sm" style={{ color: "var(--text-muted)" }}>
              {t("editor.subtitle")}
            </p>
          </div>
          <button
            onClick={() => void loadPage()}
            className="cursor-pointer rounded-lg border px-3 py-2 text-xs font-medium transition-colors duration-200"
            style={{ borderColor: "var(--border)", color: "var(--text)" }}
            data-testid="load-page"
          >
            {pageInfo || t("editor.loadPage")}
          </button>
        </div>

        <div
          className="rounded-xl border"
          style={{ borderColor: "var(--border)", background: "var(--surface-2)", width: CANVAS_W }}
          data-testid="canvas"
        >
          <Stage
            ref={stageRef}
            width={CANVAS_W}
            height={CANVAS_H}
            onMouseDown={(e) => {
              if (e.target === e.target.getStage()) setSelected(null);
            }}
          >
            <Layer>
              <Rect x={10} y={10} width={CANVAS_W - 20} height={CANVAS_H - 20} fill="var(--surface)" cornerRadius={8} />
              {pageImage && (
                <KonvaImage image={pageImage} x={10} y={10} width={CANVAS_W - 20} height={CANVAS_H - 20} listening={false} opacity={0.9} />
              )}
              {bubbles.map((b) => (
                <Group
                  key={b.id}
                  draggable
                  x={b.x}
                  y={b.y}
                  onClick={() => setSelected(b.id)}
                  onTap={() => setSelected(b.id)}
                  onDragEnd={(e) => {
                    const { x, y } = e.target.position();
                    setBubbles((prev) => prev.map((p) => (p.id === b.id ? { ...p, x, y } : p)));
                  }}
                >
                  <Rect
                    width={b.w}
                    height={b.h}
                    stroke={selected === b.id ? "var(--accent)" : "var(--border)"}
                    strokeWidth={selected === b.id ? 2.5 : 1.5}
                    cornerRadius={b.vertical ? 8 : Math.min(b.h / 2, 24)}
                    fill="rgba(127,127,127,0.12)"
                    shadowColor="black"
                    shadowBlur={selected === b.id ? 8 : 2}
                    shadowOpacity={0.25}
                  />
                  {b.vertical
                    ? b.text.split("").map((ch, i) => (
                        <Text
                          key={`v${i}`}
                          x={b.w / 2 - b.fontSize / 2}
                          y={6 + i * (b.fontSize + 2)}
                          text={ch}
                          fontSize={b.fontSize}
                          fill="var(--text)"
                          listening={false}
                        />
                      ))
                    : (() => {
                        // naive horizontal wrap by measured width
                        const lines: string[] = [];
                        let line = "";
                        for (const ch of b.text) {
                          line += ch;
                          if (line.length * b.fontSize * 0.9 >= b.w - 16) {
                            lines.push(line);
                            line = "";
                          }
                        }
                        if (line) lines.push(line);
                        return lines.map((ln, i) => (
                          <Text
                            key={`h${i}`}
                            x={10}
                            y={b.h / 2 - (lines.length * (b.fontSize + 4)) / 2 + i * (b.fontSize + 4)}
                            text={ln}
                            fontSize={b.fontSize}
                            fill="var(--text)"
                            listening={false}
                          />
                        ));
                      })()}
                </Group>
              ))}
            </Layer>
          </Stage>
        </div>
      </div>

      <aside className="w-64 shrink-0" data-testid="editor-panel">
        <div
          className="rounded-xl border p-4"
          style={{ borderColor: "var(--border)", background: "var(--surface)" }}
        >
          <div className="mb-2 text-sm font-semibold">{t("editor.textLayer")}</div>
          {current ? (
            <>
              <textarea
                className="mb-3 w-full rounded-lg border p-2 text-sm"
                style={{ borderColor: "var(--border)", background: "var(--surface-2)", color: "var(--text)" }}
                value={current.text}
                onChange={(e) => updateSelected({ text: e.target.value })}
                data-testid="bubble-text"
              />
              <label className="mb-3 block text-xs" style={{ color: "var(--text-muted)" }}>
                {t("editor.fontSize")}: {current.fontSize}
                <input
                  type="range"
                  min={10}
                  max={36}
                  value={current.fontSize}
                  className="mt-1 w-full"
                  onChange={(e) => updateSelected({ fontSize: Number(e.target.value) })}
                />
              </label>
              <label className="mb-1 flex items-center gap-2 text-xs">
                <input
                  type="checkbox"
                  checked={current.vertical}
                  onChange={(e) => updateSelected({ vertical: e.target.checked })}
                />
                {t("editor.vertical")}
              </label>
              <p className="mt-2 text-[11px]" style={{ color: "var(--text-muted)" }}>
                {t("editor.dragHint")}
              </p>
            </>
          ) : (
            <div className="text-xs" style={{ color: "var(--text-muted)" }}>
              {t("editor.noSelection")}
            </div>
          )}
          <button
            onClick={exportPng}
            className="mt-4 w-full cursor-pointer rounded-lg px-3 py-2 text-sm font-semibold transition-opacity duration-200"
            style={{ background: "var(--accent)", color: "var(--accent-contrast)" }}
            data-testid="export-png"
          >
            {exported ? t("editor.exported") : t("editor.export")}
          </button>
        </div>
      </aside>
    </div>
  );
}
