use std::path::PathBuf;

use fs_ops::ops::{CancelHandle, ConflictHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    Copy,
    Move,
    Delete,
    Compress,
}

impl OpKind {
    pub fn label(self) -> &'static str {
        match self {
            OpKind::Copy => "Copying",
            OpKind::Move => "Moving",
            OpKind::Delete => "Deleting",
            OpKind::Compress => "Compressing",
        }
    }
}

pub struct OperationState {
    pub kind: OpKind,
    pub cancel: CancelHandle,
    /// Unused for `Delete`/`Compress` (nothing to collide with), but
    /// always present so callers don't have to special-case it.
    pub conflict: ConflictHandle,
    pub current_file: Option<PathBuf>,
    pub files_done: usize,
    pub files_total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub error: Option<String>,
}

impl OperationState {
    pub fn new(kind: OpKind, cancel: CancelHandle, conflict: ConflictHandle) -> Self {
        Self {
            kind,
            cancel,
            conflict,
            current_file: None,
            files_done: 0,
            files_total: 0,
            bytes_done: 0,
            bytes_total: 0,
            error: None,
        }
    }

    /// Fraction complete in `[0.0, 1.0]`, preferring byte-level granularity
    /// when it's tracked (it isn't for a same-filesystem move or a delete).
    pub fn fraction(&self) -> f32 {
        if self.bytes_total > 0 {
            self.bytes_done as f32 / self.bytes_total as f32
        } else if self.files_total > 0 {
            self.files_done as f32 / self.files_total as f32
        } else {
            0.0
        }
    }
}
