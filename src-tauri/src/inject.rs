use std::thread;
use std::time::Duration;

use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};

use crate::error::{AppError, Result};

// Paste shortcut differs per platform: Cmd+V on macOS, Ctrl+V elsewhere.
#[cfg(target_os = "macos")]
const PASTE_MODIFIER: Key = Key::Meta;
#[cfg(not(target_os = "macos"))]
const PASTE_MODIFIER: Key = Key::Control;

// The "V" key of the paste chord. On macOS we use the raw virtual keycode (kVK_ANSI_V = 9)
// rather than Key::Unicode('v'): resolving a character to a keycode routes through the Carbon
// Text Input Source (TSM) APIs, which assert they run on the main thread and abort the whole
// process (SIGTRAP) when called from our background injection thread. A raw keycode skips that
// main-thread-only lookup entirely.
#[cfg(target_os = "macos")]
const PASTE_KEY: Key = Key::Other(9);
#[cfg(not(target_os = "macos"))]
const PASTE_KEY: Key = Key::Unicode('v');

/// Restores the user's previous clipboard contents when dropped — runs even on early return or
/// panic, so we never strand our refined text on the clipboard. Preserves text or, failing that,
/// an image (arboard cannot round-trip RTF/HTML/file lists — a known v0 limitation).
struct ClipboardGuard {
    text: Option<String>,
    image: Option<arboard::ImageData<'static>>,
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Some(text) = self.text.take() {
                let _ = clipboard.set_text(text);
            } else if let Some(image) = self.image.take() {
                let _ = clipboard.set_image(image);
            }
        }
    }
}

/// Insert `text` into the focused field via clipboard-paste, preserving the user's clipboard.
pub fn inject(text: &str) -> Result<()> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| AppError::Clipboard(e.to_string()))?;

    // Snapshot the existing clipboard (text first, else an image) and arm the restore guard.
    let prev_text = clipboard.get_text().ok();
    let prev_image = if prev_text.is_none() {
        clipboard.get_image().ok()
    } else {
        None
    };
    let _guard = ClipboardGuard {
        text: prev_text,
        image: prev_image,
    };

    clipboard
        .set_text(text.to_string())
        .map_err(|e| AppError::Clipboard(e.to_string()))?;
    drop(clipboard); // release our handle before synthesizing the paste

    // Let the OS register the new clipboard contents before pasting.
    thread::sleep(Duration::from_millis(120));

    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| AppError::Inject(e.to_string()))?;
    enigo
        .key(PASTE_MODIFIER, Press)
        .map_err(|e| AppError::Inject(e.to_string()))?;
    enigo
        .key(PASTE_KEY, Click)
        .map_err(|e| AppError::Inject(e.to_string()))?;
    enigo
        .key(PASTE_MODIFIER, Release)
        .map_err(|e| AppError::Inject(e.to_string()))?;

    // Let the target app consume the paste before the guard restores the old clipboard.
    thread::sleep(Duration::from_millis(150));
    Ok(())
}

/// Type `text` directly, character by character, never touching the clipboard. Used for secure
/// (password) fields so dictated secrets don't transit clipboard history / OS clipboard sync.
pub fn type_text(text: &str) -> Result<()> {
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| AppError::Inject(e.to_string()))?;
    enigo
        .text(text)
        .map_err(|e| AppError::Inject(e.to_string()))?;
    Ok(())
}

/// Copy text to the clipboard WITHOUT restoring — the "paste manually" fallback used when
/// auto-inject is unsafe (focus changed and we no longer know the target field).
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| AppError::Clipboard(e.to_string()))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| AppError::Clipboard(e.to_string()))?;
    Ok(())
}
