use std::fs::File;
use std::io::{BufWriter, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::{CancelHandle, OpEvent, OpProgress};

/// Compresses `sources` (files and/or directories) into a new zip archive at
/// `archive`. Each source is stored under its own file name at the root of the
/// archive, preserving directory structure below it. Never overwrites: fails
/// if `archive` already exists. A cancelled or failed run removes the partial
/// archive.
pub fn compress_zip(
    sources: Vec<PathBuf>,
    archive: PathBuf,
    cancel: CancelHandle,
) -> impl Stream<Item = OpEvent> {
    let (tx, rx) = mpsc::channel(16);
    tokio::task::spawn_blocking(move || {
        let event = match write_archive(&sources, &archive, &cancel, &tx) {
            Ok(()) => OpEvent::Done,
            Err(stop) => stop,
        };
        let _ = tx.blocking_send(event);
    });
    ReceiverStream::new(rx)
}

struct ArchiveEntry {
    src: PathBuf,
    /// Path inside the archive, `/`-separated.
    name: String,
    kind: ArchiveEntryKind,
    size: u64,
    mode: u32,
}

enum ArchiveEntryKind {
    Dir,
    File,
    Symlink(PathBuf),
}

/// Walks every source up front so progress can report totals.
fn plan(sources: &[PathBuf]) -> Result<Vec<ArchiveEntry>, String> {
    let mut entries = Vec::new();
    for source in sources {
        let root_name = source
            .file_name()
            .ok_or_else(|| format!("{} has no file name", source.display()))?;
        let base = source.parent().unwrap_or(Path::new(""));
        for entry in walkdir::WalkDir::new(source) {
            let entry =
                entry.map_err(|err| format!("failed to walk {}: {err}", source.display()))?;
            let metadata = entry
                .path()
                .symlink_metadata()
                .map_err(|err| format!("failed to read {}: {err}", entry.path().display()))?;
            let rel = entry
                .path()
                .strip_prefix(base)
                .unwrap_or(Path::new(root_name));
            let name = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let kind = if metadata.is_symlink() {
                let target = std::fs::read_link(entry.path()).map_err(|err| {
                    format!("failed to read link {}: {err}", entry.path().display())
                })?;
                ArchiveEntryKind::Symlink(target)
            } else if metadata.is_dir() {
                ArchiveEntryKind::Dir
            } else {
                ArchiveEntryKind::File
            };
            entries.push(ArchiveEntry {
                src: entry.path().to_path_buf(),
                name,
                size: if metadata.is_file() {
                    metadata.len()
                } else {
                    0
                },
                mode: metadata.permissions().mode(),
                kind,
            });
        }
    }
    Ok(entries)
}

fn write_archive(
    sources: &[PathBuf],
    archive: &Path,
    cancel: &CancelHandle,
    tx: &mpsc::Sender<OpEvent>,
) -> Result<(), OpEvent> {
    let entries = plan(sources).map_err(OpEvent::Error)?;

    let file = File::create_new(archive)
        .map_err(|err| OpEvent::Error(format!("failed to create {}: {err}", archive.display())))?;
    // From here on the archive is ours, so a partial one is removed on failure.
    write_entries(file, &entries, archive, cancel, tx).inspect_err(|_| {
        let _ = std::fs::remove_file(archive);
    })
}

fn write_entries(
    file: File,
    entries: &[ArchiveEntry],
    archive: &Path,
    cancel: &CancelHandle,
    tx: &mpsc::Sender<OpEvent>,
) -> Result<(), OpEvent> {
    let files_total = entries.len();
    let bytes_total: u64 = entries.iter().map(|entry| entry.size).sum();
    let mut zip = ZipWriter::new(BufWriter::new(file));
    let io_err = |what: &Path, err: &dyn std::fmt::Display| {
        OpEvent::Error(format!("failed to compress {}: {err}", what.display()))
    };

    let mut bytes_done = 0u64;
    for (index, entry) in entries.iter().enumerate() {
        if cancel.is_cancelled() {
            return Err(OpEvent::Cancelled);
        }

        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(entry.mode & 0o7777)
            .large_file(entry.size >= u32::MAX as u64);

        match &entry.kind {
            ArchiveEntryKind::Dir => {
                zip.add_directory(entry.name.as_str(), options)
                    .map_err(|err| io_err(&entry.src, &err))?;
            }
            ArchiveEntryKind::Symlink(target) => {
                zip.add_symlink(entry.name.as_str(), target.to_string_lossy(), options)
                    .map_err(|err| io_err(&entry.src, &err))?;
            }
            ArchiveEntryKind::File => {
                zip.start_file(entry.name.as_str(), options)
                    .map_err(|err| io_err(&entry.src, &err))?;
                let mut src = File::open(&entry.src).map_err(|err| io_err(&entry.src, &err))?;
                std::io::copy(&mut src, &mut zip).map_err(|err| io_err(&entry.src, &err))?;
            }
        }

        bytes_done += entry.size;
        let _ = tx.blocking_send(OpEvent::Progress(OpProgress {
            current_file: entry.src.clone(),
            files_done: index + 1,
            files_total,
            bytes_done,
            bytes_total,
        }));
    }

    let mut writer = zip.finish().map_err(|err| io_err(archive, &err))?;
    writer.flush().map_err(|err| io_err(archive, &err))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tokio_stream::StreamExt;

    async fn run(sources: Vec<PathBuf>, archive: PathBuf) -> Vec<OpEvent> {
        let mut stream = compress_zip(sources, archive, CancelHandle::new());
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }
        events
    }

    #[tokio::test]
    async fn compresses_files_and_directories() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"alpha").unwrap();
        let project = dir.path().join("project");
        std::fs::create_dir_all(project.join("sub")).unwrap();
        std::fs::write(project.join("sub").join("b.txt"), b"beta").unwrap();
        let archive = dir.path().join("out.zip");

        let events = run(vec![dir.path().join("a.txt"), project], archive.clone()).await;
        assert!(
            matches!(events.last(), Some(OpEvent::Done)),
            "events: {events:?}"
        );

        let mut zip = zip::ZipArchive::new(File::open(&archive).unwrap()).unwrap();
        let mut contents = String::new();
        zip.by_name("project/sub/b.txt")
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        assert_eq!(contents, "beta");
        assert!(zip.by_name("a.txt").is_ok());
        assert!(zip.by_name("project/").is_ok());
    }

    #[tokio::test]
    async fn refuses_to_overwrite_an_existing_archive() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"alpha").unwrap();
        let archive = dir.path().join("out.zip");
        std::fs::write(&archive, b"keep me").unwrap();

        let events = run(vec![dir.path().join("a.txt")], archive.clone()).await;

        assert!(
            matches!(events.last(), Some(OpEvent::Error(_))),
            "events: {events:?}"
        );
        assert_eq!(std::fs::read(&archive).unwrap(), b"keep me");
    }
}
