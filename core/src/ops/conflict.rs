use std::sync::{Arc, Mutex};

use tokio::sync::oneshot;

/// How to resolve a single copy/move conflict where the destination already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    Replace,
    ReplaceAll,
    Skip,
    SkipAll,
}

/// Lets a running copy/move operation ask the UI what to do about a
/// destination that already exists, and block until it answers.
#[derive(Clone, Default)]
pub struct ConflictHandle {
    pending: Arc<Mutex<Option<oneshot::Sender<ConflictResolution>>>>,
}

impl ConflictHandle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Called by the UI to answer whichever conflict is currently pending, if any.
    pub fn respond(&self, resolution: ConflictResolution) {
        if let Some(tx) = self.pending.lock().unwrap().take() {
            let _ = tx.send(resolution);
        }
    }

    /// Blocks until the UI calls `respond`. If the sender is dropped without
    /// an answer (e.g. the operation's stream is dropped), defaults to `Skip`.
    pub(crate) async fn ask(&self) -> ConflictResolution {
        let (tx, rx) = oneshot::channel();
        *self.pending.lock().unwrap() = Some(tx);
        rx.await.unwrap_or(ConflictResolution::Skip)
    }
}
