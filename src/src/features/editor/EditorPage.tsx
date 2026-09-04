import { useState } from "react";
import { Stage, Layer, Rect, Text } from "react-konva";
import { useTranslation } from "react-i18next";

interface Bubble {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  text: string;
  vertical: boolean;
}

const INITIAL_BUBBLES: Bubble[] = [
  { id: "b1", x: 60, y: 40, w: 220, h: 90, text: "こんにちは、世界！", vertical: false },
  { id: "b2", x: 320, y: 160, w: 90, h: 200, text: "またね、またね", vertical: true },
];

export default function EditorPage() {
  const { t } = useTranslation();
  const [bubbles, setBubbles] = useState<Bubble[]>(INITIAL_BUBBLES);
  const [selected, setSelected] = useState<string | null>("b1");
  const [fontSize, setFontSize] = useState(18);

  const current = bubbles.find((b) => b.id === selected) ?? null;

  const updateSelected = (patch: Partial<Bubble>) => {
    if (!selected) return;
    setBubbles((prev) => prev.map((b) => (b.id === selected ? { ...b, ...patch } : b)));
  };

  return (
    <div data-testid="editor-page" className="flex h-full gap-6">
      <div className="flex-1">
        <h1 className="text-2xl font-bold">{t("editor.title")}</h1>
        <p className="mb-4 mt-1 text-sm" style={{ color: "var(--text-muted)" }}>
          {t("editor.subtitle")}
        </p>
        <div
          className="rounded-xl border"
          style={{ borderColor: "var(--border)", background: "var(--surface-2)", width: 620 }}
        >
          <Stage width={620} height={420} data-testid="canvas">
            <Layer>
              {/* base layer placeholder (page bitmap goes here) */}
              <Rect x={10} y={10} width={600} height={400} fill="var(--surface)" cornerRadius={8} />
              {bubbles.map((b) => (
                <Rect
                  key={`box-${b.id}`}
                  x={b.x}
                  y={b.y}
                  width={b.w}
                  height={b.h}
                  stroke={selected === b.id ? "var(--accent)" : "var(--border)"}
                  strokeWidth={selected === b.id ? 2.5 : 1.5}
                  cornerRadius={b.vertical ? 8 : b.h / 2}
                  fill="rgba(127,127,127,0.08)"
                  onClick={() => setSelected(b.id)}
                  onTap={() => setSelected(b.id)}
                />
              ))}
              {bubbles.flatMap((b) =>
                b.vertical
                  ? b.text.split("").map((ch, i) => (
                      <Text
                        key={`${b.id}-v${i}`}
                        x={b.x + b.w / 2 - fontSize / 2}
                        y={b.y + 8 + i * (fontSize + 2)}
                        text={ch}
                        fontSize={fontSize}
                        fill="var(--text)"
                        listening={false}
                      />
                    ))
                  : [
                      <Text
                        key={`${b.id}-h`}
                        x={b.x + 12}
                        y={b.y + b.h / 2 - fontSize / 2}
                        text={b.text}
                        fontSize={fontSize}
                        fill="var(--text)"
                        listening={false}
                      />,
                    ],
              )}
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
              />
              <label className="mb-3 block text-xs" style={{ color: "var(--text-muted)" }}>
                {t("editor.fontSize")}: {fontSize}
                <input
                  type="range"
                  min={10}
                  max={36}
                  value={fontSize}
                  className="mt-1 w-full"
                  onChange={(e) => setFontSize(Number(e.target.value))}
                />
              </label>
              <label className="flex items-center gap-2 text-xs">
                <input
                  type="checkbox"
                  checked={current.vertical}
                  onChange={(e) => updateSelected({ vertical: e.target.checked })}
                />
                {t("editor.vertical")}
              </label>
            </>
          ) : (
            <div className="text-xs" style={{ color: "var(--text-muted)" }}>
              —
            </div>
          )}
          <button
            className="mt-4 w-full rounded-lg px-3 py-2 text-sm font-semibold"
            style={{ background: "var(--accent)", color: "var(--accent-contrast)" }}
          >
            {t("editor.export")}
          </button>
        </div>
      </aside>
    </div>
  );
}
