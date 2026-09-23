use std::path::Path;

use tokio::fs;

use crate::entry::{DirEntry, EntryKind};
use crate::error::FsError;

/// Lists the immediate children of `path`. Does not recurse into subdirectories.
///
/// Symlinks are reported as `EntryKind::Symlink` without following them, so
/// a symlink pointing at a directory is not (yet) treated as navigable.
pub async fn list_dir(path: &Path) -> Result<Vec<DirEntry>, FsError> {
    let mut read_dir = fs::read_dir(path)
        .await
        .map_err(|source| FsError::ReadDir {
            path: path.to_path_buf(),
            source,
        })?;

    let mut entries = Vec::new();
    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|source| FsError::ReadDir {
            path: path.to_path_buf(),
            source,
        })?
    {
        let entry_path = entry.path();
        let metadata = entry
            .metadata()
            .await
            .map_err(|source| FsError::Metadata {
                path: entry_path.clone(),
                source,
            })?;

        let kind = if metadata.is_symlink() {
            EntryKind::Symlink
        } else if metadata.is_dir() {
            EntryKind::Dir
        } else {
            EntryKind::File
        };

        entries.push(DirEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry_path,
            kind,
            size: metadata.len(),
            modified: metadata.modified().ok(),
        });
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn lists_files_and_dirs() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();

        let mut entries = list_dir(dir.path()).await.unwrap();
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "a.txt");
        assert_eq!(entries[0].kind, EntryKind::File);
        assert_eq!(entries[0].size, 5);
        assert_eq!(entries[1].name, "sub");
        assert_eq!(entries[1].kind, EntryKind::Dir);
    }

    #[tokio::test]
    async fn missing_dir_errors() {
        let missing = std::path::PathBuf::from("/nonexistent/cosmic-commander-test-path");
        assert!(list_dir(&missing).await.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn detects_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, b"data").unwrap();
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let entries = list_dir(dir.path()).await.unwrap();
        let link_entry = entries.iter().find(|e| e.name == "link.txt").unwrap();
        assert_eq!(link_entry.kind, EntryKind::Symlink);
    }
}
