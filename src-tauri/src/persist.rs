//! On-disk persistence for [`AppConfig`] (JSON in the OS app-config dir).

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::config::AppConfig;

fn config_file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("config.json"))
}

/// Load saved config, or fall back to defaults if it's missing/unreadable. A corrupt file is
/// moved aside (`.json.bak`) so a bad write can't permanently wedge settings. `#[serde(default)]`
/// on `AppConfig` means older files missing newly-added fields still load (forward migration).
pub fn load(app: &AppHandle) -> AppConfig {
    let Some(path) = config_file(app) else {
        return AppConfig::default();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return AppConfig::default();
    };
    match serde_json::from_slice::<AppConfig>(&bytes) {
        Ok(config) => config,
        Err(_) => {
            let _ = std::fs::rename(&path, path.with_extension("json.bak"));
            AppConfig::default()
        }
    }
}

/// Persist config to disk (best-effort; failures are non-fatal and just logged).
pub fn save(app: &AppHandle, config: &AppConfig) {
    let Some(path) = config_file(app) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(config) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                eprintln!("failed to save config: {e}");
            }
        }
        Err(e) => eprintln!("failed to serialize config: {e}"),
    }
}
