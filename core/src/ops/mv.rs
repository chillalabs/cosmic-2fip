use std::path::{Path, PathBuf};

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

use super::engine::{copy_planned_files, StopReason};
use super::plan::plan_tree;
use super::{CancelHandle, ConflictHandle, ConflictResolution, OpEvent, OpProgress};

/// Moves each of `sources` into `dest_dir`. Tries a same-filesystem rename
/// first (instant, atomic, whole tree at once); falls back to copying the
/// tree then removing the original when source and destination are on
/// different filesystems.
pub fn move_paths(
    sources: Vec<PathBuf>,
    dest_dir: PathBuf,
    cancel: CancelHandle,
    conflict: ConflictHandle,
) -> impl Stream<Item = OpEvent> {
    let (tx, rx) = mpsc::channel(16);
    tokio::spawn(run(sources, dest_dir, cancel, conflict, tx));
    ReceiverStream::new(rx)
}

async fn run(
    sources: Vec<PathBuf>,
    dest_dir: PathBuf,
    cancel: CancelHandle,
    conflict: ConflictHandle,
    tx: mpsc::Sender<OpEvent>,
) {
    let files_total = sources.len();
    let mut replace_all = false;
    let mut skip_all = false;

    for (index, source) in sources.into_iter().enumerate() {
        if cancel.is_cancelled() {
            let _ = tx.send(OpEvent::Cancelled).await;
            return;
        }

        let Some(name) = source.file_name().map(PathBuf::from) else {
            let _ = tx
                .send(OpEvent::Error(format!(
                    "{} has no file name",
                    source.display()
                )))
                .await;
            return;
        };
        let dest = dest_dir.join(&name);

        if !replace_all && tokio::fs::try_exists(&dest).await.unwrap_or(false) {
            if skip_all {
                continue;
            }
            let _ = tx
                .send(OpEvent::Conflict {
                    src: source.clone(),
                    dest: dest.clone(),
                })
                .await;
            match conflict.ask().await {
                ConflictResolution::Replace => {}
                ConflictResolution::ReplaceAll => replace_all = true,
                ConflictResolution::Skip => continue,
                ConflictResolution::SkipAll => {
                    skip_all = true;
                    continue;
                }
            }
        }

        let mut progress_already_sent = false;
        match tokio::fs::rename(&source, &dest).await {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::CrossesDevices => {
                match copy_then_remove(&source, &dest, &cancel, &conflict, &tx).await {
                    // The fallback already reported fine-grained per-file
                    // progress through `tx`; don't also send the coarse
                    // per-item progress below.
                    Ok(()) => progress_already_sent = true,
                    Err(FallbackOutcome::Cancelled) => {
                        let _ = tx.send(OpEvent::Cancelled).await;
                        return;
                    }
                    Err(FallbackOutcome::Failed(err)) => {
                        let _ = tx.send(OpEvent::Error(err)).await;
                        return;
                    }
                }
            }
            Err(err) => {
                let _ = tx
                    .send(OpEvent::Error(format!(
                        "failed to move {} to {}: {err}",
                        source.display(),
                        dest.display()
                    )))
                    .await;
                return;
            }
        }

        if !progress_already_sent {
            let _ = tx
                .send(OpEvent::Progress(OpProgress {
                    current_file: dest,
                    files_done: index + 1,
                    files_total,
                    bytes_done: 0,
                    bytes_total: 0,
                }))
                .await;
        }
    }

    let _ = tx.send(OpEvent::Done).await;
}

enum FallbackOutcome {
    Cancelled,
    Failed(String),
}

/// Cross-filesystem fallback for a single source: copy its whole tree onto
/// `dest`, then remove the original once every file has copied successfully.
async fn copy_then_remove(
    source: &Path,
    dest: &Path,
    cancel: &CancelHandle,
    conflict: &ConflictHandle,
    tx: &mpsc::Sender<OpEvent>,
) -> Result<(), FallbackOutcome> {
    let source_owned = source.to_path_buf();
    let dest_owned = dest.to_path_buf();
    let planned = tokio::task::spawn_blocking(move || plan_tree(&source_owned, &dest_owned))
        .await
        .map_err(|err| FallbackOutcome::Failed(format!("move planning task failed: {err}")))?
        .map_err(FallbackOutcome::Failed)?;

    let files_total = planned.len();
    let bytes_total: u64 = planned.iter().map(|file| file.size).sum();

    match copy_planned_files(&planned, files_total, bytes_total, cancel, conflict, tx).await {
        Ok(()) => {}
        Err(StopReason::Cancelled) => return Err(FallbackOutcome::Cancelled),
        Err(StopReason::Error(err)) => return Err(FallbackOutcome::Failed(err)),
    }

    remove_source(source).await.map_err(FallbackOutcome::Failed)
}

async fn remove_source(source: &Path) -> Result<(), String> {
    let metadata = tokio::fs::symlink_metadata(source)
        .await
        .map_err(|err| format!("failed to read {}: {err}", source.display()))?;
    if metadata.is_dir() {
        tokio::fs::remove_dir_all(source).await
    } else {
        tokio::fs::remove_file(source).await
    }
    .map_err(|err| format!("failed to remove {}: {err}", source.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn moves_a_single_file_on_the_same_filesystem() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        let dest = dir.path().join("dest");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dest).unwrap();
        let file = src.join("a.txt");
        std::fs::write(&file, b"hi").unwrap();

        let mut stream = move_paths(
            vec![file.clone()],
            dest.clone(),
            CancelHandle::new(),
            ConflictHandle::new(),
        );
        while stream.next().await.is_some() {}

        assert!(!file.exists());
        assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"hi");
    }

    #[tokio::test]
    async fn moves_a_directory_as_a_whole_tree() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        let dest = dir.path().join("dest");
        std::fs::create_dir_all(src.join("project").join("sub")).unwrap();
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(
            src.join("project").join("sub").join("nested.txt"),
            b"nested",
        )
        .unwrap();

        let project = src.join("project");
        let mut stream = move_paths(
            vec![project.clone()],
            dest.clone(),
            CancelHandle::new(),
            ConflictHandle::new(),
        );
        while stream.next().await.is_some() {}

        assert!(!project.exists());
        assert_eq!(
            std::fs::read(dest.join("project").join("sub").join("nested.txt")).unwrap(),
            b"nested"
        );
    }

    #[tokio::test]
    async fn existing_destination_file_asks_before_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        let dest = dir.path().join("dest");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(src.join("a.txt"), b"new").unwrap();
        std::fs::write(dest.join("a.txt"), b"old").unwrap();

        let conflict = ConflictHandle::new();
        let mut stream = move_paths(
            vec![src.join("a.txt")],
            dest.clone(),
            CancelHandle::new(),
            conflict.clone(),
        );

        assert!(matches!(
            stream.next().await,
            Some(OpEvent::Conflict { .. })
        ));
        conflict.respond(ConflictResolution::Skip);

        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }
        assert!(matches!(events.last(), Some(OpEvent::Done)));
        // Skipped: source untouched, destination untouched.
        assert_eq!(std::fs::read(src.join("a.txt")).unwrap(), b"new");
        assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"old");
    }
}
