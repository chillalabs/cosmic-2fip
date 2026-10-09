use std::path::PathBuf;

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

use super::engine::{copy_planned_files, StopReason};
use super::plan::plan_into;
use super::{involves_server, CancelHandle, ConflictHandle, OpEvent};

/// Copies `sources` into `dest_dir`, preserving directory structure for any
/// directories among them. Emits [`OpEvent::Progress`] after each file, and
/// [`OpEvent::Conflict`] whenever a destination already exists.
pub fn copy(
    sources: Vec<PathBuf>,
    dest_dir: PathBuf,
    cancel: CancelHandle,
    conflict: ConflictHandle,
) -> impl Stream<Item = OpEvent> {
    let (tx, rx) = mpsc::channel(16);
    if involves_server(&sources, &dest_dir) {
        tokio::spawn(crate::vfs::transfer::run_copy(
            sources, dest_dir, false, cancel, conflict, tx,
        ));
    } else {
        tokio::spawn(run(sources, dest_dir, cancel, conflict, tx));
    }
    ReceiverStream::new(rx)
}

async fn run(
    sources: Vec<PathBuf>,
    dest_dir: PathBuf,
    cancel: CancelHandle,
    conflict: ConflictHandle,
    tx: mpsc::Sender<OpEvent>,
) {
    let planned = match plan_into(&sources, &dest_dir).await {
        Ok(planned) => planned,
        Err(err) => {
            let _ = tx.send(OpEvent::Error(err)).await;
            return;
        }
    };

    let files_total = planned.len();
    let bytes_total: u64 = planned.iter().map(|file| file.size).sum();

    match copy_planned_files(&planned, files_total, bytes_total, &cancel, &conflict, &tx).await {
        Ok(()) => {
            let _ = tx.send(OpEvent::Done).await;
        }
        Err(StopReason::Cancelled) => {
            let _ = tx.send(OpEvent::Cancelled).await;
        }
        Err(StopReason::Error(err)) => {
            let _ = tx.send(OpEvent::Error(err)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::ConflictResolution;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn copies_a_single_file() {
        let src_dir = tempfile::tempdir().unwrap();
        let dest_dir = tempfile::tempdir().unwrap();
        let src_file = src_dir.path().join("a.txt");
        std::fs::write(&src_file, b"hello").unwrap();

        let mut stream = copy(
            vec![src_file],
            dest_dir.path().to_path_buf(),
            CancelHandle::new(),
            ConflictHandle::new(),
        );
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }

        assert!(matches!(events.last(), Some(OpEvent::Done)));
        assert_eq!(
            std::fs::read(dest_dir.path().join("a.txt")).unwrap(),
            b"hello"
        );
    }

    #[tokio::test]
    async fn copies_a_directory_preserving_structure() {
        let src_dir = tempfile::tempdir().unwrap();
        let dest_dir = tempfile::tempdir().unwrap();
        let project = src_dir.path().join("project");
        std::fs::create_dir_all(project.join("sub")).unwrap();
        std::fs::write(project.join("top.txt"), b"top").unwrap();
        std::fs::write(project.join("sub").join("nested.txt"), b"nested").unwrap();

        let mut stream = copy(
            vec![project.clone()],
            dest_dir.path().to_path_buf(),
            CancelHandle::new(),
            ConflictHandle::new(),
        );
        while stream.next().await.is_some() {}

        let dest_project = dest_dir.path().join("project");
        assert_eq!(std::fs::read(dest_project.join("top.txt")).unwrap(), b"top");
        assert_eq!(
            std::fs::read(dest_project.join("sub").join("nested.txt")).unwrap(),
            b"nested"
        );
    }

    #[tokio::test]
    async fn cancelling_before_the_first_file_yields_no_progress() {
        let src_dir = tempfile::tempdir().unwrap();
        let dest_dir = tempfile::tempdir().unwrap();
        let src_file = src_dir.path().join("a.txt");
        std::fs::write(&src_file, b"hello").unwrap();

        let cancel = CancelHandle::new();
        cancel.cancel();

        let mut stream = copy(
            vec![src_file],
            dest_dir.path().to_path_buf(),
            cancel,
            ConflictHandle::new(),
        );
        assert!(matches!(stream.next().await, Some(OpEvent::Cancelled)));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn existing_destination_asks_and_replaces() {
        let src_dir = tempfile::tempdir().unwrap();
        let dest_dir = tempfile::tempdir().unwrap();
        let src_file = src_dir.path().join("a.txt");
        std::fs::write(&src_file, b"new").unwrap();
        std::fs::write(dest_dir.path().join("a.txt"), b"old").unwrap();

        let conflict = ConflictHandle::new();
        let mut stream = copy(
            vec![src_file],
            dest_dir.path().to_path_buf(),
            CancelHandle::new(),
            conflict.clone(),
        );

        match stream.next().await {
            Some(OpEvent::Conflict { .. }) => {}
            other => panic!("expected a Conflict event, got {other:?}"),
        }
        conflict.respond(ConflictResolution::Replace);

        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }
        assert!(matches!(events.last(), Some(OpEvent::Done)));
        assert_eq!(
            std::fs::read(dest_dir.path().join("a.txt")).unwrap(),
            b"new"
        );
    }

    #[tokio::test]
    async fn existing_destination_can_be_skipped() {
        let src_dir = tempfile::tempdir().unwrap();
        let dest_dir = tempfile::tempdir().unwrap();
        let src_file = src_dir.path().join("a.txt");
        std::fs::write(&src_file, b"new").unwrap();
        std::fs::write(dest_dir.path().join("a.txt"), b"old").unwrap();

        let conflict = ConflictHandle::new();
        let mut stream = copy(
            vec![src_file],
            dest_dir.path().to_path_buf(),
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
        // Left untouched, since the user chose to skip it.
        assert_eq!(
            std::fs::read(dest_dir.path().join("a.txt")).unwrap(),
            b"old"
        );
    }

    #[tokio::test]
    async fn refuses_to_copy_a_file_onto_itself() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, b"precious").unwrap();

        let mut stream = copy(
            vec![file.clone()],
            dir.path().to_path_buf(),
            CancelHandle::new(),
            ConflictHandle::new(),
        );
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }

        assert!(
            matches!(events.last(), Some(OpEvent::Error(_))),
            "events: {events:?}"
        );
        assert_eq!(std::fs::read(&file).unwrap(), b"precious");
    }
}
