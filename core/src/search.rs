//! Find Files: walks a folder tree in the background and streams the paths
//! whose names match, in batches, so the UI can show them as they come.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

use crate::ops::CancelHandle;
use crate::settings::is_hidden;

/// A file or folder whose name matched.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SearchHit {
    pub path: PathBuf,
    pub is_dir: bool,
}

/// What a running search reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchEvent {
    /// More matches, in the order they were found.
    Found(Vec<SearchHit>),
    /// The search ended: finished, cancelled, or stopped at the result limit
    /// (`truncated`).
    Done { truncated: bool },
}

/// Matches sent together at most this often, so a big search doesn't flood
/// the UI with one message per file.
const BATCH_INTERVAL: Duration = Duration::from_millis(100);
const BATCH_SIZE: usize = 200;

/// Searches `root` and all its subfolders (not following symbolic links) for
/// files and folders whose name satisfies `matches`. Hidden files and
/// folders are skipped unless `include_hidden`. Stops after `limit` matches
/// or when `cancel` fires.
pub fn search(
    root: PathBuf,
    matches: impl Fn(&str) -> bool + Send + 'static,
    include_hidden: bool,
    limit: usize,
    cancel: CancelHandle,
) -> impl Stream<Item = SearchEvent> {
    let (tx, rx) = mpsc::channel(8);
    tokio::task::spawn_blocking(move || {
        let truncated = walk(&root, &matches, include_hidden, limit, &cancel, &tx);
        let _ = tx.blocking_send(SearchEvent::Done { truncated });
    });
    ReceiverStream::new(rx)
}

/// Returns whether the search stopped at `limit`.
fn walk(
    root: &Path,
    matches: &impl Fn(&str) -> bool,
    include_hidden: bool,
    limit: usize,
    cancel: &CancelHandle,
    tx: &mpsc::Sender<SearchEvent>,
) -> bool {
    let entries = walkdir::WalkDir::new(root)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| include_hidden || !is_hidden(&entry.file_name().to_string_lossy()))
        // Unreadable folders are skipped, like `find 2>/dev/null`.
        .filter_map(Result::ok);

    let mut batch = Vec::new();
    let mut last_sent = Instant::now();
    let mut found = 0;
    let mut truncated = false;
    for entry in entries {
        if cancel.is_cancelled() {
            break;
        }
        if matches(&entry.file_name().to_string_lossy()) {
            let is_dir = entry.file_type().is_dir();
            batch.push(SearchHit {
                path: entry.into_path(),
                is_dir,
            });
            found += 1;
            if found >= limit {
                truncated = true;
                break;
            }
        }
        if !batch.is_empty() && (batch.len() >= BATCH_SIZE || last_sent.elapsed() >= BATCH_INTERVAL)
        {
            if tx
                .blocking_send(SearchEvent::Found(std::mem::take(&mut batch)))
                .is_err()
            {
                return false; // Nobody is listening any more.
            }
            last_sent = Instant::now();
        }
    }
    if !batch.is_empty() {
        let _ = tx.blocking_send(SearchEvent::Found(batch));
    }
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tokio_stream::StreamExt;

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("docs/reports")).unwrap();
        fs::create_dir_all(root.join(".secret")).unwrap();
        fs::write(root.join("docs/report-1.txt"), "").unwrap();
        fs::write(root.join("docs/reports/report-2.txt"), "").unwrap();
        fs::write(root.join("docs/notes.md"), "").unwrap();
        fs::write(root.join(".secret/report-3.txt"), "").unwrap();
        dir
    }

    async fn run(
        root: &Path,
        word: &'static str,
        hidden: bool,
        limit: usize,
    ) -> (Vec<PathBuf>, bool) {
        let events: Vec<SearchEvent> = search(
            root.to_path_buf(),
            move |name| name.contains(word),
            hidden,
            limit,
            CancelHandle::new(),
        )
        .collect()
        .await;
        let mut found = Vec::new();
        let mut truncated = None;
        for event in events {
            match event {
                SearchEvent::Found(hits) => found.extend(hits.into_iter().map(|hit| hit.path)),
                SearchEvent::Done { truncated: t } => truncated = Some(t),
            }
        }
        found.sort();
        (found, truncated.expect("a Done event"))
    }

    #[tokio::test]
    async fn finds_matches_in_subfolders_and_skips_hidden_ones() {
        let dir = tree();
        let root = dir.path();
        let (found, truncated) = run(root, "report", false, 100).await;
        assert_eq!(
            found,
            [
                root.join("docs/report-1.txt"),
                root.join("docs/reports"),
                root.join("docs/reports/report-2.txt"),
            ]
        );
        assert!(!truncated);

        let (found, _) = run(root, "report-3", true, 100).await;
        assert_eq!(found, [root.join(".secret/report-3.txt")]);
    }

    #[tokio::test]
    async fn stops_at_the_limit() {
        let dir = tree();
        let (found, truncated) = run(dir.path(), "report", false, 2).await;
        assert_eq!(found.len(), 2);
        assert!(truncated);
    }
}
