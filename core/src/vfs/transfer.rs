//! Copy, move and delete when a server is involved: local → remote (upload),
//! remote → local (download), and remote → remote (through a temporary local
//! file). Reports the same [`OpEvent`]s as local operations, with byte-level
//! progress, so the progress panel, Cancel and "file already exists" work
//! unchanged.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use super::RemoteLocation;
use crate::entry::EntryKind;
use crate::ops::{CancelHandle, ConflictHandle, ConflictResolution, OpEvent, OpProgress};

/// One thing to create at the destination.
struct Item {
    src: PathBuf,
    dest: PathBuf,
    is_dir: bool,
    size: u64,
}

/// Copies (or, with `remove_sources`, moves) `sources` into `dest_dir` when
/// either side is on a server.
pub async fn run_copy(
    sources: Vec<PathBuf>,
    dest_dir: PathBuf,
    remove_sources: bool,
    cancel: CancelHandle,
    conflict: ConflictHandle,
    tx: mpsc::Sender<OpEvent>,
) {
    let event = match copy_all(&sources, &dest_dir, remove_sources, &cancel, &conflict, &tx).await {
        Ok(()) => OpEvent::Done,
        Err(Stop::Cancelled) => OpEvent::Cancelled,
        Err(Stop::Error(message)) => OpEvent::Error(message),
    };
    let _ = tx.send(event).await;
}

/// Deletes `paths`; on a server that's always permanent (servers have no
/// trash).
pub async fn run_delete(paths: Vec<PathBuf>, cancel: CancelHandle, tx: mpsc::Sender<OpEvent>) {
    let files_total = paths.len();
    for (index, path) in paths.into_iter().enumerate() {
        if cancel.is_cancelled() {
            let _ = tx.send(OpEvent::Cancelled).await;
            return;
        }
        if let Err(err) = remove(&path).await {
            let _ = tx
                .send(OpEvent::Error(format!(
                    "failed to delete {}: {err}",
                    path.display()
                )))
                .await;
            return;
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

enum Stop {
    Cancelled,
    Error(String),
}

impl From<String> for Stop {
    fn from(message: String) -> Self {
        Stop::Error(message)
    }
}

async fn copy_all(
    sources: &[PathBuf],
    dest_dir: &Path,
    remove_sources: bool,
    cancel: &CancelHandle,
    conflict: &ConflictHandle,
    tx: &mpsc::Sender<OpEvent>,
) -> Result<(), Stop> {
    // A move within one server is a rename: instant, whole folders at once.
    if remove_sources && same_server(sources, dest_dir) {
        return rename_all(sources, dest_dir, cancel, conflict, tx).await;
    }

    let mut items = Vec::new();
    for source in sources {
        let name = file_name(source)?;
        plan(source, &join(dest_dir, &name), &mut items).await?;
    }
    let files_total = items.iter().filter(|item| !item.is_dir).count();
    let bytes_total: u64 = items.iter().map(|item| item.size).sum();

    let mut files_done = 0;
    let mut bytes_done = 0u64;
    let mut replace_all = false;
    let mut skip_all = false;
    for item in &items {
        if cancel.is_cancelled() {
            return Err(Stop::Cancelled);
        }
        if item.is_dir {
            make_dirs(&item.dest).await?;
            continue;
        }
        if !replace_all && exists(&item.dest).await? {
            let skip = if skip_all {
                true
            } else {
                let _ = tx
                    .send(OpEvent::Conflict {
                        src: item.src.clone(),
                        dest: item.dest.clone(),
                    })
                    .await;
                match conflict.ask().await {
                    ConflictResolution::Replace => false,
                    ConflictResolution::ReplaceAll => {
                        replace_all = true;
                        false
                    }
                    ConflictResolution::Skip => true,
                    ConflictResolution::SkipAll => {
                        skip_all = true;
                        true
                    }
                }
            };
            if skip {
                files_done += 1;
                bytes_done += item.size;
                continue;
            }
        }
        if let Some(parent) = super::parent(&item.dest) {
            make_dirs(&parent).await?;
        }

        // Progress while the bytes flow, at most ~10 times a second.
        let start_bytes = bytes_done;
        let mut last_sent = Instant::now() - Duration::from_secs(1);
        let progress_tx = tx.clone();
        let current = item.dest.clone();
        let mut on_progress = move |copied: u64| {
            if last_sent.elapsed() >= Duration::from_millis(100) {
                last_sent = Instant::now();
                let _ = progress_tx.try_send(OpEvent::Progress(OpProgress {
                    current_file: current.clone(),
                    files_done,
                    files_total,
                    bytes_done: start_bytes + copied,
                    bytes_total,
                }));
            }
        };
        let cancelled = || cancel.is_cancelled();
        let result = copy_file(&item.src, &item.dest, &mut on_progress, &cancelled).await;
        if cancel.is_cancelled() {
            return Err(Stop::Cancelled);
        }
        result?;

        files_done += 1;
        bytes_done += item.size;
        let _ = tx
            .send(OpEvent::Progress(OpProgress {
                current_file: item.dest.clone(),
                files_done,
                files_total,
                bytes_done,
                bytes_total,
            }))
            .await;
    }

    if remove_sources {
        for source in sources {
            if cancel.is_cancelled() {
                return Err(Stop::Cancelled);
            }
            remove(source).await?;
        }
    }
    Ok(())
}

/// A move within one server: each source renamed into `dest_dir`.
async fn rename_all(
    sources: &[PathBuf],
    dest_dir: &Path,
    cancel: &CancelHandle,
    conflict: &ConflictHandle,
    tx: &mpsc::Sender<OpEvent>,
) -> Result<(), Stop> {
    let files_total = sources.len();
    let mut replace_all = false;
    let mut skip_all = false;
    for (index, source) in sources.iter().enumerate() {
        if cancel.is_cancelled() {
            return Err(Stop::Cancelled);
        }
        let dest = join(dest_dir, &file_name(source)?);
        if !replace_all && exists(&dest).await? {
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
            remove(&dest).await?;
        }
        super::rename(source, &dest).await?;
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
    Ok(())
}

fn same_server(sources: &[PathBuf], dest_dir: &Path) -> bool {
    let Some(dest) = RemoteLocation::parse(dest_dir) else {
        return false;
    };
    sources.iter().all(|source| {
        RemoteLocation::parse(source).is_some_and(|source| source.root() == dest.root())
    })
}

/// Lists everything under `src` (a file or folder, local or remote) as the
/// items to create under `dest`, folders before their contents.
async fn plan(src: &Path, dest: &Path, items: &mut Vec<Item>) -> Result<(), String> {
    if RemoteLocation::parse(src).is_some() {
        let Some(stat) = super::stat(src).await? else {
            return Err(format!("{} doesn't exist", src.display()));
        };
        if stat.kind != EntryKind::Dir {
            items.push(Item {
                src: src.to_path_buf(),
                dest: dest.to_path_buf(),
                is_dir: false,
                size: stat.size,
            });
            return Ok(());
        }
        items.push(Item {
            src: src.to_path_buf(),
            dest: dest.to_path_buf(),
            is_dir: true,
            size: 0,
        });
        for entry in super::list(src).await? {
            Box::pin(plan(&entry.path, &join(dest, &entry.name), items)).await?;
        }
        return Ok(());
    }

    let src_owned = src.to_path_buf();
    let dest_owned = dest.to_path_buf();
    let local = tokio::task::spawn_blocking(move || -> Result<Vec<Item>, String> {
        let mut items = Vec::new();
        for entry in walkdir::WalkDir::new(&src_owned) {
            let entry = entry.map_err(|err| format!("{}: {err}", src_owned.display()))?;
            let relative = entry
                .path()
                .strip_prefix(&src_owned)
                .unwrap_or(entry.path());
            let mut dest = dest_owned.clone();
            for part in relative.components() {
                dest = join(&dest, &part.as_os_str().to_string_lossy());
            }
            let file_type = entry.file_type();
            if file_type.is_dir() {
                items.push(Item {
                    src: entry.path().to_path_buf(),
                    dest,
                    is_dir: true,
                    size: 0,
                });
            } else if file_type.is_file() {
                let size = entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
                items.push(Item {
                    src: entry.path().to_path_buf(),
                    dest,
                    is_dir: false,
                    size,
                });
            }
            // Symbolic links and special files aren't copied to servers.
        }
        Ok(items)
    })
    .await
    .map_err(|err| format!("planning failed: {err}"))??;
    items.extend(local);
    Ok(())
}

async fn copy_file(
    src: &Path,
    dest: &Path,
    progress: &mut (dyn FnMut(u64) + Send),
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<(), String> {
    match (super::is_remote(src), super::is_remote(dest)) {
        (false, true) => super::upload(src, dest, progress, cancelled).await,
        (true, false) => super::download(src, dest, progress, cancelled).await,
        (true, true) => {
            // Server to server: through a temporary local file.
            let temp = tempfile_path();
            let result = async {
                super::download(src, &temp, progress, cancelled).await?;
                super::upload(&temp, dest, &mut |_| {}, cancelled).await
            }
            .await;
            let _ = tokio::fs::remove_file(&temp).await;
            result
        }
        (false, false) => tokio::fs::copy(src, dest)
            .await
            .map(|_| ())
            .map_err(|err| format!("{} → {}: {err}", src.display(), dest.display())),
    }
}

fn tempfile_path() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        ".2fip-transfer-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

async fn exists(path: &Path) -> Result<bool, String> {
    if super::is_remote(path) {
        Ok(super::stat(path).await?.is_some())
    } else {
        Ok(tokio::fs::try_exists(path).await.unwrap_or(false))
    }
}

/// Creates `path` and any missing parents.
async fn make_dirs(path: &Path) -> Result<(), String> {
    if !super::is_remote(path) {
        return tokio::fs::create_dir_all(path)
            .await
            .map_err(|err| format!("failed to create {}: {err}", path.display()));
    }
    match super::stat(path).await? {
        Some(stat) if stat.kind == EntryKind::Dir => Ok(()),
        Some(_) => Err(format!("{} exists and isn't a folder", path.display())),
        None => {
            if let Some(parent) = super::parent(path) {
                Box::pin(make_dirs(&parent)).await?;
            }
            super::create_dir(path).await
        }
    }
}

async fn remove(path: &Path) -> Result<(), String> {
    if super::is_remote(path) {
        return super::remove_all(path).await;
    }
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let metadata = std::fs::symlink_metadata(&path).map_err(|err| err.to_string())?;
        if metadata.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        }
        .map_err(|err| format!("{}: {err}", path.display()))
    })
    .await
    .map_err(|err| err.to_string())?
}

/// `dest_dir/name`, for local or remote folders.
fn join(dest_dir: &Path, name: &str) -> PathBuf {
    match RemoteLocation::parse(dest_dir) {
        Some(location) => location.join(name).to_path(),
        None => dest_dir.join(name),
    }
}

fn file_name(path: &Path) -> Result<String, String> {
    match RemoteLocation::parse(path) {
        Some(location) if location.path != "/" => Ok(location.name().to_string()),
        Some(_) => Err(format!("{} is a server's root", path.display())),
        None => path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .ok_or_else(|| format!("{} has no file name", path.display())),
    }
}
