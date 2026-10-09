use std::path::{Path, PathBuf};

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

use super::{CancelHandle, OpEvent, OpProgress};

/// Sends each of `paths` to the trash. Never deletes permanently.
pub fn delete_to_trash(paths: Vec<PathBuf>, cancel: CancelHandle) -> impl Stream<Item = OpEvent> {
    let (tx, rx) = mpsc::channel(16);
    if paths.iter().any(|path| crate::vfs::is_remote(path)) {
        // Servers have no trash: the app asks with the "permanently" wording.
        tokio::spawn(crate::vfs::transfer::run_delete(paths, cancel, tx));
    } else {
        tokio::spawn(run(paths, false, cancel, tx));
    }
    ReceiverStream::new(rx)
}

/// Deletes each of `paths` for good, folders with everything in them
/// (Shift+Delete). There's no undo: the caller must have asked the user.
/// Symbolic links are removed themselves, never what they point to.
pub fn delete_permanently(
    paths: Vec<PathBuf>,
    cancel: CancelHandle,
) -> impl Stream<Item = OpEvent> {
    let (tx, rx) = mpsc::channel(16);
    if paths.iter().any(|path| crate::vfs::is_remote(path)) {
        tokio::spawn(crate::vfs::transfer::run_delete(paths, cancel, tx));
    } else {
        tokio::spawn(run(paths, true, cancel, tx));
    }
    ReceiverStream::new(rx)
}

fn remove(path: &Path) -> std::io::Result<()> {
    // `symlink_metadata`: a link to a folder is a link, not a folder.
    if std::fs::symlink_metadata(path)?.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

async fn run(
    paths: Vec<PathBuf>,
    permanently: bool,
    cancel: CancelHandle,
    tx: mpsc::Sender<OpEvent>,
) {
    let files_total = paths.len();

    for (index, path) in paths.into_iter().enumerate() {
        if cancel.is_cancelled() {
            let _ = tx.send(OpEvent::Cancelled).await;
            return;
        }

        let to_delete = path.clone();
        let result = tokio::task::spawn_blocking(move || {
            if permanently {
                remove(&to_delete).map_err(|err| err.to_string())
            } else {
                trash::delete(&to_delete).map_err(|err| err.to_string())
            }
        })
        .await;

        match result {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                let _ = tx
                    .send(OpEvent::Error(format!(
                        "failed to {} {}: {err}",
                        if permanently { "delete" } else { "trash" },
                        path.display()
                    )))
                    .await;
                return;
            }
            Err(err) => {
                let _ = tx
                    .send(OpEvent::Error(format!("delete task failed: {err}")))
                    .await;
                return;
            }
        }

        let _ = tx
            .send(OpEvent::Progress(OpProgress {
                current_file: path,
                files_done: index + 1,
                files_total,
                bytes_done: 0,
                bytes_total: 0,
            }))
            .await;
    }

    let _ = tx.send(OpEvent::Done).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn sends_a_file_to_trash_instead_of_deleting_it() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, b"bye").unwrap();

        let mut stream = delete_to_trash(vec![file.clone()], CancelHandle::new());
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }

        assert!(
            matches!(events.last(), Some(OpEvent::Done)),
            "events: {events:?}"
        );
        assert!(!file.exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn deletes_files_and_folders_permanently_but_not_link_targets() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        let folder = dir.path().join("folder");
        let kept = dir.path().join("kept");
        let link = dir.path().join("link");
        std::fs::write(&file, b"bye").unwrap();
        std::fs::create_dir_all(folder.join("inner")).unwrap();
        std::fs::write(folder.join("inner/b.txt"), b"bye").unwrap();
        std::fs::create_dir(&kept).unwrap();
        std::fs::write(kept.join("c.txt"), b"stay").unwrap();
        std::os::unix::fs::symlink(&kept, &link).unwrap();

        let events: Vec<OpEvent> = delete_permanently(
            vec![file.clone(), folder.clone(), link.clone()],
            CancelHandle::new(),
        )
        .collect()
        .await;

        assert!(
            matches!(events.last(), Some(OpEvent::Done)),
            "events: {events:?}"
        );
        assert!(!file.exists() && !folder.exists());
        assert!(
            std::fs::symlink_metadata(&link).is_err(),
            "the link is gone"
        );
        assert!(kept.join("c.txt").exists(), "its target is untouched");
    }
}
