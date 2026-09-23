//! The panels' state (tabs, folders, active pane), saved so the next launch
//! reopens where the user left off.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::settings::ViewMode;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub left: PaneSession,
    pub right: PaneSession,
    /// Whether the right pane (rather than the left) was the active one.
    pub right_active: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PaneSession {
    /// Each tab's folder, in tab order.
    pub tabs: Vec<PathBuf>,
    /// Index into `tabs` of the tab that was showing.
    pub active_tab: usize,
    /// Each tab's list/grid mode, parallel to `tabs` (missing entries, e.g.
    /// from an older session file, mean list view).
    pub tab_view_modes: Vec<ViewMode>,
}

impl PaneSession {
    /// The tabs to reopen with their view modes, each moved to its nearest
    /// folder that still exists (or `fallback` if none does), plus which one
    /// to show. Never empty.
    pub fn restore(&self, fallback: &Path) -> (Vec<(PathBuf, ViewMode)>, usize) {
        let tabs: Vec<(PathBuf, ViewMode)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(index, dir)| {
                let dir = nearest_existing_dir(dir).unwrap_or_else(|| fallback.to_path_buf());
                let view_mode = self.tab_view_modes.get(index).copied().unwrap_or_default();
                (dir, view_mode)
            })
            .collect();
        if tabs.is_empty() {
            return (vec![(fallback.to_path_buf(), ViewMode::default())], 0);
        }
        let active = self.active_tab.min(tabs.len() - 1);
        (tabs, active)
    }
}

/// `dir` itself if it's still a directory, else its closest existing parent
/// (e.g. a deleted folder reopens at its parent).
fn nearest_existing_dir(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|ancestor| ancestor.is_dir())
        .map(Path::to_path_buf)
}

/// Loads the last saved session; an empty default (which restores to the
/// fallback folder) if there's none or it can't be read.
pub fn load() -> Session {
    load_from(&session_path())
}

pub fn save(session: &Session) -> Result<(), String> {
    save_to(&session_path(), session)
}

fn load_from(path: &Path) -> Session {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

fn save_to(path: &Path, session: &Session) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(session)
        .map_err(|err| format!("failed to serialize session: {err}"))?;
    fs::write(path, json).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn session_path() -> PathBuf {
    crate::settings::config_dir().join("session.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("session.json");
        let session = Session {
            left: PaneSession {
                tabs: vec![PathBuf::from("/a"), PathBuf::from("/b")],
                active_tab: 1,
                tab_view_modes: vec![ViewMode::Grid, ViewMode::List],
            },
            right: PaneSession {
                tabs: vec![PathBuf::from("/c")],
                active_tab: 0,
                tab_view_modes: vec![ViewMode::List],
            },
            right_active: true,
        };

        save_to(&path, &session).unwrap();

        assert_eq!(load_from(&path), session);
    }

    #[test]
    fn missing_or_corrupt_file_yields_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        assert_eq!(load_from(&path), Session::default());
        fs::write(&path, b"not json").unwrap();
        assert_eq!(load_from(&path), Session::default());
    }

    #[test]
    fn restore_moves_deleted_folders_to_their_parent_and_clamps_the_tab() {
        let dir = tempfile::tempdir().unwrap();
        let fallback = dir.path().join("home");
        let pane = PaneSession {
            tabs: vec![dir.path().join("gone").join("deeper")],
            active_tab: 7,
            tab_view_modes: vec![ViewMode::Grid],
        };

        let (tabs, active) = pane.restore(&fallback);

        assert_eq!(tabs, vec![(dir.path().to_path_buf(), ViewMode::Grid)]);
        assert_eq!(active, 0);
    }

    #[test]
    fn restoring_nothing_opens_the_fallback() {
        let fallback = PathBuf::from("/home/user");
        let (tabs, active) = PaneSession::default().restore(&fallback);
        assert_eq!(tabs, vec![(fallback, ViewMode::List)]);
        assert_eq!(active, 0);
    }
}
