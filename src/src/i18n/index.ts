import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import zhCN from "./locales/zh-CN.json";
import en from "./locales/en.json";
import ja from "./locales/ja.json";
import ko from "./locales/ko.json";

export const SUPPORTED_LANGS = ["zh-CN", "en", "ja", "ko"] as const;
export type SupportedLang = (typeof SUPPORTED_LANGS)[number];

export const LANG_META: Record<SupportedLang, { label: string; flag: string }> = {
  "zh-CN": { label: "简体中文", flag: "🇨🇳" },
  en: { label: "English", flag: "🇬🇧" },
  ja: { label: "日本語", flag: "🇯🇵" },
  ko: { label: "한국어", flag: "🇰🇷" },
};

void i18n.use(initReactI18next).init({
  resources: {
    "zh-CN": { translation: zhCN },
    en: { translation: en },
    ja: { translation: ja },
    ko: { translation: ko },
  },
  lng: navigator.language.startsWith("zh")
    ? "zh-CN"
    : (navigator.language.slice(0, 2) as SupportedLang) in LANG_META
      ? (navigator.language.slice(0, 2) as SupportedLang)
      : "zh-CN",
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

export default i18n;
