//! Drive letters (`C:\`, `D:\`, ...) on Windows, for each panel's drive
//! menu. Linux has a single root, so there are none.

use std::path::{Component, Path, PathBuf};

/// The drives Windows reports (also removable and network drives), in
/// letter order.
#[cfg(windows)]
pub fn list() -> Vec<PathBuf> {
    // SAFETY: no arguments; returns a bitmask, bit 0 = A:, bit 1 = B:, ...
    let mask = unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() };
    (0..26u8)
        .filter(|bit| mask & (1 << bit) != 0)
        .map(|bit| PathBuf::from(format!("{}:\\", (b'A' + bit) as char)))
        .collect()
}

#[cfg(not(windows))]
pub fn list() -> Vec<PathBuf> {
    Vec::new()
}

/// The drive `path` is on, e.g. `C:\` for `C:\Users` (`None` on Linux and
/// for server paths).
pub fn drive_of(path: &Path) -> Option<PathBuf> {
    match path.components().next()? {
        // Upper case, so a typed `c:\x` still matches the menu's `C:\`.
        Component::Prefix(prefix) => Some(PathBuf::from(format!(
            "{}\\",
            prefix.as_os_str().to_string_lossy().to_uppercase()
        ))),
        _ => None,
    }
}

/// The menu label of a drive: `C:\` → `C:`.
pub fn label(drive: &Path) -> String {
    drive.to_string_lossy().trim_end_matches('\\').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn linux_has_no_drives() {
        assert!(list().is_empty());
        assert_eq!(drive_of(Path::new("/home/user")), None);
    }

    #[test]
    fn labels_drop_the_backslash() {
        assert_eq!(label(Path::new("C:\\")), "C:");
    }
}
