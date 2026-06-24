//! macOS microphone-permission (TCC) helpers.
//!
//! On macOS, `cpal` can open the default input device even when the app has not been granted
//! microphone access — it just receives silence, which downstream surfaces as "no speech
//! detected". To avoid that confusing failure we (a) trigger the system permission prompt at
//! launch via AVFoundation, and (b) expose a cheap status check so the hotkey path can surface
//! a clear, actionable error if access was explicitly denied.
//!
//! Every function is a no-op on non-macOS platforms.

/// Trigger the system microphone-permission prompt if the user hasn't decided yet.
/// Safe to call repeatedly; once a decision exists it does nothing.
pub fn request_microphone_access() {
    #[cfg(target_os = "macos")]
    macos::request_microphone_access();
}

/// True only when microphone access has been explicitly denied or restricted — i.e. recording
/// cannot possibly work until the user changes it in System Settings.
pub fn microphone_denied() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::microphone_denied()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use block2::RcBlock;
    use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};

    fn status() -> AVAuthorizationStatus {
        // SAFETY: reading the framework string constant; it is always linked.
        let media_type = unsafe { AVMediaTypeAudio }
            .expect("AVMediaTypeAudio framework constant is always linked");
        // SAFETY: the class method just reads the current TCC authorization state and has no
        // preconditions.
        unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) }
    }

    pub fn microphone_denied() -> bool {
        let s = status();
        s == AVAuthorizationStatus::Denied || s == AVAuthorizationStatus::Restricted
    }

    pub fn request_microphone_access() {
        // Only prompt when the user hasn't decided. If already authorized/denied, asking again
        // does nothing useful (and never re-prompts once denied).
        if status() != AVAuthorizationStatus::NotDetermined {
            return;
        }
        // SAFETY: reading the framework string constant; it is always linked.
        let media_type = unsafe { AVMediaTypeAudio }
            .expect("AVMediaTypeAudio framework constant is always linked");
        // The completion handler runs on an arbitrary queue; we don't need the result here (the
        // hotkey path re-checks status before recording), so an empty block is fine.
        let handler = RcBlock::new(|_granted: objc2::runtime::Bool| {});
        // SAFETY: presents the standard microphone prompt; the usage string lives in Info.plist.
        unsafe {
            AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &handler);
        }
    }
}
