//! Running as a Flatpak. Inside the sandbox `XDG_CONFIG_HOME` and
//! `XDG_CACHE_HOME` point into `~/.var/app/<id>/`, but 2fip has access to the
//! whole home folder (it's a file manager), so it keeps using the normal
//! `~/.config` and `~/.cache`: settings, favorites and thumbnails stay shared
//! with a regular install and with other file managers.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Whether 2fip runs inside a Flatpak sandbox.
pub fn in_flatpak() -> bool {
    static IN_FLATPAK: OnceLock<bool> = OnceLock::new();
    *IN_FLATPAK.get_or_init(|| Path::new("/.flatpak-info").exists())
}

/// The folder for app settings: `$XDG_CONFIG_HOME` or `~/.config` (always
/// `~/.config` in a Flatpak).
pub fn config_home() -> PathBuf {
    xdg_home("XDG_CONFIG_HOME", ".config")
}

/// The folder for caches: `$XDG_CACHE_HOME` or `~/.cache` (always `~/.cache`
/// in a Flatpak).
pub fn cache_home() -> PathBuf {
    xdg_home("XDG_CACHE_HOME", ".cache")
}

fn xdg_home(variable: &str, default: &str) -> PathBuf {
    if !in_flatpak() {
        if let Some(dir) = std::env::var_os(variable).filter(|dir| !dir.is_empty()) {
            return PathBuf::from(dir);
        }
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
        .join(default)
}
