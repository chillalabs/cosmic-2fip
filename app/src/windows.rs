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

/// File and folder icons as Windows Explorer shows them (48 px, from the
/// system image list). Cached: by extension for most files, by path for
/// files and folders with an icon of their own (programs, shortcuts,
/// Documents, ...). `grey` gives the Monochrome style's version.
pub fn shell_icon(path: &Path, is_dir: bool, grey: bool) -> Option<cosmic::widget::icon::Handle> {
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static CACHE: RefCell<HashMap<(String, bool), Option<cosmic::widget::icon::Handle>>> =
            RefCell::new(HashMap::new());
    }

    let (key, query) = icon_query(path, is_dir);
    if let Some(cached) = CACHE.with(|cache| cache.borrow().get(&(key.clone(), grey)).cloned()) {
        return cached;
    }
    let handle = shell_icon_pixels(&query).map(|(width, height, mut pixels)| {
        if grey {
            make_grey(&mut pixels);
        }
        cosmic::widget::icon::from_raster_pixels(width, height, pixels)
    });
    CACHE.with(|cache| cache.borrow_mut().insert((key, grey), handle.clone()));
    handle
}

/// What to ask Windows for, and the cache key.
enum IconQuery {
    /// The icon of the real file or folder at this path.
    Path(std::path::PathBuf),
    /// The icon for any file named like this (only the extension matters).
    Name(String),
    Folder,
}

fn icon_query(path: &Path, is_dir: bool) -> (String, IconQuery) {
    // Server paths (sftp://...) only have a name to go by.
    let local = !fs_ops::vfs::is_remote(path);
    if is_dir {
        let special = local && path.join("desktop.ini").is_file();
        return if special {
            (
                path.to_string_lossy().into_owned(),
                IconQuery::Path(path.to_path_buf()),
            )
        } else {
            ("<folder>".to_string(), IconQuery::Folder)
        };
    }
    let ext = path
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    const OWN_ICON: [&str; 6] = ["exe", "lnk", "ico", "url", "msc", "cpl"];
    if local && OWN_ICON.contains(&ext.as_str()) {
        (
            path.to_string_lossy().into_owned(),
            IconQuery::Path(path.to_path_buf()),
        )
    } else {
        (format!("*.{ext}"), IconQuery::Name(format!("file.{ext}")))
    }
}

/// The 48 px icon as RGBA pixels.
fn shell_icon_pixels(query: &IconQuery) -> Option<(u32, u32, Vec<u8>)> {
    use windows_sys::core::GUID;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
    };
    use windows_sys::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
    use windows_sys::Win32::UI::Controls::{ImageList_GetIcon, ILD_TRANSPARENT};
    use windows_sys::Win32::UI::Shell::{
        SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_SYSICONINDEX, SHGFI_USEFILEATTRIBUTES,
        SHIL_EXTRALARGE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::DestroyIcon;

    const IID_IIMAGELIST: GUID = GUID::from_u128(0x46eb5926_582e_4017_9fdf_e8998daa0950);
    thread_local! {
        // The shell needs COM on the calling thread (once is enough).
        static COM: () = {
            // SAFETY: initializes COM for this thread; never undone.
            unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
        };
    }
    COM.with(|_| ());

    let (name, attributes, flags) = match query {
        IconQuery::Path(path) => (wide(path.as_os_str()), 0, 0),
        IconQuery::Name(name) => (
            wide(name.as_ref()),
            FILE_ATTRIBUTE_NORMAL,
            SHGFI_USEFILEATTRIBUTES,
        ),
        IconQuery::Folder => (
            wide("folder".as_ref()),
            FILE_ATTRIBUTE_DIRECTORY,
            SHGFI_USEFILEATTRIBUTES,
        ),
    };
    let mut info = SHFILEINFOW::default();
    // SAFETY: `name` is NUL-terminated and `info` is a valid SHFILEINFOW;
    // the icon handle from the image list is destroyed after copying.
    unsafe {
        let found = SHGetFileInfoW(
            name.as_ptr(),
            attributes,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX | flags,
        );
        if found == 0 {
            return None;
        }
        let mut list: *mut core::ffi::c_void = std::ptr::null_mut();
        if SHGetImageList(SHIL_EXTRALARGE as i32, &IID_IIMAGELIST, &mut list) < 0 || list.is_null()
        {
            return None;
        }
        let icon = ImageList_GetIcon(list as isize, info.iIcon, ILD_TRANSPARENT);
        if icon.is_null() {
            return None;
        }
        let pixels = icon_to_rgba(icon);
        DestroyIcon(icon);
        pixels
    }
}

/// Copies an icon's color bitmap as RGBA. Old icons without an alpha
/// channel get their transparency from the icon's mask.
///
/// # Safety
/// `icon` must be a valid icon handle.
unsafe fn icon_to_rgba(
    icon: windows_sys::Win32::UI::WindowsAndMessaging::HICON,
) -> Option<(u32, u32, Vec<u8>)> {
    use windows_sys::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

    let mut info = ICONINFO::default();
    if unsafe { GetIconInfo(icon, &mut info) } == 0 {
        return None;
    }
    let read = |bitmap: HBITMAP| -> Option<(u32, u32, Vec<u8>)> {
        let mut header = BITMAP::default();
        let size = std::mem::size_of::<BITMAP>() as i32;
        if bitmap.is_null()
            || unsafe { GetObjectW(bitmap, size, &mut header as *mut BITMAP as *mut _) } == 0
        {
            return None;
        }
        let (width, height) = (header.bmWidth, header.bmHeight.abs());
        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            // Negative: rows top to bottom.
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..Default::default()
        };
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let dc = unsafe { GetDC(std::ptr::null_mut()) };
        let lines = unsafe {
            GetDIBits(
                dc,
                bitmap,
                0,
                height as u32,
                pixels.as_mut_ptr() as *mut _,
                &mut bmi,
                DIB_RGB_COLORS,
            )
        };
        unsafe { ReleaseDC(std::ptr::null_mut(), dc) };
        (lines != 0).then_some((width as u32, height as u32, pixels))
    };
    let color = read(info.hbmColor);
    let mask = read(info.hbmMask);
    unsafe {
        DeleteObject(info.hbmColor);
        DeleteObject(info.hbmMask);
    }
    let (width, height, mut pixels) = color?;
    // Windows gives BGRA; the image wants RGBA.
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    if pixels.chunks_exact(4).all(|pixel| pixel[3] == 0) {
        let Some((_, _, mask)) = mask else {
            return None;
        };
        // Mask: black = icon, white = transparent.
        for (pixel, mask) in pixels.chunks_exact_mut(4).zip(mask.chunks_exact(4)) {
            pixel[3] = if mask[0] == 0 { 255 } else { 0 };
        }
    }
    Some((width, height, pixels))
}

/// Grey version of RGBA pixels (same lightness).
fn make_grey(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let [r, g, b] = [pixel[0], pixel[1], pixel[2]].map(f32::from);
        let grey = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
        pixel[0] = grey;
        pixel[1] = grey;
        pixel[2] = grey;
    }
}
