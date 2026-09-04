//! Shared application state.

use me_polish::PolishConfig;
use parking_lot::Mutex;
use std::path::PathBuf;

#[derive(Default)]
pub struct AppState {
    /// Currently open project (path is authoritative; Project owns its data).
    pub open_project: Mutex<Option<PathBuf>>,
    /// Polish endpoint config; api_key is injected from the OS keyring at
    /// call time and is NEVER persisted here.
    pub polish: Mutex<PolishConfig>,
}
