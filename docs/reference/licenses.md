# 第三方模型许可证登记

本项目代码以 AGPL-3.0 发布；模型权重保留其原始许可证。应用本体不打包任何模型，权重均在运行时由用户从 ModelScope 下载。

| 模型 | 来源 | 许可证 | 备注 |
|---|---|---|---|
| PP-OCRv5 det/cls/rec + 字典 | RapidAI/RapidOCR（PaddleOCR 权重） | Apache-2.0 | 中/英/日/韩识别 |
| comic-text-detector | manga-image-translator release | 随上游（使用前核对原仓库 LICENSE） | 文本区域检测 |
| AOT Inpainter / LaMa-mpe | manga-image-translator release | 随上游（使用前核对） | 抹字修复 |
| Hy-MT2-1.8B / 7B / 30B-A3B | Tencent-Hunyuan/Hy-MT2 | 混元开源协议（使用前核对官方仓库条款） | GGUF 量化用于本地翻译 |

> 注意：上表「使用前核对」条目需要在集成对应模型下载时，逐项核对其官方仓库的许可证原文并在此登记结论。仅调用模型权重、不复制 GPL 项目源码，规避许可证传染。
