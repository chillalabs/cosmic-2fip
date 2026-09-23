use std::path::PathBuf;

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

use super::{CancelHandle, OpEvent, OpProgress};

/// Sends each of `paths` to the trash. Never deletes permanently.
pub fn delete_to_trash(paths: Vec<PathBuf>, cancel: CancelHandle) -> impl Stream<Item = OpEvent> {
    let (tx, rx) = mpsc::channel(16);
    tokio::spawn(run(paths, cancel, tx));
    ReceiverStream::new(rx)
}

async fn run(paths: Vec<PathBuf>, cancel: CancelHandle, tx: mpsc::Sender<OpEvent>) {
    let files_total = paths.len();

    for (index, path) in paths.into_iter().enumerate() {
        if cancel.is_cancelled() {
            let _ = tx.send(OpEvent::Cancelled).await;
            return;
        }

        let to_delete = path.clone();
        let result = tokio::task::spawn_blocking(move || trash::delete(&to_delete)).await;

        match result {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                let _ = tx
                    .send(OpEvent::Error(format!("failed to trash {}: {err}", path.display())))
                    .await;
                return;
            }
            Err(err) => {
                let _ = tx.send(OpEvent::Error(format!("delete task failed: {err}"))).await;
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

        assert!(matches!(events.last(), Some(OpEvent::Done)), "events: {events:?}");
        assert!(!file.exists());
    }
}
