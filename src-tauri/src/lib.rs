//! manga-eroico — Tauri 2 host library.
//!
//! Bridges the React frontend to the Rust pipeline: project management,
//! hardware detection, model downloads and pipeline runs (streaming events).

pub mod commands;
pub mod state;

use commands::*;
use state::AppState;

#[tokio::main]
pub async fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_hardware_info,
            decide_tier,
            list_models,
            get_llm_for_tier,
            download_model,
            model_exists,
            create_project,
            open_project,
            commands::get_translated_page,
            commands::get_project_overview,
            save_project,
            add_page,
            add_chapter,
            set_glossary_term,
            set_node_enabled,
            set_node_param,
            run_pipeline_page,
            run_pipeline_all,
            polish_preview,
        ])
        .setup(|app| {
            // Debug self-test: exercise the real download path once at startup
            // so command-layer failures show up in stderr immediately.
            #[cfg(debug_assertions)]
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    match crate::commands::download_model(
                        handle,
                        "ppocrv5_det".into(),
                        "models".into(),
                    )
                    .await
                    {
                        Ok(p) => eprintln!("[selftest] download OK: {p}"),
                        Err(e) => eprintln!("[selftest] download FAILED: {e}"),
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running manga-eroico");
}
