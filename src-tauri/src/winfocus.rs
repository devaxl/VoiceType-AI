//! Best-effort foreground-window + secure-field inspection used by the inject guards.
//!
//! Windows-only; other platforms return neutral values so the focus/secure guards simply no-op
//! until a native implementation is added (macOS via the Accessibility API — see PRD §9).

/// An opaque id for the current foreground window (its HWND address on Windows, 0 elsewhere).
#[cfg(windows)]
pub fn foreground_window_id() -> i64 {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    unsafe { GetForegroundWindow() as isize as i64 }
}

#[cfg(not(windows))]
pub fn foreground_window_id() -> i64 {
    0
}

/// True if the focused control is a classic Win32 password field (`EDIT` + `ES_PASSWORD`).
///
/// Best-effort only: it does NOT detect password fields in browsers/Electron/UWP apps (which
/// render their own controls). Any failure conservatively returns `false` (treat as not secure).
#[cfg(windows)]
pub fn focused_is_secure() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GetWindowLongW,
        GetWindowThreadProcessId, GUITHREADINFO,
    };

    const GWL_STYLE: i32 = -16;
    const ES_PASSWORD: u32 = 0x0020;

    unsafe {
        let foreground = GetForegroundWindow();
        if foreground.is_null() {
            return false;
        }

        // Read the focused control for the foreground window's thread without AttachThreadInput.
        let thread_id = GetWindowThreadProcessId(foreground, std::ptr::null_mut());
        let mut info: GUITHREADINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
        if GetGUIThreadInfo(thread_id, &mut info) == 0 {
            return false;
        }

        let focus = info.hwndFocus;
        if focus.is_null() {
            return false;
        }

        let mut class_buf = [0u16; 64];
        let len = GetClassNameW(focus, class_buf.as_mut_ptr(), class_buf.len() as i32);
        if len <= 0 {
            return false;
        }
        let class = String::from_utf16_lossy(&class_buf[..len as usize]);
        let style = GetWindowLongW(focus, GWL_STYLE) as u32;

        class.eq_ignore_ascii_case("Edit") && (style & ES_PASSWORD) != 0
    }
}

#[cfg(not(windows))]
pub fn focused_is_secure() -> bool {
    false
}
