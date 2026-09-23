//! Metadata for the "Show Details" dialog: everything `list_dir` doesn't
//! already provide, including the recursive size of directories.

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::entry::EntryKind;

#[derive(Debug, Clone)]
pub struct EntryDetails {
    pub path: PathBuf,
    pub kind: EntryKind,
    pub mime_type: Option<String>,
    pub symlink_target: Option<PathBuf>,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
    /// Permission bits only (`st_mode & 0o7777`).
    pub mode: u32,
    pub owner: String,
    pub group: String,
}

#[derive(Debug, Clone)]
pub struct Details {
    /// Per-item metadata; only filled in when exactly one path was requested.
    pub single: Option<EntryDetails>,
    pub item_count: usize,
    /// Total size in bytes, recursing into directories.
    pub total_size: u64,
    /// Number of files and directories found *inside* the requested directories.
    pub contained_files: usize,
    pub contained_dirs: usize,
}

/// Gathers details for `paths`. Walks directories recursively to total their
/// size, so this can take a while on big trees.
pub async fn details(paths: Vec<PathBuf>) -> Result<Details, String> {
    tokio::task::spawn_blocking(move || gather(&paths))
        .await
        .map_err(|err| format!("details task failed: {err}"))?
}

fn gather(paths: &[PathBuf]) -> Result<Details, String> {
    let mut details = Details {
        single: None,
        item_count: paths.len(),
        total_size: 0,
        contained_files: 0,
        contained_dirs: 0,
    };

    for path in paths {
        let metadata = path
            .symlink_metadata()
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        if !metadata.is_dir() {
            details.total_size += metadata.len();
            continue;
        }
        // Unreadable subdirectories are skipped rather than failing the whole
        // dialog: a partial total is more useful than none.
        for entry in walkdir::WalkDir::new(path)
            .min_depth(1)
            .into_iter()
            .flatten()
        {
            let Ok(entry_metadata) = entry.metadata() else {
                continue;
            };
            if entry_metadata.is_dir() {
                details.contained_dirs += 1;
            } else {
                details.contained_files += 1;
                details.total_size += entry_metadata.len();
            }
        }
    }

    if let [path] = paths {
        details.single = Some(entry_details(path)?);
    }
    Ok(details)
}

fn entry_details(path: &Path) -> Result<EntryDetails, String> {
    let metadata = path
        .symlink_metadata()
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let kind = if metadata.is_symlink() {
        EntryKind::Symlink
    } else if metadata.is_dir() {
        EntryKind::Dir
    } else {
        EntryKind::File
    };
    Ok(EntryDetails {
        path: path.to_path_buf(),
        kind,
        mime_type: crate::mime::mime_type(path),
        symlink_target: std::fs::read_link(path).ok(),
        modified: metadata.modified().ok(),
        accessed: metadata.accessed().ok(),
        created: metadata.created().ok(),
        mode: metadata.mode() & 0o7777,
        owner: lookup_name("/etc/passwd", metadata.uid()),
        group: lookup_name("/etc/group", metadata.gid()),
    })
}

/// Resolves a uid/gid to its name via the local passwd/group file, falling
/// back to the number itself (e.g. for LDAP users not listed there).
fn lookup_name(db: &str, id: u32) -> String {
    std::fs::read_to_string(db)
        .ok()
        .and_then(|contents| {
            contents.lines().find_map(|line| {
                let mut fields = line.split(':');
                let name = fields.next()?;
                let entry_id = fields.nth(1)?.parse::<u32>().ok()?;
                (entry_id == id).then(|| name.to_string())
            })
        })
        .unwrap_or_else(|| id.to_string())
}

/// Formats permission bits as `rwxr-xr-x`.
pub fn format_mode(mode: u32) -> String {
    let mut out = String::with_capacity(9);
    for shift in [6, 3, 0] {
        let bits = (mode >> shift) & 0o7;
        out.push(if bits & 0o4 != 0 { 'r' } else { '-' });
        out.push(if bits & 0o2 != 0 { 'w' } else { '-' });
        out.push(if bits & 0o1 != 0 { 'x' } else { '-' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn totals_directory_contents_recursively() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project");
        std::fs::create_dir_all(project.join("sub")).unwrap();
        std::fs::write(project.join("a.txt"), b"12345").unwrap();
        std::fs::write(project.join("sub").join("b.txt"), b"123").unwrap();

        let details = details(vec![project]).await.unwrap();

        assert_eq!(details.total_size, 8);
        assert_eq!(details.contained_files, 2);
        assert_eq!(details.contained_dirs, 1);
        assert!(matches!(details.single.unwrap().kind, EntryKind::Dir));
    }

    #[tokio::test]
    async fn only_summarises_multiple_items() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, b"12").unwrap();
        std::fs::write(&b, b"345").unwrap();

        let details = details(vec![a, b]).await.unwrap();

        assert!(details.single.is_none());
        assert_eq!(details.item_count, 2);
        assert_eq!(details.total_size, 5);
    }

    #[test]
    fn formats_permission_bits() {
        assert_eq!(format_mode(0o755), "rwxr-xr-x");
        assert_eq!(format_mode(0o640), "rw-r-----");
    }
}
