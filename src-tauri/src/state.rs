use std::sync::atomic::AtomicU64;
use std::sync::Mutex;

use crate::audio::Recorder;
use crate::config::AppConfig;

/// The single owner of the dictation lifecycle. Transitions are enforced in `hotkey::toggle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Idle,
    Recording,
    Processing,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Idle => "idle",
            Status::Recording => "recording",
            Status::Processing => "processing",
        }
    }
}

/// Managed application state. Held by Tauri and accessed from commands + the hotkey handler.
pub struct AppState {
    pub status: Mutex<Status>,
    pub recorder: Mutex<Recorder>,
    pub config: Mutex<AppConfig>,
    /// Bumped every time a new recording starts. Lets an in-flight pipeline detect it was
    /// superseded (cancel-prior) and discard its result instead of injecting stale text.
    pub generation: AtomicU64,
    /// Foreground window id captured when recording started (0 = unknown / non-Windows).
    pub target_window: Mutex<i64>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(Status::Idle),
            recorder: Mutex::new(Recorder::default()),
            config: Mutex::new(AppConfig::default()),
            generation: AtomicU64::new(0),
            target_window: Mutex::new(0),
        }
    }
}
