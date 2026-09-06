//! Replicates src-tauri download_model exactly (non-www URL, deep dest path):
//! cargo run -p model-manager --example fetch_cmd
use model_manager::{download_file, Registry};

#[tokio::main]
async fn main() {
    let spec_id = std::env::args().nth(1).unwrap_or_else(|| "ppocrv5_det".into());
    let spec = Registry::find(&spec_id).expect("spec");
    let url = format!(
        "https://modelscope.cn/models/{}/resolve/master/{}",
        spec.modelscope_repo, spec.file
    );
    println!("URL: {url}");
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    let dest_dir = std::path::PathBuf::from(home).join(".manga-eroico").join("models");
    let dest = dest_dir.join(spec.file);
    println!("DEST: {}", dest.display());
    let client = reqwest::Client::builder()
        .user_agent("manga-eroico/0.1")
        .build()
        .unwrap();
    let started = std::time::Instant::now();
    download_file(&client, &url, dest.clone(), &|d, t| {
        if let Some(t) = t {
            let pct = ((d as f64 / t as f64) * 100.0) as u8;
            print!("\r{pct}%");
        }
    })
    .await
    .unwrap_or_else(|e| panic!("DOWNLOAD FAILED: {e}"));
    println!("\nOK in {:.1}s -> {}", started.elapsed().as_secs_f32(), dest.display());
}
