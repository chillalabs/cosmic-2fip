use tokio::sync::mpsc;

use super::conflict::{ConflictHandle, ConflictResolution};
use super::plan::PlannedFile;
use super::{CancelHandle, OpEvent, OpProgress};

pub(super) enum StopReason {
    Cancelled,
    Error(String),
}

/// Copies each of `planned` to its destination, checking for cancellation and
/// destination conflicts between files. `files_total`/`bytes_total` describe
/// the whole batch (already known by the caller, since directory recursion
/// happened up front). Emits `OpEvent::Progress` after each file (including
/// skipped ones, so the bar still reaches completion) and `OpEvent::Conflict`
/// whenever a destination already exists, then awaits `conflict.ask()` for
/// how to proceed. Does *not* send `OpEvent::Done` — the caller does that
/// once it's done everything it needs to (e.g. also removing sources, for a
/// cross-filesystem move).
pub(super) async fn copy_planned_files(
    planned: &[PlannedFile],
    files_total: usize,
    bytes_total: u64,
    cancel: &CancelHandle,
    conflict: &ConflictHandle,
    tx: &mpsc::Sender<OpEvent>,
) -> Result<(), StopReason> {
    let mut bytes_done = 0u64;
    let mut files_done = 0usize;
    let mut replace_all = false;
    let mut skip_all = false;

    for file in planned {
        if cancel.is_cancelled() {
            return Err(StopReason::Cancelled);
        }

        if is_same_file(&file.src, &file.dest).await {
            // Copying a file onto itself would truncate it before reading it.
            return Err(StopReason::Error(format!(
                "cannot copy {} onto itself",
                file.src.display()
            )));
        }

        if !replace_all && tokio::fs::try_exists(&file.dest).await.unwrap_or(false) {
            if skip_all {
                files_done += 1;
                continue;
            }

            let _ = tx
                .send(OpEvent::Conflict {
                    src: file.src.clone(),
                    dest: file.dest.clone(),
                })
                .await;

            match conflict.ask().await {
                ConflictResolution::Replace => {}
                ConflictResolution::ReplaceAll => replace_all = true,
                ConflictResolution::Skip => {
                    files_done += 1;
                    continue;
                }
                ConflictResolution::SkipAll => {
                    skip_all = true;
                    files_done += 1;
                    continue;
                }
            }
        }

        if let Some(parent) = file.dest.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|err| {
                StopReason::Error(format!("failed to create {}: {err}", parent.display()))
            })?;
        }
        tokio::fs::copy(&file.src, &file.dest)
            .await
            .map_err(|err| {
                StopReason::Error(format!(
                    "failed to copy {} to {}: {err}",
                    file.src.display(),
                    file.dest.display()
                ))
            })?;

        bytes_done += file.size;
        files_done += 1;
        let _ = tx
            .send(OpEvent::Progress(OpProgress {
                current_file: file.dest.clone(),
                files_done,
                files_total,
                bytes_done,
                bytes_total,
            }))
            .await;
    }

    Ok(())
}

async fn is_same_file(a: &std::path::Path, b: &std::path::Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (tokio::fs::metadata(a).await, tokio::fs::metadata(b).await) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}
