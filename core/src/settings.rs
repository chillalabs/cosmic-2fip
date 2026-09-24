//! User preferences, persisted as JSON next to the favorites.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Hide dotfiles (names starting with `.`) from the file listings.
    pub hide_hidden_files: bool,
    /// UI language: "system" (follow the desktop), "en", "es" (Spain) or
    /// "es-419" (Latin America).
    pub language: String,
    pub icon_style: IconStyle,
    /// Show previews of images, PDFs, videos, ... instead of type icons.
    pub show_thumbnails: bool,
    /// Text size of file and folder names in the listings.
    pub name_font_size: FontSize,
    /// List view: show the extension in its own "Ext" column (like Total
    /// Commander) instead of as part of the name.
    pub separate_extension: bool,
}

/// Text size for file and folder names; smaller sizes fit more files on
/// screen (the list's rows get shorter too).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontSize {
    /// libcosmic's normal body text size.
    #[default]
    Default,
    Small,
    Smaller,
    Tiny,
}

impl FontSize {
    /// Size in pixels.
    pub fn px(self) -> u16 {
        match self {
            FontSize::Default => 14,
            FontSize::Small => 13,
            FontSize::Smaller => 12,
            FontSize::Tiny => 11,
        }
    }
}

/// How a pane lays out its files (saved per pane in the session).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    /// Rows with Name / Size / Modified columns.
    #[default]
    List,
    /// Large icons with the name below, wrapping to fill the width.
    Grid,
}

/// How file and folder icons are drawn in the listings.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconStyle {
    /// Full-color icons from the icon theme, like COSMIC Files.
    #[default]
    Colorful,
    /// Single-color symbolic icons.
    Monochrome,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hide_hidden_files: true,
            language: "system".to_string(),
            icon_style: IconStyle::default(),
            show_thumbnails: true,
            name_font_size: FontSize::default(),
            separate_extension: true,
        }
    }
}

/// Loads the saved settings, falling back to defaults if none have been
/// saved yet or the file can't be parsed. Fields missing from an older file
/// get their default value.
pub fn load() -> Settings {
    load_from(&settings_path())
}

/// Persists `settings`, creating the config directory if needed.
pub fn save(settings: &Settings) -> Result<(), String> {
    save_to(&settings_path(), settings)
}

fn load_from(path: &Path) -> Settings {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

fn save_to(path: &Path, settings: &Settings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(settings)
        .map_err(|err| format!("failed to serialize settings: {err}"))?;
    fs::write(path, json).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

/// `$XDG_CONFIG_HOME/cosmic-commander`, or `~/.config/cosmic-commander`.
pub(crate) fn config_dir() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("cosmic-commander");
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    home.join(".config").join("cosmic-commander")
}

/// Whether `name` is a hidden file by Unix convention.
pub fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load_from(&dir.path().join("settings.json")),
            Settings::default()
        );
    }

    #[test]
    fn round_trips_through_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("settings.json");
        let settings = Settings {
            hide_hidden_files: false,
            language: "en".to_string(),
            icon_style: IconStyle::Monochrome,
            show_thumbnails: false,
            name_font_size: FontSize::Smaller,
            separate_extension: false,
        };

        save_to(&path, &settings).unwrap();

        assert_eq!(load_from(&path), settings);
    }

    #[test]
    fn missing_fields_get_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, br#"{ "hide_hidden_files": false }"#).unwrap();

        let loaded = load_from(&path);

        assert!(!loaded.hide_hidden_files);
        assert_eq!(loaded.language, "system");
        assert_eq!(loaded.icon_style, IconStyle::Colorful);
        assert_eq!(loaded.name_font_size, FontSize::Default);
        assert!(loaded.separate_extension);
    }

    #[test]
    fn font_sizes_get_smaller_and_default_matches_libcosmic() {
        assert_eq!(FontSize::Default.px(), 14);
        let sizes = [FontSize::Default, FontSize::Small, FontSize::Smaller, FontSize::Tiny];
        assert!(sizes.windows(2).all(|pair| pair[0].px() > pair[1].px()));
    }

    #[test]
    fn dotfiles_are_hidden() {
        assert!(is_hidden(".bashrc"));
        assert!(!is_hidden("notes.txt"));
    }
}
