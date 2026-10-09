use std::io;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("failed to read directory {path}: {source}")]
    ReadDir { path: PathBuf, source: io::Error },
    #[error("failed to read metadata for {path}: {source}")]
    Metadata { path: PathBuf, source: io::Error },
    /// A server folder (sftp://, ftp://) couldn't be listed.
    #[error("{0}")]
    Remote(String),
}
