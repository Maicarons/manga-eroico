//! model-manager: detects hardware, picks a lite/standard/pro model tier and
//! downloads models from ModelScope with resume + SHA256 verification.
//! The app bundle ships no models — everything is fetched here at runtime.

pub mod downloader;
pub mod hardware;
pub mod registry;

pub use downloader::{download_file, verify_sha256, DownloadError};
pub use hardware::{HardwareInfo, Tier};
pub use registry::{Lang, ModelRole, ModelSpec, Registry};
