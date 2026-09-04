# Getting Started

## Install

Download the installer for your platform from [GitHub Releases](https://github.com/Maicarons/manga-eroico/releases) (Windows NSIS/MSI, macOS DMG, Linux AppImage/deb). Installers ship **no models** — the first-run wizard downloads them.

## Model download

1. Open the "Models" page; the app detects hardware (CPU / RAM / GPU) and recommends a **lite / standard / pro** tier;
2. Pick OCR language packs (ja / ko / zh / en) and a Hy-MT2 translation model, then download from ModelScope with resume & SHA256 verification.

## Translate a manga

1. Create a project (`.mepro`) in the Projects library and pick source/target languages;
2. Import pages — chapters are grouped automatically;
3. Hit run on the Workflow page: detect → OCR → inpaint → translate execute in order;
4. (Optional) Configure an OpenAI-compatible endpoint (cloud LLM or LM Studio / Ollama) in Settings and enable the polish node for chapter-level refinement;
5. Fine-tune lettering in the Editor (font size / vertical / stroke) and export.

## For developers

```bash
git clone https://github.com/Maicarons/manga-eroico
cd manga-eroico
pnpm install && pnpm dev      # frontend (mocked pipeline)
cargo test --workspace        # Rust unit tests
pnpm tauri dev                # desktop app
```

See the [development plan](/development-plan) (Chinese) for architecture details.
