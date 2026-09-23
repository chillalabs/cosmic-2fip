use std::path::Path;
use std::process::Command;

/// Returns the MIME type the desktop associates with `path` (e.g.
/// `text/plain`, `inode/directory`), as reported by `xdg-mime`, so it matches
/// what the desktop's own "Open With" and default-app logic use. Blocking.
pub fn mime_type(path: &Path) -> Option<String> {
    let output = Command::new("xdg-mime")
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
