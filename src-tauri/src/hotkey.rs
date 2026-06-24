use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

use crate::pipeline;
use crate::state::{AppState, Status};
use crate::winfocus;

/// The default global hotkey: Alt+Shift+D (lower collision risk than Ctrl+Shift+Space).
pub fn default_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::KeyD)
}

/// Plugin handler. Tap-to-toggle: we act on key *press* only and ignore the release.
pub fn handle_shortcut(app: &AppHandle, state: ShortcutState) {
    if state == ShortcutState::Pressed {
        toggle(app);
    }
}

/// Toggle the dictation lifecycle. Also exposed to the UI via the `trigger_toggle` command.
pub fn toggle(app: &AppHandle) {
    let app_state = app.state::<AppState>();
    let current = *app_state.status.lock().unwrap();

    match current {
        Status::Idle => start_recording(app, &app_state),

        Status::Recording => {
            let recording = {
                let mut recorder = app_state.recorder.lock().unwrap();
                recorder.stop()
            };
            let my_gen = app_state.generation.load(Ordering::SeqCst);
            let target_window = *app_state.target_window.lock().unwrap();
            *app_state.status.lock().unwrap() = Status::Processing;
            let _ = app.emit("status", Status::Processing.as_str());

            match recording {
                Ok(rec) => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        pipeline::run(app_handle, rec, my_gen, target_window).await;
                    });
                }
                Err(e) => {
                    *app_state.status.lock().unwrap() = Status::Idle;
                    let _ = app.emit("error", e.to_string());
                    let _ = app.emit("status", Status::Idle.as_str());
                }
            }
        }

        // Cancel-prior: a press during processing starts a fresh capture. start_recording bumps
        // the generation, which makes the in-flight pipeline discard its result before injecting.
        Status::Processing => start_recording(app, &app_state),
    }
}

fn start_recording(app: &AppHandle, app_state: &AppState) {
    {
        let mut recorder = app_state.recorder.lock().unwrap();
        if let Err(e) = recorder.start() {
            let _ = app.emit("error", e.to_string());
            return;
        }
    }
    app_state.generation.fetch_add(1, Ordering::SeqCst);
    *app_state.target_window.lock().unwrap() = winfocus::foreground_window_id();
    *app_state.status.lock().unwrap() = Status::Recording;
    let _ = app.emit("status", Status::Recording.as_str());
}
