<div align="center">

# manga-eroico

**Fully-automatic manga translation, local-first & visual.**
**漫画全自动翻译工具 · 本地优先 · 全流程可视化**

Rust + React + Tauri · Powered by [RapidOCR](https://github.com/RapidAI/RapidOCR) & [Hy-MT2](https://github.com/Tencent-Hunyuan/Hy-MT2)

[![CI](https://github.com/Maicarons/manga-eroico/actions/workflows/ci.yml/badge.svg)](./.github/workflows/ci.yml)
[![Docs](https://img.shields.io/badge/docs-vitepress-blue)](https://maicarons.github.io/manga-eroico/)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-green.svg)](./LICENSE)

</div>

## ✨ Features

- 🔁 **Visual workflow** — detect → OCR → inpaint → translate → polish (optional) → render, every node inspectable & re-runnable
- 🗂️ **Project-based** — every translation job is a `.mepro` project: pages, chapters, glossary, artifacts, versioned & rollback-able
- 🧠 **Local AI** — RapidOCR (PP-OCRv5/v6) for detection & OCR, Hy-MT2 (1.8B/7B/30B-A3B) for 33-language translation, all on-device
- 📈 **Hardware-adaptive** — detects GPU/RAM and picks a lite / standard / pro model tier automatically
- 🌐 **OpenAI-compatible polish** — optional chapter-level context-aware text polishing via any OpenAI-compatible API (cloud or LM Studio / Ollama)
- 🌏 **i18n** — UI in 简体中文 / English / 日本語 / 한국어; OCR & translation languages independent of UI language
- 🌙 **Dark mode** first-class citizen
- 📦 **Small installer** — app bundles contain **no models**; models download from ModelScope in-app with resume + checksum

## 🚀 Quick Start

```bash
# prerequisites: Rust (1.85+), Node 20+, pnpm 9+
pnpm install
pnpm dev          # web preview (mocked pipeline)
pnpm tauri dev    # full desktop app

cargo test        # Rust unit tests
pnpm test         # frontend unit tests
```

## 📚 Documentation

- [开发计划 / Development Plan](./docs/development-plan.md)
- Docs site (VitePress): `pnpm docs:dev` → published to GitHub Pages via CI
- [Third-party model licenses](./docs/reference/licenses.md)

## 📜 License

This project is licensed under **[AGPL-3.0](./LICENSE)**.
Model weights keep their own licenses — see [docs/reference/licenses.md](./docs/reference/licenses.md).
