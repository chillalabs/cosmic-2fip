//! The user's well-known folders (Documents, Downloads, ...), as configured
//! in `~/.config/user-dirs.dirs` by `xdg-user-dirs`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserDir {
    Home,
    Desktop,
    Documents,
    Download,
    Music,
    Pictures,
    PublicShare,
    Templates,
    Videos,
}

/// Maps each well-known folder's path to what it is. Missing or unreadable
/// config just yields fewer entries (always at least the home folder).
#[cfg(not(windows))]
pub fn load() -> HashMap<PathBuf, UserDir> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return HashMap::new();
    };
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"))
        .join("user-dirs.dirs");
    let contents = std::fs::read_to_string(config).unwrap_or_default();
    parse(&contents, &home)
}

/// Windows: the standard folders in the user's profile (folders moved
/// elsewhere, e.g. to OneDrive, just show a plain folder icon).
#[cfg(windows)]
pub fn load() -> HashMap<PathBuf, UserDir> {
    let home = crate::sandbox::home_dir();
    let mut dirs = HashMap::from([(home.clone(), UserDir::Home)]);
    for (name, kind) in [
        ("Desktop", UserDir::Desktop),
        ("Documents", UserDir::Documents),
        ("Downloads", UserDir::Download),
        ("Music", UserDir::Music),
        ("Pictures", UserDir::Pictures),
        ("Videos", UserDir::Videos),
    ] {
        dirs.insert(home.join(name), kind);
    }
    dirs
}

/// Parses `user-dirs.dirs` lines like `XDG_DOCUMENTS_DIR="$HOME/Documents"`.
#[cfg_attr(windows, allow(dead_code))]
fn parse(contents: &str, home: &Path) -> HashMap<PathBuf, UserDir> {
    let mut dirs = HashMap::from([(home.to_path_buf(), UserDir::Home)]);
    for line in contents.lines() {
        let Some((key, value)) = line.trim().split_once('=') else {
            continue;
        };
        let kind = match key {
            "XDG_DESKTOP_DIR" => UserDir::Desktop,
            "XDG_DOCUMENTS_DIR" => UserDir::Documents,
            "XDG_DOWNLOAD_DIR" => UserDir::Download,
            "XDG_MUSIC_DIR" => UserDir::Music,
            "XDG_PICTURES_DIR" => UserDir::Pictures,
            "XDG_PUBLICSHARE_DIR" => UserDir::PublicShare,
            "XDG_TEMPLATES_DIR" => UserDir::Templates,
            "XDG_VIDEOS_DIR" => UserDir::Videos,
            _ => continue,
        };
        let value = value.trim().trim_matches('"');
        let path = match value.strip_prefix("$HOME") {
            Some(rest) => home.join(rest.trim_start_matches('/')),
            None => PathBuf::from(value),
        };
        // xdg-user-dirs points unused entries at $HOME itself; keep "Home".
        if path != home {
            dirs.insert(path, kind);
        }
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_home_relative_and_absolute_entries() {
        let home = Path::new("/home/user");
        let contents = r#"
# comment
XDG_DOCUMENTS_DIR="$HOME/Documents"
XDG_MUSIC_DIR="/mnt/music"
XDG_TEMPLATES_DIR="$HOME/"
"#;

        let dirs = parse(contents, home);

        assert_eq!(dirs.get(home), Some(&UserDir::Home));
        assert_eq!(
            dirs.get(Path::new("/home/user/Documents")),
            Some(&UserDir::Documents)
        );
        assert_eq!(dirs.get(Path::new("/mnt/music")), Some(&UserDir::Music));
        assert_eq!(dirs.len(), 3);
    }
}
