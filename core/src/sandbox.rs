//! Running as a Flatpak. Inside the sandbox `XDG_CONFIG_HOME` and
//! `XDG_CACHE_HOME` point into `~/.var/app/<id>/`, but 2fip has access to the
//! whole home folder (it's a file manager), so it keeps using the normal
//! `~/.config` and `~/.cache`: settings, favorites and thumbnails stay shared
//! with a regular install and with other file managers.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Whether 2fip runs inside a Flatpak sandbox.
pub fn in_flatpak() -> bool {
    static IN_FLATPAK: OnceLock<bool> = OnceLock::new();
    *IN_FLATPAK.get_or_init(|| Path::new("/.flatpak-info").exists())
}

/// The user's home folder: `$HOME`, or `%USERPROFILE%` on Windows.
pub fn home_dir() -> PathBuf {
    #[cfg(windows)]
    let (variable, fallback) = ("USERPROFILE", "C:\\");
    #[cfg(not(windows))]
    let (variable, fallback) = ("HOME", "/");
    non_empty_var(variable).unwrap_or_else(|| PathBuf::from(fallback))
}

fn non_empty_var(variable: &str) -> Option<PathBuf> {
    std::env::var_os(variable)
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
}

/// The folder for app settings: `$XDG_CONFIG_HOME` or `~/.config` (always
/// `~/.config` in a Flatpak); `%APPDATA%` on Windows.
#[cfg(not(windows))]
pub fn config_home() -> PathBuf {
    xdg_home("XDG_CONFIG_HOME", ".config")
}

/// The folder for caches: `$XDG_CACHE_HOME` or `~/.cache` (always `~/.cache`
/// in a Flatpak); `%LOCALAPPDATA%` on Windows.
#[cfg(not(windows))]
pub fn cache_home() -> PathBuf {
    xdg_home("XDG_CACHE_HOME", ".cache")
}

#[cfg(not(windows))]
fn xdg_home(variable: &str, default: &str) -> PathBuf {
    if !in_flatpak() {
        if let Some(dir) = non_empty_var(variable) {
            return dir;
        }
    }
    home_dir().join(default)
}

#[cfg(windows)]
pub fn config_home() -> PathBuf {
    non_empty_var("APPDATA").unwrap_or_else(|| home_dir().join("AppData\\Roaming"))
}

#[cfg(windows)]
pub fn cache_home() -> PathBuf {
    non_empty_var("LOCALAPPDATA").unwrap_or_else(|| home_dir().join("AppData\\Local"))
}

/// A command that runs `program` on the host system: through
/// `flatpak-spawn --host` in a Flatpak (2fip has the
/// `org.freedesktop.Flatpak` permission for this), directly otherwise. Used
/// for the terminal, other apps, `xdg-mime` / `xdg-open` and thumbnailers,
/// none of which exist inside the sandbox.
pub fn host_command(program: impl AsRef<OsStr>) -> Command {
    if in_flatpak() {
        let mut command = Command::new("flatpak-spawn");
        command.arg("--host").arg(program);
        command
    } else {
        Command::new(program)
    }
}

/// [`host_command`] starting in the folder `dir`.
pub fn host_command_in(program: impl AsRef<OsStr>, dir: &Path) -> Command {
    if in_flatpak() {
        let mut command = Command::new("flatpak-spawn");
        command
            .arg("--host")
            .arg(format!("--directory={}", dir.display()))
            .arg(program);
        command
    } else {
        let mut command = Command::new(program);
        command.current_dir(dir);
        command
    }
}

/// Where the host's system path `path` (e.g. `/usr/share/...`) is visible:
/// under `/run/host` in a Flatpak (the sandbox has its own `/usr`). Paths in
/// the home folder are the same inside and out.
pub fn host_path(path: &Path) -> PathBuf {
    let system = ["/usr", "/bin", "/sbin", "/lib", "/etc", "/opt"];
    if in_flatpak() && system.iter().any(|dir| path.starts_with(dir)) {
        Path::new("/run/host").join(path.strip_prefix("/").unwrap_or(path))
    } else {
        path.to_path_buf()
    }
}

/// Whether `program` (a name or an absolute path) is installed on the host.
pub fn host_program_exists(program: &str) -> bool {
    if program.contains('/') {
        return host_path(Path::new(program)).is_file();
    }
    let dirs: Vec<PathBuf> = if in_flatpak() {
        ["/usr/local/bin", "/usr/bin", "/bin"]
            .iter()
            .map(|dir| host_path(Path::new(dir)))
            .collect()
    } else {
        std::env::var_os("PATH")
            .map(|paths| std::env::split_paths(&paths).collect())
            .unwrap_or_default()
    };
    dirs.iter().any(|dir| dir.join(program).is_file())
}

/// The host's data folders (`~/.local/share`, Flatpak exports, `/usr/share`),
/// most important first: where `.desktop` files, thumbnailers and app icons
/// live. Outside a Flatpak, `$XDG_DATA_HOME` and `$XDG_DATA_DIRS`.
pub fn host_data_dirs() -> Vec<PathBuf> {
    let home = home_dir();
    if in_flatpak() {
        return vec![
            home.join(".local/share"),
            home.join(".local/share/flatpak/exports/share"),
            PathBuf::from("/var/lib/flatpak/exports/share"),
            host_path(Path::new("/usr/local/share")),
            host_path(Path::new("/usr/share")),
        ];
    }
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    let data_dirs = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|dirs| !dirs.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_string());
    std::iter::once(data_home)
        .chain(data_dirs.split(':').map(PathBuf::from))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outside_a_flatpak_commands_and_paths_are_unchanged() {
        // The tests never run inside a Flatpak.
        assert!(!in_flatpak());
        assert_eq!(
            host_path(Path::new("/usr/share/x")),
            PathBuf::from("/usr/share/x")
        );
        assert_eq!(host_command("ls").get_program(), "ls");
        assert!(host_program_exists("sh"));
    }
}
