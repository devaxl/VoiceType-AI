use serde::{Serialize, Serializer};
use thiserror::Error;

/// All recoverable failures in the dictation pipeline.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("no microphone found — connect an input device and grant Microphone access in System Settings → Privacy & Security")]
    NoInputDevice,

    #[error("audio error: {0}")]
    Audio(String),

    #[error("no speech detected — speak a little longer, or check Microphone access in System Settings → Privacy & Security")]
    NoSpeech,

    #[error("missing {0} API key — set it in Settings")]
    MissingApiKey(&'static str),

    #[error("network error: {0}")]
    Network(String),

    #[error("transcription failed: {0}")]
    Stt(String),

    #[error("refinement failed: {0}")]
    Refine(String),

    #[error("injection failed: {0}")]
    Inject(String),

    #[error("clipboard error: {0}")]
    Clipboard(String),

    #[error("keyring error: {0}")]
    Keyring(String),
}

/// Serialize as a plain string so the message surfaces cleanly in the frontend.
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
