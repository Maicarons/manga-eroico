# 快速开始

## 安装

从 [GitHub Releases](https://github.com/Maicarons/manga-eroico/releases) 下载对应平台的安装包（Windows NSIS/MSI、macOS DMG、Linux AppImage/deb）。安装包**不包含模型**，首次启动会引导你下载。

## 模型下载

1. 打开「模型管理」页面，应用会自动检测硬件（CPU / 内存 / 显存）并推荐 **lite / standard / pro** 三档之一；
2. 按需勾选 OCR 语言包（日 / 韩 / 中 / 英）与翻译模型（Hy-MT2），从 ModelScope 一键下载，支持断点续传与 SHA256 校验。

## 翻译一本漫画

1. 在「工程库」新建一个工程（`.mepro`），选择源语言与目标语言；
2. 导入漫画图片，自动按章节分组；
3. 在「工作流」页点击运行——检测 → OCR → 修复 → 翻译依次自动完成；
4. （可选）在「设置」页配置 OpenAI 兼容接口（云端 LLM 或本地 LM Studio / Ollama），开启润色节点做章节级上下文润色；
5. 在「编辑器」微调排版（字号 / 竖排 / 描边），导出成品。

## 开发者

```bash
git clone https://github.com/Maicarons/manga-eroico
cd manga-eroico
pnpm install && pnpm dev      # 前端（mock 管线）
cargo test --workspace        # Rust 单测
pnpm tauri dev                # 桌面应用
```

架构与完整规划见[开发计划方案](/development-plan)。
