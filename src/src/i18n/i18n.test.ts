import { describe, expect, it } from "vitest";
import zhCN from "./locales/zh-CN.json";
import en from "./locales/en.json";
import ja from "./locales/ja.json";
import ko from "./locales/ko.json";

const RESOURCES = { "zh-CN": zhCN, en, ja, ko } as Record<string, unknown>;

/** Recursively collect leaf-key paths of a JSON object. */
function leafPaths(obj: unknown, prefix = ""): string[] {
  if (obj === null || typeof obj !== "object") return [prefix];
  return Object.entries(obj as Record<string, unknown>).flatMap(([k, v]) =>
    leafPaths(v, prefix ? `${prefix}.${k}` : k),
  );
}

describe("i18n completeness", () => {
  const base = leafPaths(zhCN);

  it("every locale covers all zh-CN keys", () => {
    for (const [lang, res] of Object.entries(RESOURCES)) {
      const keys = leafPaths(res);
      const missing = base.filter((k) => !keys.includes(k));
      expect(missing, `${lang} missing: ${missing.join(", ")}`).toEqual([]);
      const extra = keys.filter((k) => !base.includes(k));
      expect(extra, `${lang} extra: ${extra.join(", ")}`).toEqual([]);
    }
  });

  it("no untranslated empty values", () => {
    for (const [lang, res] of Object.entries(RESOURCES)) {
      for (const key of leafPaths(res)) {
        const val = key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], res);
        expect(String(val ?? "").length, `${lang}:${key} is empty`).toBeGreaterThan(0);
      }
    }
  });
});
