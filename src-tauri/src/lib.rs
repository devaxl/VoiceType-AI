mod audio;
mod commands;
mod config;
mod error;
mod hotkey;
mod http;
mod inject;
mod macperm;
mod persist;
mod pipeline;
mod refine;
mod secrets;
mod state;
mod stt;
mod tray;
mod winfocus;

use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    hotkey::handle_shortcut(app, event.state());
                })
                .build(),
        )
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::has_api_key,
            commands::set_api_key,
            commands::get_config,
            commands::set_profiles,
            commands::set_active_profile,
            commands::set_vocabulary,
            commands::get_status,
            commands::get_hotkey,
            commands::set_hotkey,
            commands::trigger_toggle,
        ])
        .setup(|app| {
            // Load persisted settings (replacing in-memory defaults) before anything reads them.
            *app.state::<AppState>().config.lock().unwrap() = persist::load(app.handle());

            // Register the saved global dictation hotkey, falling back to the default if the
            // stored accelerator is somehow invalid.
            let initial_hotkey = app.state::<AppState>().config.lock().unwrap().hotkey.clone();
            if app
                .global_shortcut()
                .register(initial_hotkey.as_str())
                .is_err()
            {
                app.global_shortcut().register(hotkey::default_shortcut())?;
            }

            // System-tray icon with a Settings/Quit menu.
            tray::setup_tray(app.handle())?;

            // Run in the background: closing the window HIDES it to the tray instead of
            // quitting. The global hotkey keeps working while hidden; the tray "Quit"
            // menu item is the only thing that fully exits the app.
            if let Some(window) = app.get_webview_window("main") {
                let win = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win.hide();
                    }
                });
            }

            // On macOS, ask for microphone access up front so the system prompt appears at launch
            // (with the Info.plist usage string) instead of silently capturing nothing on the
            // first dictation. No-op on other platforms.
            macperm::request_microphone_access();

            // Floating status HUD: a small transparent, click-through, always-on-top window that
            // shows recording/processing/success/error feedback even when the main window is hidden.
            // Transparency on macOS relies on the `macos-private-api` feature + `macOSPrivateApi`
            // config flag (both enabled); without them a borderless window renders as a solid
            // white rectangle.
            {
                if let Ok(hud) = tauri::WebviewWindowBuilder::new(
                    app.handle(),
                    "hud",
                    tauri::WebviewUrl::App("index.html".into()),
                )
                .title("VoiceType HUD")
                .inner_size(240.0, 72.0)
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .focused(false)
                .resizable(false)
                .shadow(false)
                .visible(true)
                .build()
                {
                    let _ = hud.set_ignore_cursor_events(true);
                    if let Ok(Some(monitor)) = hud.primary_monitor() {
                        let size = monitor.size();
                        let scale = monitor.scale_factor();
                        let w = (240.0 * scale) as i32;
                        let h = (72.0 * scale) as i32;
                        let x = (size.width as i32 - w) / 2;
                        let y = size.height as i32 - h - (90.0 * scale) as i32;
                        let _ = hud.set_position(tauri::PhysicalPosition::new(x, y));
                    }
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running VoiceType AI");
}
