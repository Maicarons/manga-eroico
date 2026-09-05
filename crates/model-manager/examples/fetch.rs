//! Fetch real model files listed in the registry via the crate's own
//! resumable downloader — doubles as an end-to-end downloader test and the
//! manual "download the models" entrypoint.
//!
//! Usage: cargo run -p model-manager --example fetch -- ppocrv5_det rec_mixed ...
use model_manager::downloader::download_file;
use model_manager::registry::{Registry, ModelSpec};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn resolve_url(spec: &ModelSpec) -> String {
    if !spec.url_override.is_empty() {
        return spec.url_override.to_string();
    }
    format!(
        "https://www.modelscope.cn/models/{}/resolve/master/{}",
        spec.modelscope_repo, spec.file
    )
}

#[tokio::main]
async fn main() {
    let ids: Vec<String> = std::env::args().skip(1).collect();
    if ids.is_empty() {
        eprintln!("usage: fetch <spec_id> [spec_id...]");
        eprintln!("known ids:");
        for s in Registry::all() {
            eprintln!("  {:<22} {} ({} MiB)", s.id, s.file, s.size_mib);
        }
        std::process::exit(2);
    }

    let client = reqwest::Client::builder()
        .user_agent("manga-eroico-fetch/0.1")
        .build()
        .expect("reqwest client");

    for id in ids {
        let spec = match Registry::find(&id) {
            Some(s) => s,
            None => {
                eprintln!("unknown spec id: {id}");
                std::process::exit(2);
            }
        };
        let url = resolve_url(&spec);
        let dest = PathBuf::from("models").join(&id).join(
            spec.file.rsplit('/').next().unwrap_or(&spec.file),
        );
        if dest.exists() {
            println!("[{}] already present: {}", spec.id, dest.display());
            continue;
        }
        println!("[{}] {} -> {}", spec.id, url, dest.display());
        let last = Arc::new(AtomicU64::new(0));
        let last2 = last.clone();
        let started = std::time::Instant::now();
        download_file(&client, &url, dest.clone(), &move |bytes, total| {
            let mb = bytes / (1024 * 1024);
            if mb != last2.swap(mb, Ordering::Relaxed) {
                match total {
                    Some(t) => println!("[{}]   {} / {} MiB", spec.id, mb, t / (1024 * 1024)),
                    None => println!("[{}]   {} MiB", spec.id, mb),
                }
            }
        })
        .await
        .unwrap_or_else(|e| panic!("[{}] download failed: {e}", spec.id));
        println!(
            "[{}] done in {:.1}s ({})",
            spec.id,
            started.elapsed().as_secs_f32(),
            dest.display()
        );
    }
}
