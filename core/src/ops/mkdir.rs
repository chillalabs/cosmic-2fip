use std::path::PathBuf;

/// Creates a new directory named `name` inside `parent`, returning its path.
pub async fn create_dir(parent: PathBuf, name: String) -> Result<PathBuf, String> {
    if let Some(location) = crate::vfs::RemoteLocation::parse(&parent) {
        let path = location.join(&name).to_path();
        crate::vfs::create_dir(&path).await?;
        return Ok(path);
    }
    let path = parent.join(&name);
    tokio::fs::create_dir(&path)
        .await
        .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn creates_a_directory() {
        let dir = tempfile::tempdir().unwrap();

        let created = create_dir(dir.path().to_path_buf(), "New folder".to_string())
            .await
            .unwrap();

        assert_eq!(created, dir.path().join("New folder"));
        assert!(created.is_dir());
    }

    #[tokio::test]
    async fn fails_when_the_name_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("existing")).unwrap();

        let result = create_dir(dir.path().to_path_buf(), "existing".to_string()).await;

        assert!(result.is_err());
    }
}
