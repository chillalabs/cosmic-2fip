//! Windows-only helpers (Win32 calls) for what Linux does with desktop
//! tools: opening files with their default app, and finding where a file
//! dragged from Explorer was dropped.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::null;

use windows_sys::Win32::Foundation::{POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetAncestor, GetClientRect, GetCursorPos, WindowFromPoint, GA_ROOT, SW_SHOWNORMAL,
};

/// Opens `path` with its default app, like double-clicking it in Explorer.
/// Files without one get Windows' "How do you want to open this file?"
/// dialog.
pub fn open(path: &Path) {
    if !shell_execute(None, path) && !shell_execute(Some("openas"), path) {
        eprintln!("failed to open {}", path.display());
    }
}

/// Runs `verb` (`None` = the file type's default verb) on `path`.
fn shell_execute(verb: Option<&str>, path: &Path) -> bool {
    let file = wide(path.as_os_str());
    let verb = verb.map(|verb| wide(verb.as_ref()));
    let verb_ptr = verb.as_ref().map_or(null(), |verb| verb.as_ptr());
    // SAFETY: both strings are NUL-terminated UTF-16 and outlive the call.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb_ptr,
            file.as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        )
    };
    // Values above 32 mean success.
    result as usize > 32
}

fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
    text.encode_wide().chain(std::iter::once(0)).collect()
}

/// Whether Shift is held down right now. Needed for drops from Explorer:
/// during a drag the window gets no key events.
pub fn shift_held() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_SHIFT};
    // SAFETY: a plain key state query. The top bit means "down".
    unsafe { GetAsyncKeyState(VK_SHIFT as i32) < 0 }
}

/// Whether the mouse pointer is over the right half of the window under it,
/// i.e. over the right panel. `None` if Windows can't tell.
pub fn cursor_over_right_half() -> Option<bool> {
    let mut point = POINT { x: 0, y: 0 };
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: plain Win32 queries writing into the local structs above.
    unsafe {
        if GetCursorPos(&mut point) == 0 {
            return None;
        }
        let window = GetAncestor(WindowFromPoint(point), GA_ROOT);
        if window.is_null()
            || ScreenToClient(window, &mut point) == 0
            || GetClientRect(window, &mut rect) == 0
        {
            return None;
        }
    }
    Some(point.x > (rect.right - rect.left) / 2)
}
