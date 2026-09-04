//! Resumable HTTP downloader with SHA256 verification.
//!
//! Uses HTTP Range requests against a `.part` file so interrupted downloads
//! (large GGUF files!) continue where they left off. Primary source is
//! ModelScope; any mirror URL works since it's plain HTTP.

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("checksum mismatch for {path}: expected {expected}, got {actual}")]
    ChecksumMismatch { path: PathBuf, expected: String, actual: String },
    #[error("server does not support resume (no Accept-Ranges) for {0}")]
    ResumeUnsupported(String),
}

/// Progress callback: bytes downloaded so far, total bytes (if known).
pub type ProgressFn<'a> = &'a (dyn Fn(u64, Option<u64>) + Send + Sync);

/// Downloads `url` into `dest` (writes to `dest.part`, renames on success).
/// Existing `.part` data is resumed via a Range request when supported.
pub async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest: PathBuf,
    progress: ProgressFn<'_>,
) -> Result<(), DownloadError> {
    let part = dest.with_extension("part");
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let already = if part.exists() { tokio::fs::metadata(&part).await?.len() } else { 0 };

    let mut request = client.get(url);
    if already > 0 {
        request = request.header("Range", format!("bytes={already}-"));
    }
    let response = request.send().await?.error_for_status()?;

    // 206 = resumed; 200 = server ignores Range, restart from scratch.
    let resume_from = if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        already
    } else {
        if already > 0 {
            tokio::fs::remove_file(&part).await?;
        }
        0
    };

    let total = response.content_length().map(|len| len + resume_from);
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&part)
        .await?;
    if resume_from > 0 {
        file.seek(std::io::SeekFrom::Start(resume_from)).await?;
    }

    let mut downloaded = resume_from;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        progress(downloaded, total);
    }
    file.flush().await?;
    drop(file);

    tokio::fs::rename(&part, &dest).await?;
    Ok(())
}

/// Computes the SHA256 of `path` (streamed, 1 MiB chunks).
pub fn sha256_file(path: &Path) -> Result<String, DownloadError> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
