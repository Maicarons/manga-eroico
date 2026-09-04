---
layout: home

hero:
  name: manga-eroico
  text: 漫画全自动翻译工具
  tagline: 基于 RapidOCR + Hy-MT2 · 本地优先 · 全流程可视化 · AGPL-3.0 开源
  actions:
    - theme: brand
      text: 快速开始
      link: /guide/getting-started
    - theme: alt
      text: 开发计划方案
      link: /development-plan

features:
  - title: 可视化工作流
    details: 检测 → OCR → 抹字修复 → 翻译 → 润色（可选）→ 渲染，每个节点可查看、可开关、可重跑。
  - title: 工程化管理
    details: 每个翻译任务是一个 .mepro 工程：页面、章节、术语表、中间产物版本化保存、可回滚。
  - title: 本地 AI 推理
    details: RapidOCR（PP-OCRv5/v6）检测识别，腾讯混元 Hy-MT2 翻译（33 语互译），全程本地运行。
  - title: 硬件自适应
    details: 自动检测 GPU / 内存，按 lite / standard / pro 三档选择模型，无独显也能跑。
  - title: 章节级润色
    details: 通过 OpenAI 兼容接口对整章上下文分析后逐条润色译文，默认关闭、随时开关。
  - title: 四语言界面
    details: 简体中文 / English / 日本語 / 한국어，界面语言与 OCR、翻译语言完全解耦。
---
