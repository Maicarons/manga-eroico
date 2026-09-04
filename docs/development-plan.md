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
导入 → ①文本检测 → ②OCR → ③抹字修复 → ④机器翻译 → ⑤润色（可选） → ⑥排版渲染 → 导出
        CTD         RapidOCR   AOT/LaMa     Hy-MT2     OpenAI 兼容 API    规则引擎
```

| 阶段 | 输入 | 模型/引擎 | 输出 | 可视化节点能力 |
|---|---|---|---|---|
| ① 检测 | 原图 | comic-text-detector | 文本区域多边形 + mask | 查看框选结果，手动增删框 |
| ② OCR | 裁剪区域 | RapidOCR PP-OCRv5/v6（det 自带 / rec 按语言切换）+ 方向分类 | 原文 + 置信度 | 双栏对照审校，低置信度高亮 |
| ③ 修复 | 原图 + mask | AOT（快）/ LaMa-mpe（优） | 无字底图 | 修复前后对比滑块 |
| ④ 翻译 | 原文列表 + 上下文 | Hy-MT2-1.8B / 7B / 30B-A3B（GGUF，按硬件选档） | 译文 | 术语表注入、风格指令、逐条重译 |
| ⑤ 润色 | ④的译文 + 章节上下文 | OpenAI 兼容接口（云端或本地 LLM） | 润色后译文 | **默认关闭**；开启后 diff 对比，逐条采纳/忽略 |
| ⑥ 渲染 | 底图 + 译文 + 框 | 规则引擎（无模型）：字体度量、竖排、自动缩行、描边 | 成品页 | Konva 画布手动微调 |

关键设计点：

1. **OCR 识别语言与翻译目标语言解耦**——det 通用，rec 模型按源语言（日/英/韩/中）下载对应字典与模型；Hy-MT2 负责 33 语互译，因此源语言不限于首发 UI 语言。
2. **DAG 而非硬编码管线**——`pipeline-core` 把六段建模为节点图，用户可在工作流画布上禁用某段（如已有翻译稿，跳过 ④⑤）、重跑某段、并行分页处理；**润色节点默认关闭**，开启后以实线节点纳入 DAG，未配置 API 时自动置灰并提示。
3. **一切中间产物可回放**——每个节点输出落盘为工程内工件（JSON + 图片），工作流画布上点开节点即查看，支持从任意节点重跑（增量翻译整本漫画的基础）。

### 3.2 翻译质量增强

- **上下文聚合**：同一页气泡按阅读顺序（日漫右→左）拼接为带位置提示的批量翻译请求，Hy-MT2 支持术语表/风格指令，用于人名一致性。
- **术语表（Glossary）**：工程级 JSON，翻译/润色节点均作为指令注入。
- **GEMBA 式自检（P2 可选）**：低置信度 OCR（<0.7）的条目标记出来要求人工确认后再送翻译。

### 3.3 润色节点设计（可选开关）

- **接入方式**：标准 OpenAI 兼容 Chat Completions（`base_url` + `api_key` + `model` 三项配置），云端（GPT / DeepSeek / 混元）与本地（LM Studio / Ollama）均可，同一套协议。
- **工作方式（章节级上下文）**：以章节为单位聚合——按阅读顺序把整章已译气泡（附带页码/位置提示/术语表）拼成结构化请求，先让 LLM 输出全章风格与一致性分析（人名统一、语气、用语习惯），再对每条译文给出润色版本，保证跨页语境连贯而非逐句孤立润色。
- **结果呈现**：编辑器内 diff 对比（机翻原文 vs 润色），逐条「采纳 / 忽略 / 手改」；可只对选中页重跑润色。
- **开关语义**：默认关闭；关闭时 ④ 直接连 ⑥，产物沿用 ④ 的译文；开启即计入章节进度矩阵。
- **密钥安全**：API key 存系统凭据管理器（Windows Credential Manager / macOS Keychain / libsecret），**不写入工程文件**；工程内只存 base_url 与 model 名。
- **降级策略**：请求失败自动重试 → 失败页标记跳过并保留机翻结果，不阻塞整章流程。

### 3.4 工程化管理

一个翻译任务 = 一个工程，目录式格式（对外后缀 `.mepro`，本质是目录，便于双击关联打开与整包备份）：

```
MyManga.mepro/
├─ project.json        # 元信息：名称、语言对、硬件档位、各节点配置（含润色开关）、章节树、schema 版本
├─ pages/              # 原始导入图（内容寻址命名，去重）
├─ artifacts/          # 各节点产物（mask / OCR JSON / 底图 / 成品图），按 节点×页×版本 存放
├─ glossary.json       # 工程级术语表
├─ history/            # 操作快照与回滚点
└─ cache/              # 可随时清理的临时缓存
```

- **工程库（应用首页）**：最近工程卡片（封面缩略图、进度、语言对、上次打开时间），新建 / 导入 / 克隆 / 删除（进回收站）。
- **章节树管理**：批量导入按文件名规则自动分组为章节，支持手动拖拽调整；进度以「页 × 节点」完成度矩阵展示。
- **版本与回滚**：每次节点运行生成新工件版本（旧版本保留可回滚）；只重跑变更页 = 增量翻译。
- **迁移与兼容**：`project.json` 携带 schema 版本号，应用升级时自动迁移；工程整目录可直接拷贝/打包导出，跨设备无锁。

---

## 4. 模型管理与系统适配

### 4.1 系统检测

`model-manager` 启动时探测：GPU 型号/显存（CUDA via `nvml-wrapper`、DirectML、Metal）、系统内存、CPU 核数、磁盘剩余空间，据此生成**硬件档位**。

### 4.2 模型档位矩阵（ModelScope 下载）

| 档位 | 判定条件 | 检测/OCR | 修复 | 翻译 | 下载体积约 |
|---|---|---|---|---|---|
| lite | 无独显 或 VRAM < 4GB | PP-OCRv5 mobile | AOT | Hy-MT2-1.8B Q4（或 1.25-bit ~440MB 端侧版） | ~1.5GB |
| standard | VRAM 4–8GB | PP-OCRv5 server det + mobile rec | LaMa-mpe | Hy-MT2-1.8B Q8 / 7B Q4 | ~4GB |
| pro | VRAM ≥ 12GB 或 Apple M 系列 ≥ 16GB 统一内存 | PP-OCRv5 server 全套 + v6 rec 可选 | LaMa-mpe | Hy-MT2-7B Q8 / 30B-A3B Q4 | ~10GB+ |

- 语言包独立下载：日/韩/中/英 rec 模型 + 字典各自几 MB~几十 MB，按需拉取。
- 下载能力：ModelScope HTTP API 直链下载 + **断点续传 + SHA256 校验 + 失败自动重试**，镜像源可切换（HuggingFace 兜底）。
- 模型存放：Tauri `app_data_dir()/models/`，**打包产物不含任何模型**，首次启动引导进入模型管理器向导（检测硬件 → 推荐档位 → 一键下载 → 进度/校验展示）。

---

## 5. 仓库结构（GitHub / monorepo）

```
manga-eroico/
├─ LICENSE                      # AGPL-3.0
├─ README.md                    # 中英双语 + 截图 + 徽章
├─ Cargo.toml                   # cargo workspace
├─ package.json                 # pnpm workspace（前端 + docs）
├─ crates/
│  ├─ pipeline-core/            # DAG 引擎、任务调度、事件流
│  ├─ me-project/               # .mepro 工程读写、章节树、工件版本/回滚
│  ├─ me-detect/  me-ocr/  me-inpaint/  me-translate/  me-polish/  me-render/
│  ├─ model-manager/            # ModelScope 下载、系统探测、档位决策
│  └─ me-server/                # 无头 CLI / HTTP 服务模式（可选）
├─ src-tauri/                   # Tauri 2 host（命令/事件/打包配置）
├─ src/                         # React 前端
│  ├─ app/                      # 路由、布局
│  ├─ features/
│  │  ├─ projects/              # 工程库：新建/重开/章节树/进度矩阵
│  │  ├─ workflow/              # React Flow 工作流画布（含润色开关节点）
│  │  ├─ editor/                # Konva 漫画编辑器（含润色 diff 视图）
│  │  ├─ models/                # 模型管理器 UI
│  │  ├─ batch/                 # 批量队列
│  │  └─ settings/
│  ├─ i18n/                     # zh-CN / en / ja / ko 资源
│  └─ design-system/            # Tokens、主题（明/暗）、组件库
├─ docs/                        # VitePress 文档站（本文件所在）
├─ tests/
│  ├─ e2e/                      # Playwright + tauri-driver
│  └─ golden/                   # 渲染金样图测试
└─ .github/workflows/           # ci.yml / release.yml / docs.yml
```

---

## 6. 前端 UI/UX 方案

### 6.1 设计流程约定

M1 阶段调用 **ui-ux-pro-max**（风格/色板/组件选型）与 **frontend-design**（生产级前端实现规范）skill，产出 `docs/design/design-system.md` + Tokens 代码后才开始写界面。核心约束提前锁定：

- **暗黑模式为一等公民**：所有颜色走 CSS 变量语义层（`--bg`/`--surface`/`--text`/`--accent`…），Tailwind v4 主题化；跟随系统 + 手动切换。
- **对普通用户友好**：默认打开即「拖图进来」的极简首屏；高级能力（节点图、参数、术语表）收进「专家模式」开关；每一步有明确进度与可撤销操作。
- 中漫/日漫阅读方向、CJK 排版（竖排、标点挤压）作为渲染层硬需求。

### 6.2 四大核心界面

1. **工程库（首页）**：工程卡片（封面/进度/语言对）+ 新建向导（选图 → 命名 → 自动检测语言与档位）；章节树与「页 × 节点」进度矩阵入口。
2. **工作流画布**（React Flow）：六个默认节点串成流水线（润色默认关闭、虚线旁路显示），节点上有实时状态（排队/运行/完成/失败）与产物缩略图，点击节点 → 右侧详情面板（查看 mask、OCR 对照表、修复对比、译文编辑、润色 diff）。
3. **漫画编辑器**（Konva）：底图层 + 文字层分离，框选文字可改字体/字号/颜色/描边/竖排，实时预览成品；润色开启时叠加 diff 审阅模式。
4. **模型管理器**：硬件检测结果卡片 + 档位推荐 + 下载队列（进度/速度/校验），语言包按需勾选；润色 endpoint 配置（base_url/model，key 走系统凭据管理器）。

---

## 7. i18n 方案

| 范围 | 方案 |
|---|---|
| 应用 UI | react-i18next，`zh-CN / en / ja / ko` 四语言首发；命名空间按 feature 拆分；ICU 处理复数/占位 |
| 文档站 | VitePress `locales`：首发 `zh`（默认）+ `en`，`ja/ko` 随 M4 跟进 |
| OCR/翻译语言 | 与 UI 语言解耦：rec 语言包独立，Hy-MT2 原生 33 语 |

约定：代码内不写死文案，新增 UI 必须同步四语言 key（CI 加 key 完整性检查）。

---

## 8. 测试策略

| 层 | 工具 | 覆盖内容 |
|---|---|---|
| Rust 单测 | `cargo test` + `insta` 快照 | DAG 引擎调度/暂停恢复、各模型前后处理（几何变换、mask 后处理、CTC 解码、字体度量）、`.mepro` 工程读写与 schema 迁移——用固定小图 + mock 推理输出，**CI 不依赖真实模型** |
| Rust 集成 | `#[ignore]` 标记 + nightly job | 真模型小样本端到端冒烟（下载 mini 模型跑通六段）；润色节点用 wiremock 模拟 OpenAI 兼容接口（正常/超时/降级路径） |
| 前端单测 | Vitest + Testing Library | 组件、状态 store、i18n key 完整性 |
| E2E（Web 层） | Playwright | 工作流画布交互、模型管理器向导、编辑器操作 |
| E2E（App 层） | tauri-driver（WebDriver） | 打包后真应用冒烟：导入图 → 跑 mock pipeline → 导出 |
| 金样测试 | `tests/golden/` | 渲染层对固定输入输出 PNG，感知哈希（pHash）比对阈值 |
| 门槛 | CI 必过 | Rust 单测 + 前端单测 + Web E2E + clippy -D warnings + tsc |

---

## 9. CI/CD 与发布

| Workflow | 触发 | 内容 |
|---|---|---|
| `ci.yml` | PR / push main | lint（clippy/prettier/eslint）→ Rust test（3 平台矩阵）→ 前端 test + Web E2E → tauri build 试打包（验证不含模型：检查产物体积与内容） |
| `release.yml` | tag `v*` | tauri-action 三平台签名打包（msi/nsis、dmg、AppImage/deb）→ GitHub Release 草稿 |
| `docs.yml` | push main（docs/ 变更） | VitePress build（多 locale）→ 部署 GitHub Pages |

模型分发**只走 ModelScope**，Release 产物仅含应用本体（预期 < 50MB 量级，正可作为「不含模型」的 CI 断言）。

---

## 10. 里程碑规划

| 里程碑 | 内容 | 交付判据 |
|---|---|---|
| **M0 地基**（第 1–2 周） | 仓库初始化（AGPL LICENSE、workspace、pnpm、VitePress 骨架）、Tauri 空壳跑通三平台、CI 三条流水线上线、i18n 骨架四语言 | CI 全绿；空应用可打包 |
| **M1 设计 + 模型管理**（第 2–4 周） | 调用 UI/UX skill 产出设计系统；模型管理器（系统检测/档位/ModelScope 下载/校验/续传） | 空应用内可完成模型下载向导 |
| **M2 管线核心**（第 4–7 周） | 检测→OCR→修复→翻译→渲染五段 crates 打通（先 CPU 档位）+ `me-project` 工程读写 + **润色节点**（OpenAI 兼容 client、章节上下文聚合、降级重试），DAG 引擎 + 事件流 | CLI（me-server）能创建工程→整页出译图→保存重开；润色节点经 mock 接口联调通过 |
| **M3 可视化工作流 + 编辑器 + 工程库**（第 7–10 周） | React Flow 画布接通管线事件（含润色开关与 diff 视图）；Konva 编辑器（文字层编辑/竖排/导出）；工程库首页 + 章节树 + 进度矩阵 | 全自动翻译一页并在编辑器微调导出；润色可开关并逐条采纳 |
| **M4 批量 + 打磨**（第 10–13 周） | 整本/文件夹批量队列、章节级增量重跑与版本回滚、GPU 档位（CUDA/Metal/DirectML）、文档站四语言补全、UI/UX 走查 | 整本漫画 30 分钟内可完成翻译（含可选润色） |
| **M5 测试加固 + v0.1 发布**（第 13–15 周） | E2E 全覆盖、金样测试、性能 profile、README/文档终稿、GitHub Release v0.1 | 三平台安装包可下载可用 |
| **M6+ 展望** | 在线色板/字体市场、Web GPU 推理、插件系统、LLM 校对（Hy-MT2 指令风格化） | — |

> 排期为相对节奏，按 M0→M5 顺序推进，每个里程碑结束做一次回顾并同步 docs。

---

## 11. 许可证与合规

- 本项目代码：**AGPL-3.0**（满足要求 1）。
- 第三方模型许可证登记到 `docs/reference/licenses.md`：
  - RapidOCR / PaddleOCR 模型：Apache-2.0 ✅
  - Hy-MT2：腾讯混元开源协议（使用前核对其附加条款，登记原文链接）
  - comic-text-detector / LaMa：跟随上游发布物许可，逐项登记
- 仅**调用模型权重**、不复制 GPL 项目的源码，规避许可证传染冲突；Koharu/manga-image-translator 仅作行为参考。

---

## 12. 风险与对策

| 风险 | 对策 |
|---|---|
| `ort` CUDA/DirectML EP 在 Windows 打包后的动态库地狱 | 采用 `load-dynamic` 策略 + 运行时下载对应 EP 运行库；CI 中加冒烟断言 |
| Hy-MT2 GGUF 转换质量/上游未提供 | 优先用官方已发布量化版；必要时自行 llama.cpp 转换并登记脚本 |
| 竖排 CJK 排版复杂度高 | M2 先做横排 + 简单竖排，M4 迭代标点挤压/自动换行细节 |
| comic-text-detector 模型较老 | 保留 det 节点可替换设计（trait 抽象），后续可换 PP-OCRv5 det 双通道 |
| ModelScope 下载在海外不稳定 | 支持 HF 镜像切换 + 断点续传 |
| 范围蔓延（五段全自研） | 严格执行里程碑判据；渲染/编辑器细节允许 M4 后继续打磨 |

---

## 13. 参考资料

- RapidOCR（RapidAI）：https://github.com/RapidAI/RapidOCR（PP-OCRv5/v6 ONNX 模型清单 `python/rapidocr/default_models.yaml`）
- Hy-MT2（腾讯混元）：https://github.com/Tencent-Hunyuan/Hy-MT2 ｜ ModelScope 合集：https://modelscope.cn/collections/Tencent-Hunyuan/Hy-MT2
- manga-image-translator（检测/修复模型发布物）：https://github.com/zyddnys/manga-image-translator
- Koharu（Rust 漫画翻译先行者）：https://github.com/mayocream/koharu
- ort（Rust ONNX Runtime 绑定）：https://ort.pyke.io
- Tauri 2：https://v2.tauri.app ｜ React Flow：https://xyflow.com ｜ Konva：https://konvajs.org
- VitePress：https://vitepress.dev
