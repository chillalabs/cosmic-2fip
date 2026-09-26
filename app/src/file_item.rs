use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use cosmic::iced::Length;
use cosmic::widget::{icon, table, Icon};
use fs_ops::settings::{FontSize, IconStyle};
use fs_ops::user_dirs::UserDir;
use fs_ops::{DirEntry, EntryKind};

use crate::fl;

/// Display preferences applied when building a listing.
#[derive(Debug, Clone)]
pub struct ListingOptions {
    pub hide_hidden: bool,
    pub icon_style: IconStyle,
    /// Well-known folders (Documents, Downloads, ...) that get their own icon.
    pub user_dirs: Arc<HashMap<PathBuf, UserDir>>,
    /// Show content previews (images, PDFs, videos, ...) instead of icons.
    pub show_thumbnails: bool,
    /// Text size of the names (and, in the list, the other columns).
    pub font_size: FontSize,
    /// List view: extension in its own column (else part of the name).
    pub separate_extension: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Column {
    #[default]
    Name,
    /// The file's extension (after the last dot), shown separately like in
    /// Total Commander.
    Ext,
    Size,
    Modified,
}

impl std::fmt::Display for Column {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            Column::Name => fl!("column-name"),
            Column::Ext => fl!("column-ext"),
            Column::Size => fl!("column-size"),
            Column::Modified => fl!("column-modified"),
        })
    }
}

impl table::ItemCategory for Column {
    fn width(&self) -> Length {
        match self {
            Column::Name => Length::Fill,
            Column::Ext => Length::Fixed(70.0),
            Column::Size => Length::Fixed(120.0),
            Column::Modified => Length::Fixed(180.0),
        }
    }
}

pub struct FileItem {
    pub path: PathBuf,
    pub kind: EntryKind,
    name: String,
    /// `name` split for the Name / Ext columns: files only, and only when
    /// there's an extension (folders and `.bashrc` keep the whole name).
    stem: String,
    ext: String,
    size: u64,
    modified: Option<SystemTime>,
    /// MIME type guessed from the extension (files only), e.g. `image/png`.
    mime: Option<String>,
    icon: icon::Handle,
}

impl FileItem {
    /// The name without its extension, for the Name column when the
    /// extension has its own column.
    pub fn stem(&self) -> &str {
        &self.stem
    }

    pub fn is_dir(&self) -> bool {
        matches!(self.kind, EntryKind::Dir)
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The themed icon for this entry's folder kind or file type.
    pub fn icon(&self) -> &icon::Handle {
        &self.icon
    }

    pub fn mime(&self) -> Option<&str> {
        self.mime.as_deref()
    }

    pub fn modified(&self) -> Option<SystemTime> {
        self.modified
    }
}

impl FileItem {
    pub fn new(entry: DirEntry, options: &ListingOptions) -> Self {
        let mime = match entry.kind {
            EntryKind::File => fs_ops::thumbnail::guess_mime(&entry.path),
            _ => None,
        };
        let (stem, ext) = split_name(&entry.name, entry.kind);
        Self {
            icon: icon_handle(entry.kind, &entry.path, mime.as_deref(), options),
            mime,
            stem,
            ext,
            path: entry.path,
            kind: entry.kind,
            name: entry.name,
            size: entry.size,
            modified: entry.modified,
        }
    }
}

/// The themed icon for an entry: its file type's icon (e.g. `application-pdf`,
/// falling back to the type family like `image-x-generic`), or the special
/// icon of a well-known folder; colorful or monochrome (`-symbolic`).
fn icon_handle(
    kind: EntryKind,
    path: &Path,
    mime: Option<&str>,
    options: &ListingOptions,
) -> icon::Handle {
    let suffix = match options.icon_style {
        IconStyle::Colorful => "",
        IconStyle::Monochrome => "-symbolic",
    };
    if let (EntryKind::File, Some(mime)) = (kind, mime) {
        // Icon themes name type icons after the MIME type, '/' -> '-'.
        let specific = format!("{}{suffix}", mime.replace('/', "-"));
        let family = match mime.split('/').next().unwrap_or_default() {
            "image" => "image-x-generic",
            "audio" => "audio-x-generic",
            "video" => "video-x-generic",
            "font" => "font-x-generic",
            "text" => "text-x-generic",
            _ => "application-x-generic",
        };
        let fallbacks = [family, "text-x-generic"]
            .into_iter()
            .map(|name| format!("{name}{suffix}").into())
            .collect();
        return icon::from_name(specific)
            .fallback(Some(icon::IconFallback::Names(fallbacks)))
            .handle();
    }
    icon::from_name(icon_name(kind, path, options)).handle()
}

/// Picks the themed icon for an entry without a known file type: special
/// icons for the user's well-known folders, in the colorful or monochrome
/// (`-symbolic`) variant.
fn icon_name(kind: EntryKind, path: &Path, options: &ListingOptions) -> &'static str {
    let (colorful, monochrome) = match kind {
        EntryKind::Dir => match options.user_dirs.get(path) {
            Some(UserDir::Home) => ("user-home", "user-home-symbolic"),
            Some(UserDir::Desktop) => ("user-desktop", "user-desktop-symbolic"),
            Some(UserDir::Documents) => ("folder-documents", "folder-documents-symbolic"),
            Some(UserDir::Download) => ("folder-download", "folder-download-symbolic"),
            Some(UserDir::Music) => ("folder-music", "folder-music-symbolic"),
            Some(UserDir::Pictures) => ("folder-pictures", "folder-pictures-symbolic"),
            Some(UserDir::PublicShare) => ("folder-publicshare", "folder-publicshare-symbolic"),
            Some(UserDir::Templates) => ("folder-templates", "folder-templates-symbolic"),
            Some(UserDir::Videos) => ("folder-videos", "folder-videos-symbolic"),
            None => ("folder", "folder-symbolic"),
        },
        EntryKind::File => ("text-x-generic", "text-x-generic-symbolic"),
        // The icon theme has no full-color symlink icon.
        EntryKind::Symlink => (
            "emblem-symbolic-link-symbolic",
            "emblem-symbolic-link-symbolic",
        ),
    };
    match options.icon_style {
        IconStyle::Colorful => colorful,
        IconStyle::Monochrome => monochrome,
    }
}

impl table::ItemInterface<Column> for FileItem {
    fn get_icon(&self, category: Column) -> Option<Icon> {
        if category != Column::Name {
            return None;
        }
        Some(icon::icon(self.icon.clone()))
    }

    fn get_text(&self, category: Column) -> Cow<'static, str> {
        match category {
            Column::Name => self.name.clone().into(),
            Column::Ext => self.ext.clone().into(),
            Column::Size => {
                if self.is_dir() {
                    Cow::Borrowed("")
                } else {
                    format_size(self.size).into()
                }
            }
            Column::Modified => self
                .modified
                .map(format_modified)
                .unwrap_or_default()
                .into(),
        }
    }

    fn compare(&self, other: &Self, category: Column) -> std::cmp::Ordering {
        match category {
            Column::Name => self.name.to_lowercase().cmp(&other.name.to_lowercase()),
            Column::Ext => self.ext.to_lowercase().cmp(&other.ext.to_lowercase()),
            Column::Size => self.size.cmp(&other.size),
            Column::Modified => self.modified.cmp(&other.modified),
        }
    }
}

/// Splits a file name into (name, extension) at the last dot, like Total
/// Commander: `archive.tar.gz` → (`archive.tar`, `gz`). Folders, names without
/// a dot and dotfiles like `.bashrc` keep the whole name with no extension.
fn split_name(name: &str, kind: EntryKind) -> (String, String) {
    if kind == EntryKind::Dir {
        return (name.to_string(), String::new());
    }
    match name.rfind('.') {
        Some(dot) if dot > 0 && dot + 1 < name.len() => {
            (name[..dot].to_string(), name[dot + 1..].to_string())
        }
        _ => (name.to_string(), String::new()),
    }
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

pub fn format_modified(time: SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_names_at_the_last_dot() {
        let file = |name| split_name(name, EntryKind::File);
        assert_eq!(file("report.pdf"), ("report".into(), "pdf".into()));
        assert_eq!(file("archive.tar.gz"), ("archive.tar".into(), "gz".into()));
        assert_eq!(file("README"), ("README".into(), String::new()));
        assert_eq!(file(".bashrc"), (".bashrc".into(), String::new()));
        assert_eq!(
            file("ends-with-dot."),
            ("ends-with-dot.".into(), String::new())
        );
        assert_eq!(
            split_name("photos.2024", EntryKind::Dir),
            ("photos.2024".into(), String::new())
        );
    }
}
