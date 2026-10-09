//! 2fip's own icon sets, drawn as SVG in the app: the same on every system,
//! no icon theme needed. All three share the file-type colors and symbols:
//!
//! - **Vivid:** bright and flat; pages in the type's color with a white
//!   symbol, a color per special folder.
//! - **Classic** (shown as "Windows style"): yellow folders with a colored badge on
//!   special ones; white pages with a colored symbol.
//! - **Soft** (shown as "macOS style"): light-blue folders with an embossed symbol on
//!   special ones; white pages with a colored symbol and band.
//!
//! The drawings are original; they borrow only the general look. 2fip isn't
//! affiliated with Microsoft or Apple (see the help's Settings page).

use std::collections::HashMap;
use std::sync::OnceLock;

use cosmic::widget::icon;
use fs_ops::user_dirs::UserDir;

/// Which of 2fip's icon sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconSet {
    Vivid,
    Classic,
    Soft,
}

/// A file type's look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileCategory {
    Image,
    Audio,
    Video,
    Pdf,
    Archive,
    Code,
    Spreadsheet,
    Presentation,
    Document,
    Text,
    Font,
    Program,
    DiskImage,
    Other,
}

impl FileCategory {
    /// The category of a file from its MIME type and name.
    pub fn of(mime: Option<&str>, name: &str) -> FileCategory {
        let extension = name
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())
            .unwrap_or_default();
        let by_extension = match extension.as_str() {
            "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "zst" | "7z" | "rar" | "deb" | "rpm"
            | "lz" | "lzma" | "cab" | "jar" => Some(FileCategory::Archive),
            "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "c" | "h" | "cpp" | "hpp" | "cc" | "go"
            | "java" | "kt" | "swift" | "rb" | "php" | "sh" | "bash" | "zsh" | "fish" | "lua"
            | "pl" | "cs" | "dart" | "json" | "toml" | "yaml" | "yml" | "xml" | "html" | "htm"
            | "css" | "scss" | "sql" | "ron" | "ftl" | "ini" | "conf" | "cfg" | "nix" | "vue"
            | "svelte" | "zig" => Some(FileCategory::Code),
            "ods" | "xls" | "xlsx" | "csv" | "tsv" | "numbers" => Some(FileCategory::Spreadsheet),
            "odp" | "ppt" | "pptx" | "key" => Some(FileCategory::Presentation),
            "odt" | "doc" | "docx" | "rtf" | "pages" | "epub" => Some(FileCategory::Document),
            "pdf" => Some(FileCategory::Pdf),
            "iso" | "img" | "dmg" | "qcow2" | "vdi" | "vmdk" => Some(FileCategory::DiskImage),
            "appimage" | "exe" | "msi" | "flatpak" | "flatpakref" | "desktop" | "run" | "bin"
            | "so" => Some(FileCategory::Program),
            "ttf" | "otf" | "woff" | "woff2" => Some(FileCategory::Font),
            "md" | "txt" | "log" | "rst" => Some(FileCategory::Text),
            _ => None,
        };
        if let Some(category) = by_extension {
            return category;
        }
        let Some(mime) = mime else {
            return FileCategory::Other;
        };
        match mime.split('/').next().unwrap_or_default() {
            "image" => FileCategory::Image,
            "audio" => FileCategory::Audio,
            "video" => FileCategory::Video,
            "font" => FileCategory::Font,
            "text" => FileCategory::Text,
            _ if mime == "application/pdf" => FileCategory::Pdf,
            _ => FileCategory::Other,
        }
    }

    /// Its color.
    fn color(self) -> &'static str {
        match self {
            FileCategory::Image => "#9B5CF6",
            FileCategory::Audio => "#FF8A3D",
            FileCategory::Video => "#EC4B7C",
            FileCategory::Pdf => "#E53935",
            FileCategory::Archive => "#B7791F",
            FileCategory::Code => "#22A55B",
            FileCategory::Spreadsheet => "#16935A",
            FileCategory::Presentation => "#F4511E",
            FileCategory::Document => "#3B82F6",
            FileCategory::Text => "#64748B",
            FileCategory::Font => "#0FA3A3",
            FileCategory::Program => "#6366F1",
            FileCategory::DiskImage => "#5B7083",
            FileCategory::Other => "#94A3B8",
        }
    }

    /// Its symbol, on the lower part of a 48×48 page, in the color `{c}`.
    fn symbol(self) -> &'static str {
        match self {
            FileCategory::Image => {
                r#"<circle cx="18" cy="24" r="3" fill="{c}"/><path d="M12 38l8-9 5 5 4-4 7 8z" fill="{c}"/>"#
            }
            FileCategory::Audio => {
                r#"<path d="M21 21v12.2a4 4 0 1 0 2.5 3.7V25.5l9-2.5v8.2a4 4 0 1 0 2.5 3.7V18z" fill="{c}"/>"#
            }
            FileCategory::Video => r#"<path d="M19 21v16l13-8z" fill="{c}"/>"#,
            FileCategory::Pdf => {
                r#"<path d="M14 24h20M14 29h20M14 34h13" stroke="{c}" stroke-width="2.6" stroke-linecap="round"/><rect x="12" y="18" width="10" height="2.6" rx="1.3" fill="{c}"/>"#
            }
            FileCategory::Archive => {
                r#"<path d="M22 15h4v3h-4zM22 21h4v3h-4zM22 27h4v3h-4z" fill="{c}"/><rect x="20.5" y="32" width="7" height="8" rx="1.5" fill="{c}"/>"#
            }
            FileCategory::Code => {
                r#"<path d="M19 23l-6 6 6 6M29 23l6 6-6 6" stroke="{c}" stroke-width="2.8" fill="none" stroke-linecap="round" stroke-linejoin="round"/>"#
            }
            FileCategory::Spreadsheet => {
                r#"<rect x="13" y="21" width="22" height="17" rx="1.5" stroke="{c}" stroke-width="2.4" fill="none"/><path d="M13 27h22M13 32.5h22M21 21v17" stroke="{c}" stroke-width="2.2"/>"#
            }
            FileCategory::Presentation => {
                r#"<path d="M16 37V29M24 37V22M32 37V26" stroke="{c}" stroke-width="4" stroke-linecap="round"/>"#
            }
            FileCategory::Document => {
                r#"<path d="M14 22h20M14 27h20M14 32h20M14 37h12" stroke="{c}" stroke-width="2.4" stroke-linecap="round"/>"#
            }
            FileCategory::Text => {
                r#"<path d="M14 23h20M14 28h16M14 33h20" stroke="{c}" stroke-width="2.4" stroke-linecap="round"/>"#
            }
            FileCategory::Font => {
                r#"<path d="M16 38l8-18 8 18M19 32h10" stroke="{c}" stroke-width="3" fill="none" stroke-linecap="round" stroke-linejoin="round"/>"#
            }
            FileCategory::Program => {
                r#"<circle cx="24" cy="29" r="7.5" stroke="{c}" stroke-width="3" fill="none"/><circle cx="24" cy="29" r="2.6" fill="{c}"/>"#
            }
            FileCategory::DiskImage => {
                r#"<circle cx="24" cy="29" r="9" stroke="{c}" stroke-width="2.6" fill="none"/><circle cx="24" cy="29" r="2.4" fill="{c}"/>"#
            }
            FileCategory::Other => "",
        }
    }
}

const PAGE: &str = "M11 3h19l11 11v28a3 3 0 0 1-3 3H11a3 3 0 0 1-3-3V6a3 3 0 0 1 3-3z";
const FOLD: &str = "M30 3v8a3 3 0 0 0 3 3h8z";
const FOLDER_BACK: &str =
    "M4 11a3 3 0 0 1 3-3h11.5l4 4H41a3 3 0 0 1 3 3v24a3 3 0 0 1-3 3H7a3 3 0 0 1-3-3z";
const FOLDER_FRONT: &str = "M4 18a3 3 0 0 1 3-3h34a3 3 0 0 1 3 3v21a3 3 0 0 1-3 3H7a3 3 0 0 1-3-3z";

/// A special folder's color (front, back) and symbol (around 24,29, in `{c}`).
fn folder_style(dir: Option<UserDir>) -> (&'static str, &'static str, &'static str) {
    match dir {
        None => ("#FBBF24", "#D99A0B", ""),
        Some(UserDir::Home) => (
            "#6366F1",
            "#4446C9",
            r#"<path d="M24 22l-8 7h2.5v7h4v-4.5h3V36h4v-7H32z" fill="{c}"/>"#,
        ),
        Some(UserDir::Desktop) => (
            "#14B8A6",
            "#0D8C7F",
            r#"<rect x="15" y="23" width="18" height="11" rx="1.5" fill="{c}"/><rect x="21" y="35" width="6" height="2" fill="{c}"/>"#,
        ),
        Some(UserDir::Documents) => (
            "#3B82F6",
            "#2563C9",
            r#"<path d="M19 21h7l4 4v12H19z" fill="{c}"/>"#,
        ),
        Some(UserDir::Download) => (
            "#22C55E",
            "#16964A",
            r#"<path d="M24 21v11M19 28l5 5 5-5" stroke="{c}" stroke-width="3" fill="none" stroke-linecap="round" stroke-linejoin="round"/><path d="M17 36h14" stroke="{c}" stroke-width="3" stroke-linecap="round"/>"#,
        ),
        Some(UserDir::Music) => (
            "#FF8A3D",
            "#D9661B",
            r#"<path d="M22 22v9.5a3 3 0 1 0 2 2.8V25.5l6-1.7v5.7a3 3 0 1 0 2 2.8V20.5z" fill="{c}"/>"#,
        ),
        Some(UserDir::Pictures) => (
            "#A855F7",
            "#7E37C4",
            r#"<circle cx="20" cy="25" r="2.3" fill="{c}"/><path d="M15 36l6-7 4 4 3-3 5 6z" fill="{c}"/>"#,
        ),
        Some(UserDir::PublicShare) => (
            "#06B6D4",
            "#0589A1",
            r#"<circle cx="24" cy="29" r="7" stroke="{c}" stroke-width="2.4" fill="none"/><path d="M17 29h14M24 22c3 4 3 10 0 14M24 22c-3 4-3 10 0 14" stroke="{c}" stroke-width="1.8" fill="none"/>"#,
        ),
        Some(UserDir::Templates) => (
            "#94A3B8",
            "#6B7A8F",
            r#"<rect x="17" y="22" width="14" height="14" rx="1.5" stroke="{c}" stroke-width="2.4" fill="none" stroke-dasharray="3 2"/>"#,
        ),
        Some(UserDir::Videos) => (
            "#F43F5E",
            "#C21E3D",
            r#"<path d="M21 23v12l10-6z" fill="{c}"/>"#,
        ),
    }
}

fn svg(defs: &str, body: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><defs>{defs}</defs>{body}</svg>"#
    )
}

fn gradient(id: &str, top: &str, bottom: &str) -> String {
    format!(
        r#"<linearGradient id="{id}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{top}"/><stop offset="1" stop-color="{bottom}"/></linearGradient>"#
    )
}

fn file_svg(set: IconSet, category: FileCategory) -> String {
    let color = category.color();
    match set {
        IconSet::Vivid => svg(
            "",
            &format!(
                r##"<path d="{PAGE}" fill="{color}"/><path d="{FOLD}" fill="#fff" fill-opacity=".45"/>{}"##,
                category.symbol().replace("{c}", "#fff")
            ),
        ),
        IconSet::Classic => svg(
            &gradient("page", "#ffffff", "#f3f4f6"),
            &format!(
                r##"<path d="{PAGE}" fill="url(#page)" stroke="#c9ced6" stroke-width="1.2"/><path d="{FOLD}" fill="#e3e6eb" stroke="#c9ced6" stroke-width="1.2" stroke-linejoin="round"/>{}"##,
                category.symbol().replace("{c}", color)
            ),
        ),
        IconSet::Soft => svg(
            &format!(
                r#"{}<clipPath id="clip"><path d="{PAGE}"/></clipPath>"#,
                gradient("page", "#ffffff", "#eef0f4")
            ),
            &format!(
                r##"<path d="{PAGE}" fill="#000" fill-opacity=".12" transform="translate(0 1)"/><path d="{PAGE}" fill="url(#page)"/><path d="{FOLD}" fill="#d9dde4"/><g transform="translate(24 24) scale(.8) translate(-24 -27)">{}</g><rect x="8" y="38" width="33" height="7" fill="{color}" clip-path="url(#clip)"/>"##,
                category.symbol().replace("{c}", color)
            ),
        ),
    }
}

fn folder_svg(set: IconSet, dir: Option<UserDir>) -> String {
    let (front, back, symbol) = folder_style(dir);
    match set {
        IconSet::Vivid => svg(
            "",
            &format!(
                r##"<path d="{FOLDER_BACK}" fill="{back}"/><path d="{FOLDER_FRONT}" fill="{front}"/><path d="M8 19h32" stroke="#fff" stroke-opacity=".35" stroke-width="1.5" stroke-linecap="round"/>"##
            ),
        ),
        IconSet::Classic => {
            let defs =
                gradient("front", "#ffd767", "#f7b731") + &gradient("back", "#e9a51a", "#d48c0c");
            let mut body = format!(
                r#"<path d="{FOLDER_BACK}" fill="url(#back)"/><path d="{FOLDER_FRONT}" fill="url(#front)"/>"#
            );
            if !symbol.is_empty() {
                // A round badge in the folder's color with a white symbol,
                // large enough to read in list view.
                body += &format!(
                    r##"<circle cx="33" cy="33" r="12" fill="{front}" stroke="#fff" stroke-width="1.6"/><g transform="translate(33 33) scale(.68) translate(-24 -29)">{}</g>"##,
                    symbol.replace("{c}", "#fff")
                );
            }
            svg(&defs, &body)
        }
        IconSet::Soft => {
            let defs =
                gradient("front", "#8fcdfb", "#57aaf0") + &gradient("back", "#5eaaf0", "#3f8fde");
            let mut body = format!(
                r##"<path d="{FOLDER_BACK}" fill="url(#back)"/><path d="{FOLDER_FRONT}" fill="url(#front)"/><path d="M7 16h34" stroke="#fff" stroke-opacity=".5" stroke-width="1.2" stroke-linecap="round"/>"##
            );
            if !symbol.is_empty() {
                body += &format!(
                    r#"<g opacity=".55">{}</g>"#,
                    symbol.replace("{c}", "#2a7fd0")
                );
            }
            svg(&defs, &body)
        }
    }
}

/// What an icon shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Folder(IconSet, Option<UserDir>),
    File(IconSet, FileCategory),
}

/// Icon handles are built once and shared (they hold the SVG data).
fn cached(key: Key) -> icon::Handle {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<Key, icon::Handle>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut cache = cache.lock().unwrap_or_else(|poison| poison.into_inner());
    cache
        .entry(key)
        .or_insert_with(|| {
            let svg = match key {
                Key::Folder(set, dir) => folder_svg(set, dir),
                Key::File(set, category) => file_svg(set, category),
            };
            icon::from_svg_bytes(svg.into_bytes())
        })
        .clone()
}

pub fn folder(set: IconSet, dir: Option<UserDir>) -> icon::Handle {
    cached(Key::Folder(set, dir))
}

pub fn file(set: IconSet, mime: Option<&str>, name: &str) -> icon::Handle {
    cached(Key::File(set, FileCategory::of(mime, name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIRS: [Option<UserDir>; 10] = [
        None,
        Some(UserDir::Home),
        Some(UserDir::Desktop),
        Some(UserDir::Documents),
        Some(UserDir::Download),
        Some(UserDir::Music),
        Some(UserDir::Pictures),
        Some(UserDir::PublicShare),
        Some(UserDir::Templates),
        Some(UserDir::Videos),
    ];
    const CATEGORIES: [FileCategory; 14] = [
        FileCategory::Image,
        FileCategory::Audio,
        FileCategory::Video,
        FileCategory::Pdf,
        FileCategory::Archive,
        FileCategory::Code,
        FileCategory::Spreadsheet,
        FileCategory::Presentation,
        FileCategory::Document,
        FileCategory::Text,
        FileCategory::Font,
        FileCategory::Program,
        FileCategory::DiskImage,
        FileCategory::Other,
    ];

    #[test]
    fn sorts_files_into_categories() {
        assert_eq!(
            FileCategory::of(Some("image/png"), "a.png"),
            FileCategory::Image
        );
        assert_eq!(
            FileCategory::of(Some("application/pdf"), "a.pdf"),
            FileCategory::Pdf
        );
        assert_eq!(
            FileCategory::of(Some("text/plain"), "main.rs"),
            FileCategory::Code
        );
        assert_eq!(
            FileCategory::of(None, "backup.TAR.GZ"),
            FileCategory::Archive
        );
        assert_eq!(
            FileCategory::of(Some("text/plain"), "notes.txt"),
            FileCategory::Text
        );
        assert_eq!(FileCategory::of(None, "README"), FileCategory::Other);
    }

    #[test]
    fn writes_a_preview_of_every_icon() {
        // Writes 2fip-icon-sets.html to the temp folder, to look at the
        // icons in a browser. Each SVG is an <img> of its own, as in the app
        // (inline SVGs would share their gradient names).
        let mut html = String::from(
            "<html><body style='font:12px sans-serif'><style>img{width:48px;height:48px}\
             div{display:flex;flex-wrap:wrap;gap:8px;margin-bottom:16px}</style>",
        );
        for set in [IconSet::Vivid, IconSet::Classic, IconSet::Soft] {
            html += &format!("<h3>{set:?}</h3><div>");
            let svgs = DIRS
                .iter()
                .map(|dir| folder_svg(set, *dir))
                .chain(CATEGORIES.iter().map(|category| file_svg(set, *category)));
            for svg in svgs {
                assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
                assert!(!svg.contains("{c}"), "unfilled color in {svg}");
                let data: String = svg
                    .chars()
                    .map(|c| match c {
                        '#' => "%23".to_string(),
                        '"' => "'".to_string(),
                        c => c.to_string(),
                    })
                    .collect();
                html += &format!("<img src=\"data:image/svg+xml;utf8,{data}\">");
            }
            html += "</div>";
        }
        std::fs::write(std::env::temp_dir().join("2fip-icon-sets.html"), html).unwrap();
    }
}
