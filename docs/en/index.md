---
layout: home

hero:
  name: manga-eroico
  text: Fully-automatic manga translation
  tagline: Powered by RapidOCR + Hy-MT2 · local-first · visual workflow · AGPL-3.0
  actions:
    - theme: brand
      text: Getting Started
      link: /en/guide/getting-started
    - theme: alt
      text: Development Plan (zh)
      link: /development-plan

features:
  - title: Visual workflow
    details: Detect → OCR → inpaint → translate → polish (optional) → render. Every node is inspectable, toggleable and re-runnable.
  - title: Project-based
    details: Each job is a .mepro project — pages, chapters, glossary and versioned artifacts with rollback.
  - title: Local AI
    details: RapidOCR (PP-OCRv5/v6) detection & recognition, Tencent Hunyuan Hy-MT2 translation (33 languages), fully on-device.
  - title: Hardware-adaptive
    details: Detects GPU / RAM and picks a lite / standard / pro model tier — runs even without a discrete GPU.
  - title: Chapter-level polish
    details: Optional polish node over any OpenAI-compatible API analyses whole-chapter context before refining each bubble.
  - title: Four UI languages
    details: 简体中文 / English / 日本語 / 한국어 — UI language fully decoupled from OCR & translation languages.
---
