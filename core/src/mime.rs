use std::path::Path;

/// Returns the MIME type the desktop associates with `path` (e.g.
/// `text/plain`, `inode/directory`), as reported by `xdg-mime`, so it matches
/// what the desktop's own "Open With" and default-app logic use. Without
/// `xdg-mime` (e.g. in a Flatpak) it's guessed from the name. Blocking.
pub fn mime_type(path: &Path) -> Option<String> {
    query_xdg_mime(path).or_else(|| guess(path))
}

/// The type from the file name's extension (folders: `inode/directory`).
fn guess(path: &Path) -> Option<String> {
    if path.is_dir() {
        return Some("inode/directory".to_string());
    }
    crate::thumbnail::guess_mime(path)
}

fn query_xdg_mime(path: &Path) -> Option<String> {
    let output = crate::sandbox::host_command("xdg-mime")
        .args(["query", "filetype"])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mime = String::from_utf8(output.stdout).ok()?;
    // Some backends append parameters, e.g. `text/plain; charset=us-ascii`.
    let mime = mime.split(';').next()?.trim();
    (!mime.is_empty()).then(|| mime.to_string())
}
