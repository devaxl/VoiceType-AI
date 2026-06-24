use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, Manager};

use crate::audio::{self, Recording};
use crate::error::{AppError, Result};
use crate::state::{AppState, Status};
use crate::winfocus;
use crate::{inject, refine, secrets, stt};

/// Run the full dictation pipeline for one recording, then return to `Idle` (unless a newer
/// dictation has superseded this one, in which case we stay out of its way).
pub async fn run(app: AppHandle, recording: Recording, my_gen: u64, target_window: i64) {
    let result = run_inner(&app, recording, my_gen, target_window).await;

    let state = app.state::<AppState>();
    if state.generation.load(Ordering::SeqCst) != my_gen {
        // Superseded by a newer dictation (cancel-prior) — it owns the status now.
        return;
    }
    if let Err(e) = result {
        let _ = app.emit("error", e.to_string());
    }
    *state.status.lock().unwrap() = Status::Idle;
    let _ = app.emit("status", Status::Idle.as_str());
}

async fn run_inner(
    app: &AppHandle,
    recording: Recording,
    my_gen: u64,
    target_window: i64,
) -> Result<()> {
    if audio::is_too_short(&recording) {
        return Err(AppError::NoSpeech);
    }

    let wav = audio::to_wav(&recording)?;
    let api_key = secrets::get_api_key()?;

    let (system_prompt, vocabulary, stt_model, refine_model) = {
        let state = app.state::<AppState>();
        let cfg = state.config.lock().unwrap();
        (
            cfg.active_prompt(),
            cfg.vocabulary.clone(),
            cfg.stt_model.clone(),
            cfg.refine_model.clone(),
        )
    };

    // 1. Speech-to-text (with the custom-vocabulary hint).
    let transcript = stt::transcribe(&api_key, &stt_model, wav, &vocabulary).await?;
    if is_empty_or_hallucination(&transcript) {
        return Err(AppError::NoSpeech);
    }

    // 2. Refinement, with a fallback to the raw transcript on any failure.
    let final_text = match refine::refine(&api_key, &refine_model, &system_prompt, &transcript).await
    {
        Ok(refined) if !refined.trim().is_empty() => refined,
        _ => transcript,
    };

    // 3a. Cancel-prior: bail before injecting if a newer dictation has started.
    if app.state::<AppState>().generation.load(Ordering::SeqCst) != my_gen {
        let _ = app.emit("info", "Previous dictation canceled.");
        return Ok(());
    }

    // 3b. Focus-verify guard: only auto-paste if focus is still on the original window.
    let now_window = winfocus::foreground_window_id();
    if target_window != 0 && now_window != target_window {
        inject::copy_to_clipboard(&final_text)?;
        let _ = app.emit("result", &final_text);
        let _ = app.emit(
            "info",
            "Focus changed — text copied to clipboard, press Ctrl+V to paste.",
        );
        return Ok(());
    }

    // 3c. Secure-field guard: type directly (never via clipboard) into a password field.
    if winfocus::focused_is_secure() {
        let to_type = final_text.clone();
        tauri::async_runtime::spawn_blocking(move || inject::type_text(&to_type))
            .await
            .map_err(|e| AppError::Inject(e.to_string()))??;
        let _ = app.emit("result", &final_text);
        let _ = app.emit("info", "Secure field — typed directly (clipboard bypassed).");
        return Ok(());
    }

    // 3d. Normal path: clipboard-paste injection with one retry, then a copy-to-clipboard
    // "paste manually" fallback so a transient inject failure never loses the user's words.
    if try_inject(&final_text).await.is_err() && try_inject(&final_text).await.is_err() {
        inject::copy_to_clipboard(&final_text)?;
        let _ = app.emit("result", &final_text);
        let _ = app.emit(
            "info",
            "Couldn't paste automatically — copied to clipboard, press Ctrl+V.",
        );
        return Ok(());
    }

    let _ = app.emit("result", &final_text);
    Ok(())
}

/// Run one blocking clipboard-paste attempt off the async runtime.
async fn try_inject(text: &str) -> Result<()> {
    let owned = text.to_string();
    tauri::async_runtime::spawn_blocking(move || inject::inject(&owned))
        .await
        .map_err(|e| AppError::Inject(e.to_string()))?
}

/// Conservative guard against Whisper-family phantom phrases emitted on near-silent audio.
/// Deliberately small to avoid dropping legitimate short dictation (e.g. a bare "thank you").
fn is_empty_or_hallucination(text: &str) -> bool {
    let normalized = text
        .trim()
        .trim_end_matches(|c| c == '.' || c == '!' || c == '?')
        .trim()
        .to_lowercase();

    if normalized.is_empty() {
        return true;
    }

    const GHOSTS: &[&str] = &[
        "thanks for watching",
        "thank you for watching",
        "please subscribe",
        "subtitles by the amara.org community",
        "subtitles by the amara org community",
    ];
    GHOSTS.contains(&normalized.as_str())
}
