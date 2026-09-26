use std::path::PathBuf;

/// Renames `path` to `new_name` within its parent directory, returning the new path.
pub async fn rename(path: PathBuf, new_name: String) -> Result<PathBuf, String> {
    let Some(parent) = path.parent() else {
        return Err(format!("{} has no parent directory", path.display()));
    };
    let dest = parent.join(&new_name);
    tokio::fs::rename(&path, &dest).await.map_err(|err| {
        format!(
            "failed to rename {} to {}: {err}",
            path.display(),
            dest.display()
        )
    })?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn renames_a_file_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, b"hi").unwrap();

        let new_path = rename(file.clone(), "b.txt".to_string()).await.unwrap();

        assert!(!file.exists());
        assert_eq!(new_path, dir.path().join("b.txt"));
        assert_eq!(std::fs::read(&new_path).unwrap(), b"hi");
    }
}
