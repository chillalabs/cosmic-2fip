use std::collections::HashSet;
use std::path::{Path, PathBuf};

use cosmic::widget::table;
use fs_ops::DirEntry;

use cosmic::widget::table::ItemInterface;
use fs_ops::settings::ViewMode;

use crate::file_item::{Column, FileItem, ListingOptions};

pub struct TabState {
    pub current_dir: PathBuf,
    /// Everything in `current_dir`, including hidden files, so toggling
    /// "Hide hidden files" doesn't need to re-read the disk.
    all_entries: Vec<DirEntry>,
    /// What the table shows: `all_entries` minus any filtered out.
    pub entries: table::MultiSelectModel<FileItem, Column>,
    pub error: Option<String>,
    pub back: Vec<PathBuf>,
    pub forward: Vec<PathBuf>,
    /// What to select once the listing being loaded arrives.
    pub pending_select: Option<PendingSelect>,
    /// Quick filter (Ctrl+S): `Some` while the filter bar is shown; only
    /// entries whose name matches the text are listed.
    pub filter: Option<String>,
    /// The selection, set aside (by path) while this tab's pane is inactive
    /// so it isn't highlighted there; restored when the pane is activated.
    stashed_selection: Option<HashSet<PathBuf>>,
    /// List or grid; each tab has its own.
    pub view_mode: ViewMode,
    /// Sort column and direction (`true` = ascending). Folders always come
    /// first either way.
    sort: (Column, bool),
}

/// A selection to apply after navigating, so the keyboard keeps its place.
#[derive(Debug, Clone)]
pub enum PendingSelect {
    /// Select this entry, e.g. the folder we just came up out of.
    Path(PathBuf),
    /// Select the first entry, e.g. after entering a folder.
    First,
}

impl TabState {
    pub fn new(current_dir: PathBuf) -> Self {
        Self {
            current_dir,
            all_entries: Vec::new(),
            entries: table::MultiSelectModel::new(vec![
                Column::Name,
                Column::Size,
                Column::Modified,
            ]),
            error: None,
            back: Vec::new(),
            forward: Vec::new(),
            pending_select: None,
            filter: None,
            stashed_selection: None,
            view_mode: ViewMode::default(),
            sort: (Column::Name, true),
        }
    }

    /// The sort column and direction (`true` = ascending).
    pub fn sort(&self) -> (Column, bool) {
        self.sort
    }

    /// Sorts by `category`: ascending the first time, reversing the
    /// direction when it's already the sort column (a header click).
    pub fn sort_by(&mut self, category: Column, options: &ListingOptions) {
        let ascending = match self.sort {
            (current, ascending) if current == category => !ascending,
            _ => true,
        };
        self.sort = (category, ascending);
        self.rebuild(options);
    }

    /// Hides the selection (remembering it) - for when the pane goes inactive.
    pub fn stash_selection(&mut self) {
        if self.stashed_selection.is_some() {
            return;
        }
        let selected: Vec<table::Entity> = self.entries.active().collect();
        let paths = selected
            .iter()
            .filter_map(|entity| self.entries.item(*entity))
            .map(|item| item.path.clone())
            .collect();
        for entity in selected {
            self.entries.deactivate(entity);
        }
        self.stashed_selection = Some(paths);
    }

    /// Brings back a selection hidden by [`Self::stash_selection`], for the
    /// entries still listed (a reload meanwhile may have removed some).
    pub fn restore_selection(&mut self) {
        let Some(paths) = self.stashed_selection.take() else {
            return;
        };
        let entities: Vec<table::Entity> = self.entries.iter().collect();
        for entity in entities {
            if self
                .entries
                .item(entity)
                .is_some_and(|item| paths.contains(&item.path))
            {
                self.entries.activate(entity);
            }
        }
    }

    /// Selects only `entity`, clearing any other selection.
    pub fn select_only(&mut self, entity: table::Entity) {
        let currently_selected: Vec<_> = self.entries.active().collect();
        for other in currently_selected {
            self.entries.deactivate(other);
        }
        self.entries.activate(entity);
    }

    /// Moves the selection one row down (`forward`) or up, like a keyboard
    /// cursor. From a multi-selection it moves from its last/first row; with
    /// nothing selected it starts at the first row. Returns the new row's
    /// position and the row count.
    pub fn move_selection(&mut self, forward: bool) -> Option<(usize, usize)> {
        let order: Vec<table::Entity> = self.entries.iter().collect();
        let count = order.len();
        let first_selected = order.iter().position(|e| self.entries.is_active(*e));
        let last_selected = order.iter().rposition(|e| self.entries.is_active(*e));
        let target = match (first_selected, last_selected) {
            (Some(_), Some(last)) if forward => (last + 1).min(count.checked_sub(1)?),
            (Some(first), Some(_)) => first.saturating_sub(1),
            _ => 0,
        };
        self.select_only(*order.get(target)?);
        Some((target, count))
    }

    /// Selects the first row if nothing is selected. Returns the selected
    /// row's position and the row count.
    pub fn ensure_selection(&mut self) -> Option<(usize, usize)> {
        let order: Vec<table::Entity> = self.entries.iter().collect();
        let position = match order.iter().position(|e| self.entries.is_active(*e)) {
            Some(position) => position,
            None => {
                self.select_only(*order.first()?);
                0
            }
        };
        Some((position, order.len()))
    }

    /// Applies (and clears) `pending_select`. Returns the selected row's
    /// position and the row count.
    pub fn apply_pending_select(&mut self) -> Option<(usize, usize)> {
        let pending = self.pending_select.take()?;
        let order: Vec<table::Entity> = self.entries.iter().collect();
        let position = match pending {
            PendingSelect::First => 0,
            PendingSelect::Path(path) => order
                .iter()
                .position(|e| self.entries.item(*e).is_some_and(|item| item.path == path))?,
        };
        self.select_only(*order.get(position)?);
        Some((position, order.len()))
    }

    /// Replaces the listing with a freshly loaded one.
    pub fn set_entries(&mut self, entries: Vec<DirEntry>, options: &ListingOptions) {
        self.all_entries = entries;
        self.entries.clear();
        self.rebuild(options);
    }

    /// Refills the table from `all_entries`, keeping the current sort order
    /// and re-selecting whatever was selected (if still visible).
    pub fn rebuild(&mut self, options: &ListingOptions) {
        let selected: HashSet<PathBuf> = self
            .entries
            .active()
            .filter_map(|entity| self.entries.item(entity))
            .map(|item| item.path.clone())
            .collect();

        self.entries.clear();
        // With the model empty this only records the sort for the header's
        // arrow; the order itself comes from inserting items pre-sorted below.
        // (The model's own sort puts folders last when descending, and skips
        // re-sorting when the column and direction are unchanged.)
        let (category, ascending) = self.sort;
        self.entries.sort(category, ascending);

        let mut items: Vec<FileItem> = self
            .all_entries
            .iter()
            .filter(|entry| !(options.hide_hidden && fs_ops::settings::is_hidden(&entry.name)))
            .filter(|entry| {
                self.filter
                    .as_deref()
                    .is_none_or(|filter| matches_filter(&entry.name, filter))
            })
            .map(|entry| FileItem::new(entry.clone(), options))
            .collect();
        items.sort_by(|a, b| compare_items(a, b, category, ascending));

        for item in items {
            let is_selected = selected.contains(&item.path);
            let inserted = self.entries.insert(item);
            if is_selected {
                inserted.activate();
            }
        }
    }
}

/// Listing order: folders before files, then by `category` in the given
/// direction, with the name breaking ties (e.g. equal sizes).
fn compare_items(a: &FileItem, b: &FileItem, category: Column, ascending: bool) -> std::cmp::Ordering {
    let by_column = a
        .compare(b, category)
        .then_with(|| a.compare(b, Column::Name));
    b.is_dir()
        .cmp(&a.is_dir())
        .then(if ascending { by_column } else { by_column.reverse() })
}

pub fn tab_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Quick-filter match, case-insensitive like Total Commander: plain text
/// matches anywhere in the name; with `*` or `?` it's a wildcard pattern
/// over the whole name (`*.txt`, `report-??.pdf`).
pub fn matches_filter(name: &str, filter: &str) -> bool {
    let filter = filter.trim().to_lowercase();
    if filter.is_empty() {
        return true;
    }
    let name = name.to_lowercase();
    if filter.contains(['*', '?']) {
        let pattern: Vec<char> = filter.chars().collect();
        let name: Vec<char> = name.chars().collect();
        wildcard_match(&pattern, &name)
    } else {
        name.contains(&filter)
    }
}

/// Classic `*` / `?` matching with backtracking to the last `*`.
fn wildcard_match(pattern: &[char], text: &[char]) -> bool {
    let (mut p, mut t) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some((p, t));
            p += 1;
        } else if let Some((star_p, star_t)) = star {
            // Let the last `*` swallow one more character and retry.
            p = star_p + 1;
            t = star_t + 1;
            star = Some((star_p, star_t + 1));
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|c| *c == '*')
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use fs_ops::settings::IconStyle;
    use fs_ops::EntryKind;

    use super::*;

    fn options() -> ListingOptions {
        ListingOptions {
            hide_hidden: false,
            icon_style: IconStyle::Colorful,
            user_dirs: Arc::new(HashMap::new()),
            show_thumbnails: false,
        }
    }

    fn tab_with_kinds(entries: &[(&str, EntryKind, u64)]) -> TabState {
        let entries = entries
            .iter()
            .map(|(name, kind, size)| DirEntry {
                name: name.to_string(),
                path: PathBuf::from("/dir").join(name),
                kind: *kind,
                size: *size,
                modified: None,
            })
            .collect();
        let mut tab = TabState::new(PathBuf::from("/dir"));
        tab.set_entries(entries, &options());
        tab
    }

    fn listed_names(tab: &TabState) -> Vec<String> {
        tab.entries
            .iter()
            .filter_map(|e| tab.entries.item(e))
            .map(|item| tab_label(&item.path))
            .collect()
    }

    #[test]
    fn folders_come_first_in_both_directions() {
        let mut tab = tab_with_kinds(&[
            ("b.txt", EntryKind::File, 1),
            ("zeta", EntryKind::Dir, 0),
            ("a.txt", EntryKind::File, 1),
            ("alpha", EntryKind::Dir, 0),
        ]);
        assert_eq!(listed_names(&tab), ["alpha", "zeta", "a.txt", "b.txt"]);

        tab.sort_by(Column::Name, &options()); // same column: descending
        assert_eq!(listed_names(&tab), ["zeta", "alpha", "b.txt", "a.txt"]);
    }

    #[test]
    fn sorting_by_size_survives_a_reload() {
        let mut tab = tab_with_kinds(&[
            ("big", EntryKind::File, 300),
            ("small", EntryKind::File, 1),
            ("mid", EntryKind::File, 20),
        ]);
        tab.sort_by(Column::Size, &options());
        assert_eq!(listed_names(&tab), ["small", "mid", "big"]);

        // A reload (same entries, same sort) must keep the size order.
        let reloaded = tab.all_entries.clone();
        tab.set_entries(reloaded, &options());
        assert_eq!(listed_names(&tab), ["small", "mid", "big"]);
    }

    fn tab_with(names: &[&str]) -> TabState {
        let options = ListingOptions {
            hide_hidden: false,
            icon_style: IconStyle::Colorful,
            user_dirs: Arc::new(HashMap::new()),
            show_thumbnails: false,
        };
        let entries = names
            .iter()
            .map(|name| DirEntry {
                name: name.to_string(),
                path: PathBuf::from("/dir").join(name),
                kind: EntryKind::File,
                size: 0,
                modified: None,
            })
            .collect();
        let mut tab = TabState::new(PathBuf::from("/dir"));
        tab.set_entries(entries, &options);
        tab
    }

    fn selected_names(tab: &TabState) -> Vec<String> {
        tab.entries
            .active()
            .filter_map(|e| tab.entries.item(e))
            .map(|item| tab_label(&item.path))
            .collect()
    }

    #[test]
    fn moving_with_nothing_selected_starts_at_the_first_row() {
        let mut tab = tab_with(&["a", "b", "c"]);
        assert_eq!(tab.move_selection(true), Some((0, 3)));
        assert_eq!(selected_names(&tab), ["a"]);
    }

    #[test]
    fn moves_down_and_up_and_stops_at_the_edges() {
        let mut tab = tab_with(&["a", "b", "c"]);
        tab.move_selection(true);
        tab.move_selection(true);
        tab.move_selection(true);
        assert_eq!(tab.move_selection(true), Some((2, 3)));
        assert_eq!(selected_names(&tab), ["c"]);

        tab.move_selection(false);
        tab.move_selection(false);
        assert_eq!(tab.move_selection(false), Some((0, 3)));
        assert_eq!(selected_names(&tab), ["a"]);
    }

    #[test]
    fn pending_path_selects_that_entry() {
        let mut tab = tab_with(&["a", "b", "c"]);
        tab.pending_select = Some(PendingSelect::Path(PathBuf::from("/dir/b")));
        assert_eq!(tab.apply_pending_select(), Some((1, 3)));
        assert_eq!(selected_names(&tab), ["b"]);
        assert!(tab.pending_select.is_none());
    }

    #[test]
    fn filter_matches_text_anywhere_ignoring_case() {
        assert!(matches_filter("Annual Report.pdf", "report"));
        assert!(matches_filter("notes.txt", ""));
        assert!(!matches_filter("notes.txt", "report"));
    }

    #[test]
    fn filter_supports_wildcards_over_the_whole_name() {
        assert!(matches_filter("notes.txt", "*.txt"));
        assert!(matches_filter("notes.TXT", "*.txt"));
        assert!(!matches_filter("notes.txt.bak", "*.txt"));
        assert!(matches_filter("report-01.pdf", "report-??.pdf"));
        assert!(!matches_filter("report-1.pdf", "report-??.pdf"));
        assert!(matches_filter("a-b-c", "a*c"));
    }

    #[test]
    fn filtered_listing_hides_non_matching_entries() {
        let mut tab = tab_with(&["apple", "banana", "apricot"]);
        tab.filter = Some("ap".to_string());
        tab.rebuild(&ListingOptions {
            hide_hidden: false,
            icon_style: IconStyle::Colorful,
            user_dirs: Arc::new(HashMap::new()),
            show_thumbnails: false,
        });
        let mut names: Vec<String> = tab
            .entries
            .iter()
            .filter_map(|e| tab.entries.item(e))
            .map(|item| tab_label(&item.path))
            .collect();
        names.sort();
        assert_eq!(names, ["apple", "apricot"]);
    }

    #[test]
    fn stashed_selection_is_hidden_then_restored() {
        let mut tab = tab_with(&["a", "b", "c"]);
        tab.move_selection(true);
        tab.move_selection(true);
        assert_eq!(selected_names(&tab), ["b"]);

        tab.stash_selection();
        assert!(selected_names(&tab).is_empty());

        tab.restore_selection();
        assert_eq!(selected_names(&tab), ["b"]);
    }

    #[test]
    fn moving_in_an_empty_folder_does_nothing() {
        let mut tab = tab_with(&[]);
        assert_eq!(tab.move_selection(true), None);
    }
}
