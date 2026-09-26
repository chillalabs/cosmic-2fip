//! Drag and drop of files: the data dragged between pa2's panels and other
//! apps (COSMIC Files, the desktop, terminals), as the standard
//! `text/uri-list` (plus plain-text paths for apps that only take text).

use std::borrow::Cow;
use std::path::PathBuf;

use cosmic::iced::clipboard::mime::{AllowedMimeTypes, AsMimeTypes};

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
                .map(|path| format!("{}\r\n", fs_ops::thumbnail::file_uri(path)))
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
            .filter_map(file_uri_to_path)
            .collect();
        if paths.is_empty() {
            Err(())
        } else {
            Ok(FileList(paths))
        }
    }
}

/// `file:///a%20b` (or `file://localhost/...`) → `/a b`.
fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
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
    Some(PathBuf::from(std::ffi::OsString::from_vec(decoded)))
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
    fn offers_plain_text_paths_for_terminals() {
        let files = FileList(vec![PathBuf::from("/a b"), PathBuf::from("/c")]);
        let text = files.as_bytes(PLAIN_TEXT).unwrap();
        assert_eq!(&*text, b"/a b\n/c");
    }
}
