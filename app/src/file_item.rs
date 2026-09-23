use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use cosmic::iced::Length;
use cosmic::widget::{icon, table, Icon};
use fs_ops::settings::IconStyle;
use fs_ops::user_dirs::UserDir;
use fs_ops::{DirEntry, EntryKind};

/// Display preferences applied when building a listing.
#[derive(Debug, Clone)]
pub struct ListingOptions {
    pub hide_hidden: bool,
    pub icon_style: IconStyle,
    /// Well-known folders (Documents, Downloads, ...) that get their own icon.
    pub user_dirs: Arc<HashMap<PathBuf, UserDir>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Column {
    #[default]
    Name,
    Size,
    Modified,
}

impl std::fmt::Display for Column {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Column::Name => "Name",
            Column::Size => "Size",
            Column::Modified => "Modified",
        })
    }
}

impl table::ItemCategory for Column {
    fn width(&self) -> Length {
        match self {
            Column::Name => Length::Fill,
            Column::Size => Length::Fixed(120.0),
            Column::Modified => Length::Fixed(180.0),
        }
    }
}

pub struct FileItem {
    pub path: PathBuf,
    pub kind: EntryKind,
    name: String,
    size: u64,
    modified: Option<SystemTime>,
    icon: &'static str,
}

impl FileItem {
    pub fn is_dir(&self) -> bool {
        matches!(self.kind, EntryKind::Dir)
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The themed icon name picked for this entry (see `ListingOptions`).
    pub fn icon_name(&self) -> &'static str {
        self.icon
    }
}

impl FileItem {
    pub fn new(entry: DirEntry, options: &ListingOptions) -> Self {
        Self {
            icon: icon_name(entry.kind, &entry.path, options),
            path: entry.path,
            kind: entry.kind,
            name: entry.name,
            size: entry.size,
            modified: entry.modified,
        }
    }
}

/// Picks the themed icon for an entry: special icons for the user's
/// well-known folders, in the colorful or monochrome (`-symbolic`) variant.
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
        Some(icon::from_name(self.icon).icon())
    }

    fn get_text(&self, category: Column) -> Cow<'static, str> {
        match category {
            Column::Name => self.name.clone().into(),
            Column::Size => {
                if self.is_dir() {
                    Cow::Borrowed("")
                } else {
                    format_size(self.size).into()
                }
            }
            Column::Modified => self.modified.map(format_modified).unwrap_or_default().into(),
        }
    }

    fn compare(&self, other: &Self, category: Column) -> std::cmp::Ordering {
        match category {
            Column::Name => self.name.to_lowercase().cmp(&other.name.to_lowercase()),
            Column::Size => self.size.cmp(&other.size),
            Column::Modified => self.modified.cmp(&other.modified),
        }
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
