//! Drag and drop of files: the data dragged between 2fip's panels and other
//! apps (COSMIC Files, the desktop, terminals), as the standard
//! `text/uri-list` (plus plain-text paths for apps that only take text).

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use cosmic::iced::clipboard::mime::{AllowedMimeTypes, AsMimeTypes};
use fs_ops::settings::ViewMode;

use crate::app::PaneId;

const URI_LIST: &str = "text/uri-list";
const PLAIN_TEXT: &str = "text/plain;charset=utf-8";

/// The files being dragged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileList(pub Vec<PathBuf>);

impl AsMimeTypes for FileList {
    fn available(&self) -> Cow<'static, [String]> {
        Cow::Owned(vec![URI_LIST.to_string(), PLAIN_TEXT.to_string()])
    }

    fn as_bytes(&self, mime_type: &str) -> Option<Cow<'static, [u8]>> {
        let text = match mime_type {
            // One URI per line, CRLF-terminated (RFC 2483).
            URI_LIST => self
                .0
                .iter()
                .map(|path| format!("{}\r\n", uri(path)))
                .collect::<String>(),
            PLAIN_TEXT => self
                .0
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("\n"),
            _ => return None,
        };
        Some(Cow::Owned(text.into_bytes()))
    }
}

impl AllowedMimeTypes for FileList {
    fn allowed() -> Cow<'static, [String]> {
        Cow::Owned(vec![URI_LIST.to_string()])
    }
}

impl TryFrom<(Vec<u8>, String)> for FileList {
    type Error = ();

    /// Reads a `text/uri-list`: local `file://` URIs only; comments (`#`)
    /// and other schemes are skipped.
    fn try_from((data, _mime): (Vec<u8>, String)) -> Result<Self, ()> {
        let text = String::from_utf8(data).map_err(|_| ())?;
        let paths: Vec<PathBuf> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .filter_map(|line| file_uri_to_path(line).or_else(|| server_uri_to_path(line)))
            .collect();
        if paths.is_empty() {
            Err(())
        } else {
            Ok(FileList(paths))
        }
    }
}

/// A tab dragged to the other panel, which opens a copy of it. Its own
/// MIME type, so only 2fip's panels accept it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabDrag {
    pub source: PaneId,
    pub dir: PathBuf,
    pub view_mode: ViewMode,
}

const TAB_MIME: &str = "application/x-2fip-tab";

impl AsMimeTypes for TabDrag {
    fn available(&self) -> Cow<'static, [String]> {
        Cow::Owned(vec![TAB_MIME.to_string()])
    }

    /// `left|list|/the/folder` (the folder last, so it may hold `|`).
    fn as_bytes(&self, mime_type: &str) -> Option<Cow<'static, [u8]>> {
        (mime_type == TAB_MIME).then(|| {
            let source = match self.source {
                PaneId::Left => "left",
                PaneId::Right => "right",
            };
            let view = match self.view_mode {
                ViewMode::List => "list",
                ViewMode::Grid => "grid",
            };
            let text = format!("{source}|{view}|{}", self.dir.display());
            Cow::Owned(text.into_bytes())
        })
    }
}

impl AllowedMimeTypes for TabDrag {
    fn allowed() -> Cow<'static, [String]> {
        Cow::Owned(vec![TAB_MIME.to_string()])
    }
}

impl TryFrom<(Vec<u8>, String)> for TabDrag {
    type Error = ();

    fn try_from((data, _mime): (Vec<u8>, String)) -> Result<Self, ()> {
        let text = String::from_utf8(data).map_err(|_| ())?;
        let mut parts = text.splitn(3, '|');
        let source = match parts.next() {
            Some("left") => PaneId::Left,
            Some("right") => PaneId::Right,
            _ => return Err(()),
        };
        let view_mode = match parts.next() {
            Some("list") => ViewMode::List,
            Some("grid") => ViewMode::Grid,
            _ => return Err(()),
        };
        let dir = PathBuf::from(parts.next().filter(|dir| !dir.is_empty()).ok_or(())?);
        Ok(TabDrag {
            source,
            dir,
            view_mode,
        })
    }
}

/// `file:///a%20b` for local files; a file on a server keeps its own
/// `sftp://` / `ftp://` address (apps that speak those protocols can open it).
fn uri(path: &Path) -> String {
    if fs_ops::vfs::is_remote(path) {
        path.display().to_string()
    } else {
        fs_ops::thumbnail::file_uri(path)
    }
}

/// `sftp://user@host:22/a` and friends, e.g. dragged from the other panel.
fn server_uri_to_path(uri: &str) -> Option<PathBuf> {
    fs_ops::vfs::RemoteLocation::parse(Path::new(uri)).map(|location| location.to_path())
}

/// `file:///a%20b` (or `file://localhost/...`) → `/a b`.
fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let path = rest.strip_prefix("localhost").unwrap_or(rest);
    if !path.starts_with('/') {
        return None; // another host
    }
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        // "%xx" needs two more bytes after the '%'.
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    bytes_to_path(decoded)
}

#[cfg(unix)]
fn bytes_to_path(bytes: Vec<u8>) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
}

/// `/C:/Users/...` → `C:/Users/...` (Windows paths are UTF-16, so the URI's
/// bytes must be UTF-8).
#[cfg(not(unix))]
fn bytes_to_path(bytes: Vec<u8>) -> Option<PathBuf> {
    let path = String::from_utf8(bytes).ok()?;
    let has_drive = path.as_bytes().get(2) == Some(&b':');
    let path = if has_drive { &path[1..] } else { &path[..] };
    Some(PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_a_uri_list() {
        let files = FileList(vec![
            PathBuf::from("/home/me/a b.txt"),
            PathBuf::from("/tmp/ñandú (1).pdf"),
        ]);
        let bytes = files.as_bytes(URI_LIST).unwrap().into_owned();
        let parsed = FileList::try_from((bytes, URI_LIST.to_string())).unwrap();
        assert_eq!(parsed, files);
    }

    #[test]
    fn reads_uri_lists_from_other_apps() {
        let data = "# a comment\r\nfile:///srv/x%23y\r\nfile://localhost/etc/hosts\r\nhttps://example.com/z\r\n";
        let parsed = FileList::try_from((data.as_bytes().to_vec(), URI_LIST.to_string())).unwrap();
        assert_eq!(
            parsed.0,
            [PathBuf::from("/srv/x#y"), PathBuf::from("/etc/hosts")]
        );
        assert!(FileList::try_from((b"https://x".to_vec(), URI_LIST.to_string())).is_err());
    }

    #[test]
    fn server_files_keep_their_server_address() {
        let files = FileList(vec![PathBuf::from("sftp://me@host:22/home/me/a.txt")]);
        let bytes = files.as_bytes(URI_LIST).unwrap().into_owned();
        assert_eq!(bytes, b"sftp://me@host:22/home/me/a.txt\r\n");
        let parsed = FileList::try_from((bytes, URI_LIST.to_string())).unwrap();
        assert_eq!(parsed, files);
    }

    #[test]
    fn tabs_travel_with_their_folder_and_view() {
        let tab = TabDrag {
            source: PaneId::Right,
            dir: PathBuf::from("sftp://me@host:22/a|b"),
            view_mode: ViewMode::Grid,
        };
        let bytes = tab.as_bytes(TAB_MIME).unwrap().into_owned();
        assert_eq!(TabDrag::try_from((bytes, TAB_MIME.to_string())), Ok(tab));
        assert!(TabDrag::try_from((b"nonsense".to_vec(), TAB_MIME.to_string())).is_err());
    }

    #[test]
    fn offers_plain_text_paths_for_terminals() {
        let files = FileList(vec![PathBuf::from("/a b"), PathBuf::from("/c")]);
        let text = files.as_bytes(PLAIN_TEXT).unwrap();
        assert_eq!(&*text, b"/a b\n/c");
    }
}
