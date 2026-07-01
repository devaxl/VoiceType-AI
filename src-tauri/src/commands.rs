use tauri::{AppHandle, State};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::config::{AppConfig, Profile, Provider};
use crate::hotkey;
use crate::secrets;
use crate::state::AppState;

#[tauri::command]
pub fn has_api_key(provider: Provider) -> bool {
    secrets::has_api_key(provider)
}

#[tauri::command]
pub fn set_api_key(provider: Provider, key: String) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("API key is empty".into());
    }
    secrets::store_api_key(provider, key).map_err(|e| e.to_string())
}

/// Set the speech-to-text provider (OpenAI or Groq) and its model, then persist.
#[tauri::command]
pub fn set_stt_config(
    app: AppHandle,
    state: State<AppState>,
    provider: Provider,
    model: String,
) -> Result<(), String> {
    let model = model.trim().to_string();
    if model.is_empty() {
        return Err("Model is empty".into());
    }
    let snapshot = {
        let mut cfg = state.config.lock().unwrap();
        cfg.stt_provider = provider;
        cfg.stt_model = model;
        cfg.clone()
    };
    crate::persist::save(&app, &snapshot);
    Ok(())
}

/// Set the refinement provider (OpenAI or Anthropic) and its model, then persist.
#[tauri::command]
pub fn set_refine_config(
    app: AppHandle,
    state: State<AppState>,
    provider: Provider,
    model: String,
) -> Result<(), String> {
    let model = model.trim().to_string();
    if model.is_empty() {
        return Err("Model is empty".into());
    }
    let snapshot = {
        let mut cfg = state.config.lock().unwrap();
        cfg.refine_provider = provider;
        cfg.refine_model = model;
        cfg.clone()
    };
    crate::persist::save(&app, &snapshot);
    Ok(())
}

/// Full config snapshot for the settings UI (profiles, active index, vocabulary, hotkey, …).
#[tauri::command]
pub fn get_config(state: State<AppState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

/// Replace the profile list (and active index) wholesale, then persist.
#[tauri::command]
pub fn set_profiles(
    app: AppHandle,
    state: State<AppState>,
    profiles: Vec<Profile>,
    active: usize,
) -> Result<(), String> {
    if profiles.is_empty() {
        return Err("Keep at least one profile".into());
    }
    let active = active.min(profiles.len() - 1);
    let snapshot = {
        let mut cfg = state.config.lock().unwrap();
        cfg.profiles = profiles;
        cfg.active_profile = active;
        cfg.clone()
    };
    crate::persist::save(&app, &snapshot);
    Ok(())
}

/// Switch the active profile (the quick-switcher path); returns the clamped index.
#[tauri::command]
pub fn set_active_profile(app: AppHandle, state: State<AppState>, index: usize) -> usize {
    let snapshot = {
        let mut cfg = state.config.lock().unwrap();
        let clamped = if cfg.profiles.is_empty() {
            0
        } else {
            index.min(cfg.profiles.len() - 1)
        };
        cfg.active_profile = clamped;
        cfg.clone()
    };
    crate::persist::save(&app, &snapshot);
    snapshot.active_profile
}

#[tauri::command]
pub fn set_vocabulary(app: AppHandle, state: State<AppState>, vocabulary: String) {
    let snapshot = {
        let mut cfg = state.config.lock().unwrap();
        cfg.vocabulary = vocabulary;
        cfg.clone()
    };
    crate::persist::save(&app, &snapshot);
}

#[tauri::command]
pub fn get_status(state: State<AppState>) -> String {
    state.status.lock().unwrap().as_str().to_string()
}

#[tauri::command]
pub fn get_hotkey(state: State<AppState>) -> String {
    state.config.lock().unwrap().hotkey.clone()
}

/// Rebind the global hotkey at runtime. Registers the new accelerator BEFORE unregistering the
/// old one, so an invalid value leaves the existing binding intact.
#[tauri::command]
pub fn set_hotkey(
    app: AppHandle,
    state: State<AppState>,
    accelerator: String,
) -> Result<String, String> {
    let accelerator = accelerator.trim().to_string();
    if accelerator.is_empty() {
        return Err("Hotkey is empty".into());
    }

    let prev = state.config.lock().unwrap().hotkey.clone();
    if prev == accelerator {
        return Ok(accelerator);
    }

    let shortcuts = app.global_shortcut();
    shortcuts
        .register(accelerator.as_str())
        .map_err(|e| format!("Could not set '{accelerator}': {e}"))?;
    let _ = shortcuts.unregister(prev.as_str());

    let snapshot = {
        let mut cfg = state.config.lock().unwrap();
        cfg.hotkey = accelerator.clone();
        cfg.clone()
    };
    crate::persist::save(&app, &snapshot);
    Ok(accelerator)
}

/// Lets the settings window trigger a dictation cycle without the hotkey (handy for testing).
#[tauri::command]
pub fn trigger_toggle(app: AppHandle) {
    hotkey::toggle(&app);
}
