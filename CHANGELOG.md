# Changelog

## Unreleased

- **Style-directed polish** (Hy-MT2 instruction following): formal / casual /
  literary presets plus custom instructions, via `--polish-style` and the
  settings page.
- **CUDA execution provider** behind the `gpu` feature; validated on an RTX
  4060 Laptop (EP registration + full batch run; CPU remains faster for small
  pages due to CUDA init overhead).

## v0.1.0 (2026-09-05)

First milestone-complete release: M0-M4 of the development plan.

### Highlights

- **Visual pipeline** — detect → OCR → inpaint → translate → polish (optional) → render
  as a React Flow canvas wired to live pipeline events; every node toggleable and re-runnable.
- **Real on-device inference** — PP-OCRv5 (det + mixed zh/en/ja/ko rec) via ONNX Runtime;
  Hy-MT2 GGUF models downloadable through the in-app ModelScope manager with resume + SHA256.
- **Project management** — every job is a `.mepro` project: pages, auto-grouped chapters,
  glossary, per-node versioned artifacts, operation history.
- **Editor** — Konva canvas with draggable bubbles, per-bubble font size & vertical CJK
  layout, translated-page underlay, PNG export.
- **Polish node** — OpenAI-compatible chapter-level refinement with per-bubble adopt/dismiss.
- **Batch** — folder import, whole-manga runs; measured at ~45 pages/min on CPU.
- **Headless CLI** — `me-server` (create / import-folder / run-page / status).
- **Docs site** — VitePress in four languages (zh-CN / en / ja / ko) on GitHub Pages.

### Known limitations

- AI inpainting (AOT/LaMa) ships later behind a feature; CPU fill covers flat bubble
  backgrounds today.
- Local GGUF inference (llama.cpp) is planned; translation currently uses any
  OpenAI-compatible endpoint (LM Studio / Ollama / cloud).
- CUDA execution provider is behind a non-default feature and needs validation on
  NVIDIA hardware.
