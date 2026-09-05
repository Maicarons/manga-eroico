import { defineConfig } from "vitepress";

export default defineConfig({
  lang: "zh-CN",
  title: "manga-eroico",
  description: "基于 RapidOCR + Hy-MT2 的漫画全自动翻译工具",
  locales: {
    "/": { label: "简体中文", lang: "zh-CN" },
    "/en/": { label: "English", lang: "en" },
    "/ja/": { label: "日本語", lang: "ja" },
    "/ko/": { label: "한국어", lang: "ko" },
  },
  themeConfig: {
    locales: {
      "/": {
        nav: [
          { text: "指南", link: "/guide/getting-started" },
          { text: "开发计划", link: "/development-plan" },
          {
            text: "GitHub",
            link: "https://github.com/Maicarons/manga-eroico",
          },
        ],
        sidebar: [
          {
            text: "指南",
            items: [{ text: "快速开始", link: "/guide/getting-started" }],
          },
          {
            text: "参考",
            items: [
              { text: "开发计划方案", link: "/development-plan" },
              { text: "模型许可证", link: "/reference/licenses" },
            ],
          },
        ],
        outlineTitle: "本页目录",
        docFooter: { prev: "上一页", next: "下一页" },
      },
      "/en/": {
        nav: [
          { text: "Guide", link: "/en/guide/getting-started" },
          { text: "GitHub", link: "https://github.com/Maicarons/manga-eroico" },
        ],
        sidebar: [
          {
            text: "Guide",
            items: [{ text: "Getting Started", link: "/en/guide/getting-started" }],
          },
        ],
      },
      "/ja/": {
        nav: [
          { text: "ガイド", link: "/ja/guide/getting-started" },
          { text: "GitHub", link: "https://github.com/Maicarons/manga-eroico" },
        ],
        sidebar: [
          {
            text: "ガイド",
            items: [{ text: "Getting Started", link: "/ja/guide/getting-started" }],
          },
        ],
      },
      "/ko/": {
        nav: [
          { text: "가이드", link: "/ko/guide/getting-started" },
          { text: "GitHub", link: "https://github.com/Maicarons/manga-eroico" },
        ],
        sidebar: [
          {
            text: "가이드",
            items: [{ text: "Getting Started", link: "/ko/guide/getting-started" }],
          },
        ],
      },
    },
  },
});
