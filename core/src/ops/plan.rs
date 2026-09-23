use std::path::{Path, PathBuf};

pub(super) struct PlannedFile {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub size: u64,
}

/// Recursively plans copying `src` (a file or directory) onto `dest_root`,
/// preserving `src`'s internal directory structure. Blocking; run inside
/// `spawn_blocking`.
pub(super) fn plan_tree(src: &Path, dest_root: &Path) -> Result<Vec<PlannedFile>, String> {
    let metadata = std::fs::symlink_metadata(src)
        .map_err(|err| format!("failed to read {}: {err}", src.display()))?;

    if !metadata.is_dir() {
        return Ok(vec![PlannedFile {
            src: src.to_path_buf(),
            dest: dest_root.to_path_buf(),
            size: metadata.len(),
        }]);
    }

    let mut planned = Vec::new();
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry.map_err(|err| format!("failed to walk {}: {err}", src.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(src).unwrap_or(entry.path());
        let size = entry
            .metadata()
            .map_err(|err| format!("failed to read {}: {err}", entry.path().display()))?
            .len();
        planned.push(PlannedFile {
            src: entry.path().to_path_buf(),
            dest: dest_root.join(rel),
            size,
        });
    }
    Ok(planned)
}

/// Plans copying each of `sources` into `dest_dir`, keyed by each source's own
/// file name (so `copy([a, dir], dest)` lands at `dest/a` and `dest/dir/...`).
pub(super) async fn plan_into(
    sources: &[PathBuf],
    dest_dir: &Path,
) -> Result<Vec<PlannedFile>, String> {
    let sources = sources.to_vec();
    let dest_dir = dest_dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut planned = Vec::new();
        for source in &sources {
            let name = source
                .file_name()
                .ok_or_else(|| format!("{} has no file name", source.display()))?;
            planned.extend(plan_tree(source, &dest_dir.join(name))?);
        }
        Ok(planned)
    })
    .await
    .map_err(|err| format!("copy planning task failed: {err}"))?
}
