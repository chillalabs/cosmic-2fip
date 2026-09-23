//! Async file operations (copy, move, delete-to-trash, rename, compress) with progress
//! reporting and cooperative cancellation.
//!
//! Copy/move/delete are exposed as streams of [`OpEvent`] rather than a single
//! future, so a caller (the UI) can show live progress and let the user cancel
//! mid-operation. Cancellation is checked between files, not mid-file, so
//! cancelling while copying one very large file lets that file finish first.

mod compress;
mod conflict;
mod copy;
mod delete;
mod engine;
mod mkdir;
mod mv;
mod plan;
mod rename;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub use compress::compress_zip;
pub use conflict::{ConflictHandle, ConflictResolution};
pub use copy::copy;
pub use delete::delete_to_trash;
pub use mkdir::create_dir;
pub use mv::move_paths;
pub use rename::rename;

/// A cooperative cancellation flag shared between the UI and a running operation.
#[derive(Debug, Clone, Default)]
pub struct CancelHandle(Arc<AtomicBool>);

impl CancelHandle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone)]
pub struct OpProgress {
    pub current_file: PathBuf,
    pub files_done: usize,
    pub files_total: usize,
    /// Byte-level progress. Not tracked for operations where it isn't
    /// meaningful (e.g. a same-filesystem move, which is an instant rename);
    /// both fields are `0` in that case.
    pub bytes_done: u64,
    pub bytes_total: u64,
}

#[derive(Debug, Clone)]
pub enum OpEvent {
    Progress(OpProgress),
    /// `dest` already exists. Sent once, then the operation blocks until the
    /// UI calls the matching [`ConflictHandle::respond`].
    Conflict { src: PathBuf, dest: PathBuf },
    Done,
    Cancelled,
    Error(String),
}
