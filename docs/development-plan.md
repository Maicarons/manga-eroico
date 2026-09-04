# manga-eroico 项目开发计划方案

> 基于 RapidOCR + Hy-MT2 的漫画全自动翻译工具
> 版本：v1.0（方案评审稿）｜日期：2026-09-05｜许可证：AGPL-3.0

---

## 1. 项目概述

### 1.1 一句话定位

manga-eroico 是一款**本地优先、全流程可视化**的漫画自动翻译桌面应用：把日文/英文/韩文/中文漫画拖进来，经过「检测 → OCR → 抹字修复 → 翻译 → 排版渲染」五段流水线，输出排版完成的译制漫画，全程模型本地推理、无云端依赖。

### 1.2 核心目标

| 维度 | 目标 |
|---|---|
| 自动化 | 单页/整本批量翻译，默认全自动，支持人工逐格校正 |
| 质量 | 检测+OCR 采用 RapidOCR PP-OCRv5/v6；翻译采用腾讯混元 Hy-MT2（33 语互译） |
| 普惠 | 无独立显卡也能跑（CPU 推理档位），有 N 卡自动启用 CUDA 加速 |
| 可视化 | 整个翻译工作流以节点图呈现，每一步可查看/编辑中间产物 |
| 工程化 | 每个翻译任务是一个独立工程（`.mepro`），页面/章节/进度/术语表/中间产物/设置全部工程内管理，可保存、重开、增量重跑 |
| 润色 | 内置可选「润色」节点：通过 OpenAI 兼容接口做章节级上下文分析，对每条译文润色，默认关闭、随时开关 |
| 隐私 | 模型本地运行，图片不上传 |
| 跨平台 | Tauri 2.x 打包 Windows / macOS / Linux |

### 1.3 与同类项目的差异

| 项目 | 形态 | 短板（本项目的切入點） |
|---|---|---|
| manga-image-translator | Python CLI/Web | 依赖重（~15GB Docker 镜像）、无可视化工作流 |
| BallonsTranslator | Python 桌面 | 非 Rust、打包体积大 |
| Koharu | Rust CLI | 无 GUI、无可视化、模型管理弱 |

manga-eroico = **Rust 性能与体积** + **可视化节点工作流** + **模型管理器（按硬件自动选型）**。

---

## 2. 总体架构

### 2.1 架构分层

```
┌────────────────────────────────────────────────────────┐
│  前端 React 19 + TypeScript（Tauri WebView）             │
│  ├─ 工作流画布（React Flow 节点图）                       │
│  ├─ 工程库（工程管理：新建/重开/章节树/进度）              │
│  ├─ 漫画编辑器（Konva.js 画布：气泡框/文字层/字体）         │
│  ├─ 模型管理器 UI（下载/校验/切换档位）                    │
│  └─ 批量队列 & 导出中心                                   │
├────────────────  Tauri IPC（invoke / event）  ──────────┤
│  Rust 后端（Cargo workspace）                            │
│  ├─ pipeline-core   工作流 DAG 引擎（暂停/恢复/重试）      │
│  ├─ me-project      工程管理（.mepro 读写、工件版本/回滚）   │
│  ├─ detect / ocr / inpaint / translate / polish / render │
│  │    （统一 ModelProvider trait，底层 ort / llama.cpp）  │
│  ├─ model-manager  ModelScope 下载+系统检测+档位选型      │
│  └─ tauri host     命令注册、事件推送、文件系统访问         │
└────────────────────────────────────────────────────────┘
```

### 2.2 技术选型总表

| 层 | 技术 | 版本/说明 |
|---|---|---|
| 框架 | Tauri | 2.x（App 打包，不含模型） |
| 后端语言 | Rust | stable（edition 2021+） |
| OCR 推理 | `ort` crate | ONNX Runtime 2.0-rc，CUDA / DirectML / CoreML / CPU EP |
| LLM 推理 | `llama-cpp-2` | Hy-MT2 GGUF 量化版，支持 CUDA/Metal/Vulkan |
| 润色接入 | `reqwest` + OpenAI 兼容 Chat Completions | 云端 LLM 或本地（LM Studio / Ollama）均可配置 |
| 文本检测 | comic-text-detector ONNX | 来自 manga-image-translator 发布物 |
| 抹字修复 | AOT / LaMa-mpe ONNX | 两档可选 |
| 前端 | React 19 + Vite + TS | strict mode |
| 画布 | Konva.js（react-konva） | 漫画编辑/排版层 |
| 工作流 | React Flow（@xyflow/react） | 节点式 pipeline 可视化 |
| 状态 | Zustand | 轻量、可与 IPC 同步 |
| 样式 | Tailwind CSS v4 | CSS 变量主题化，暗黑模式一等公民 |
| UI/UX | 调用 ui-ux-pro-max + frontend-design skill | M1 阶段产出设计系统 |
| i18n | react-i18next + ICU MessageFormat | zh-CN / en / ja / ko 首发 |
| 测试 | cargo test / Vitest / Playwright / tauri-driver | 见 §8 |
| 文档 | VitePress | GH Pages CI 自动发布 |
| 协议 | **AGPL-3.0** | 根目录 LICENSE + 各 crate 声明 |

---

## 3. 核心翻译管线设计

### 3.1 六段流水线（润色节点可开关）

```
