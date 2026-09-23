use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cosmic::app::Task;
use cosmic::iced::widget::scrollable::{snap_to, RelativeOffset};
use cosmic::iced::Length;
use cosmic::widget::menu::key_bind::KeyBind;
use cosmic::widget::{self, segmented_button, table};
use cosmic::Element;

use crate::app::{Message, PaneId};
use crate::context_menu::item_menu;
use fs_ops::settings::ViewMode;
use crate::file_item::{format_size, Column, FileItem, ListingOptions};
use crate::keybinds::Action;
use crate::launch::open_with_default_app;
use crate::tab::{tab_label, PendingSelect, TabState};

pub type TabId = segmented_button::Entity;

/// Identifies a file's thumbnail: its path plus modification time (seconds),
/// so an edited file gets a new preview.
pub type ThumbnailKey = (PathBuf, u64);

/// At most this many thumbnails are loaded or generated at once.
const THUMBNAIL_WORKERS: usize = 4;
/// Folders with more previewable files than this only preview the first ones.
const MAX_THUMBNAILS_PER_FOLDER: usize = 2000;

#[derive(Debug, Clone)]
pub enum PaneMessage {
    DirLoaded(TabId, PathBuf, Result<Vec<fs_ops::DirEntry>, String>),
    EntryDoubleClicked(table::Entity),
    /// `additive` is true when the click was Ctrl-held: toggle this item's
    /// selection without touching the others. Otherwise, select only this item.
    EntrySelected(table::Entity, bool),
    /// Right-click: selects the item unless it's already part of the
    /// selection, so context-menu actions apply to what was clicked.
    EntryRightClicked(table::Entity),
    /// A context-menu action; handled by the app, not the pane.
    Action(Action),
    /// A content preview finished loading (`None`: none could be made).
    ThumbnailReady(ThumbnailKey, Option<PathBuf>),
    SelectAll,
    /// Keyboard cursor: select the next / previous row.
    SelectNext,
    SelectPrevious,
    /// Enters the selected folder (does nothing for files).
    EnterSelectedFolder,
    /// Ctrl+S: shows the quick filter bar (if hidden) and focuses it.
    OpenFilter,
    FilterChanged(String),
    /// Hides the filter bar and shows everything again.
    CloseFilter,
    /// Turns the path bar into a text box holding the current path.
    StartPathEdit,
    PathEditChanged(String),
    /// Goes to the typed path (a file's folder, selecting the file).
    SubmitPathEdit,
    CancelPathEdit,
    SortBy(Column),
    /// Jumps the active tab straight to an arbitrary path (e.g. a favorite).
    Navigate(PathBuf),
    GoUp,
    GoBack,
    GoForward,
    NewTab,
    /// Shows the next (`true`) or previous tab, wrapping around.
    CycleTab(bool),
    CloseTab(TabId),
    CloseActiveTab,
    SelectTab(TabId),
}

pub struct PaneState {
    id: PaneId,
    tabs: segmented_button::SingleSelectModel,
    options: ListingOptions,
    /// The file listing's scroll area, so keyboard moves can keep the
    /// selected row in view.
    scroll_id: widget::Id,
    /// The quick filter's text box, so Ctrl+S can focus it.
    filter_input_id: widget::Id,
    /// The path bar's text while it's being edited (`None`: showing the
    /// clickable breadcrumbs instead).
    path_edit: Option<String>,
    /// Why the typed path couldn't be opened, shown under the path bar.
    path_edit_error: Option<String>,
    path_input_id: widget::Id,
    /// Loaded content previews, shown instead of type icons.
    thumbnails: HashMap<ThumbnailKey, widget::icon::Handle>,
    /// Previews already asked for (loaded, loading, or failed), so each file
    /// is only tried once.
    thumbnails_requested: HashSet<ThumbnailKey>,
}

impl PaneState {
    /// Creates a pane with one tab per `(folder, view mode)` in `tabs` (must
    /// not be empty), showing the tab at index `active_tab`.
    pub fn new(
        id: PaneId,
        tab_list: Vec<(PathBuf, ViewMode)>,
        active_tab: usize,
        options: ListingOptions,
    ) -> (Self, Task<Message>) {
        let mut tabs = segmented_button::SingleSelectModel::default();
        let mut tasks = Vec::with_capacity(tab_list.len());
        for (index, (dir, view_mode)) in tab_list.into_iter().enumerate() {
            let mut tab_state = TabState::new(dir.clone());
            tab_state.view_mode = view_mode;
            let tab_id = tabs
                .insert()
                .text(tab_label(&dir))
                .data(tab_state)
                .closable()
                .id();
            if index == active_tab {
                tabs.activate(tab_id);
            }
            tasks.push(load_dir(id, tab_id, dir));
        }

        let pane = Self {
            id,
            tabs,
            options,
            scroll_id: widget::Id::unique(),
            filter_input_id: widget::Id::unique(),
            path_edit: None,
            path_edit_error: None,
            path_input_id: widget::Id::unique(),
            thumbnails: HashMap::new(),
            thumbnails_requested: HashSet::new(),
        };
        (pane, Task::batch(tasks))
    }

    /// This pane's tabs and which one is showing, for saving the session.
    pub fn session(&self) -> fs_ops::session::PaneSession {
        let active = self.tabs.active();
        let mut active_tab = 0;
        let mut tabs = Vec::new();
        let mut tab_view_modes = Vec::new();
        for (index, tab) in self.tabs.iter().enumerate() {
            if tab == active {
                active_tab = index;
            }
            if let Some(tab_state) = self.tabs.data::<TabState>(tab) {
                tabs.push(tab_state.current_dir.clone());
                tab_view_modes.push(tab_state.view_mode);
            }
        }
        fs_ops::session::PaneSession {
            tabs,
            active_tab,
            tab_view_modes,
        }
    }

    pub fn update(&mut self, message: PaneMessage) -> Task<Message> {
        match message {
            PaneMessage::DirLoaded(tab, path, result) => {
                let Some(tab_state) = self.tabs.data_mut::<TabState>(tab) else {
                    // The tab was closed before the listing finished loading.
                    return Task::none();
                };
                if path != tab_state.current_dir {
                    // Stale response from a directory we've since navigated away from.
                    return Task::none();
                }
                match result {
                    Ok(entries) => {
                        tab_state.error = None;
                        tab_state.set_entries(entries, &self.options);
                    }
                    Err(err) => {
                        tab_state.set_entries(Vec::new(), &self.options);
                        tab_state.error = Some(err);
                    }
                }
                let selected = tab_state.apply_pending_select();
                let scroll = if self.tabs.is_active(tab) {
                    self.scroll_to_row(selected)
                } else {
                    Task::none()
                };
                Task::batch([scroll, self.request_thumbnails(tab)])
            }
            PaneMessage::EntryDoubleClicked(entity) => {
                let Some(tab_state) = self.tabs.active_data::<TabState>() else {
                    return Task::none();
                };
                let Some(item) = tab_state.entries.item(entity) else {
                    return Task::none();
                };
                if item.is_dir() {
                    let path = item.path.clone();
                    return self.navigate_active(path);
                }
                open_with_default_app(&item.path);
                Task::none()
            }
            PaneMessage::EntrySelected(entity, additive) => {
                if let Some(tab_state) = self.tabs.active_data_mut::<TabState>() {
                    if additive {
                        tab_state.entries.activate(entity);
                    } else {
                        tab_state.select_only(entity);
                    }
                }
                Task::none()
            }
            PaneMessage::EntryRightClicked(entity) => {
                if let Some(tab_state) = self.tabs.active_data_mut::<TabState>() {
                    if !tab_state.entries.is_active(entity) {
                        tab_state.select_only(entity);
                    }
                }
                Task::none()
            }
            PaneMessage::Action(_) => Task::none(),
            PaneMessage::ThumbnailReady(key, thumbnail) => {
                if let Some(png) = thumbnail {
                    self.thumbnails.insert(key, widget::icon::from_path(png));
                }
                Task::none()
            }
            PaneMessage::SelectAll => {
                if let Some(tab_state) = self.tabs.active_data_mut::<TabState>() {
                    let all: Vec<_> = tab_state.entries.iter().collect();
                    for entity in all {
                        tab_state.entries.activate(entity);
                    }
                }
                Task::none()
            }
            PaneMessage::SelectNext | PaneMessage::SelectPrevious => {
                let forward = matches!(message, PaneMessage::SelectNext);
                let selected = self
                    .tabs
                    .active_data_mut::<TabState>()
                    .and_then(|tab_state| tab_state.move_selection(forward));
                self.scroll_to_row(selected)
            }
            PaneMessage::EnterSelectedFolder => {
                let Some(dir) = self.selected_single_dir() else {
                    return Task::none();
                };
                if let Some(tab_state) = self.tabs.active_data_mut::<TabState>() {
                    tab_state.pending_select = Some(PendingSelect::First);
                }
                self.navigate_active(dir)
            }
            PaneMessage::OpenFilter => {
                if let Some(tab_state) = self.tabs.active_data_mut::<TabState>() {
                    tab_state.filter.get_or_insert_with(String::new);
                }
                widget::text_input::focus(self.filter_input_id.clone())
            }
            PaneMessage::FilterChanged(text) => {
                let options = self.options.clone();
                let selected = self.tabs.active_data_mut::<TabState>().and_then(|tab_state| {
                    tab_state.filter = Some(text);
                    tab_state.rebuild(&options);
                    // Keep a row selected so Enter / arrows act on a match.
                    tab_state.ensure_selection()
                });
                self.scroll_to_row(selected)
            }
            PaneMessage::CloseFilter => {
                self.close_filter();
                Task::none()
            }
            PaneMessage::SortBy(category) => {
                let options = self.options.clone();
                if let Some(tab_state) = self.tabs.active_data_mut::<TabState>() {
                    tab_state.sort_by(category, &options);
                }
                Task::none()
            }
            PaneMessage::Navigate(path) => self.navigate_active(path),
            PaneMessage::GoUp => {
                let Some(tab_state) = self.tabs.active_data::<TabState>() else {
                    return Task::none();
                };
                let Some(parent) = tab_state.current_dir.parent().map(Path::to_path_buf) else {
                    return Task::none();
                };
                // Land on the folder we came out of, like Total Commander.
                let came_from = tab_state.current_dir.clone();
                if let Some(tab_state) = self.tabs.active_data_mut::<TabState>() {
                    tab_state.pending_select = Some(PendingSelect::Path(came_from));
                }
                self.navigate_active(parent)
            }
            PaneMessage::GoBack => {
                let Some(tab_state) = self.tabs.active_data_mut::<TabState>() else {
                    return Task::none();
                };
                let Some(prev) = tab_state.back.pop() else {
                    return Task::none();
                };
                let current = std::mem::replace(&mut tab_state.current_dir, prev.clone());
                tab_state.forward.push(current);
                self.set_active_dir(prev)
            }
            PaneMessage::GoForward => {
                let Some(tab_state) = self.tabs.active_data_mut::<TabState>() else {
                    return Task::none();
                };
                let Some(next) = tab_state.forward.pop() else {
                    return Task::none();
                };
                let current = std::mem::replace(&mut tab_state.current_dir, next.clone());
                tab_state.back.push(current);
                self.set_active_dir(next)
            }
            PaneMessage::NewTab => {
                // The new tab starts as a copy of the current one's folder and view.
                let (dir, view_mode) = self
                    .tabs
                    .active_data::<TabState>()
                    .map(|tab_state| (tab_state.current_dir.clone(), tab_state.view_mode))
                    .unwrap_or_else(|| (PathBuf::from("/"), ViewMode::default()));
                let mut tab_state = TabState::new(dir.clone());
                tab_state.view_mode = view_mode;
                self.tabs
                    .insert()
                    .text(tab_label(&dir))
                    .data(tab_state)
                    .closable()
                    .activate();
                let tab_id = self.tabs.active();
                load_dir(self.id, tab_id, dir)
            }
            PaneMessage::CloseTab(entity) => {
                if self.tabs.iter().count() <= 1 {
                    // Always keep at least one tab open.
                    return Task::none();
                }
                let was_active = self.tabs.is_active(entity);
                self.tabs.remove(entity);
                if was_active {
                    let next = self.tabs.iter().next();
                    if let Some(next) = next {
                        self.tabs.activate(next);
                    }
                }
                Task::none()
            }
            PaneMessage::CycleTab(forward) => {
                let tabs: Vec<TabId> = self.tabs.iter().collect();
                let active = self.tabs.active();
                if let Some(position) = tabs.iter().position(|tab| *tab == active) {
                    let next = if forward {
                        (position + 1) % tabs.len()
                    } else {
                        (position + tabs.len() - 1) % tabs.len()
                    };
                    self.tabs.activate(tabs[next]);
                }
                Task::none()
            }
            PaneMessage::CloseActiveTab => {
                let active = self.tabs.active();
                self.update(PaneMessage::CloseTab(active))
            }
            PaneMessage::SelectTab(entity) => {
                self.cancel_path_edit();
                self.tabs.activate(entity);
                Task::none()
            }
            PaneMessage::StartPathEdit => {
                let mut text = self.current_dir().display().to_string();
                if !text.ends_with('/') {
                    // Ready for typing a subfolder name straight away.
                    text.push('/');
                }
                self.path_edit = Some(text);
                self.path_edit_error = None;
                Task::batch([
                    widget::text_input::focus(self.path_input_id.clone()),
                    widget::text_input::select_all(self.path_input_id.clone()),
                ])
            }
            PaneMessage::PathEditChanged(text) => {
                self.path_edit = Some(text);
                self.path_edit_error = None;
                Task::none()
            }
            PaneMessage::SubmitPathEdit => {
                let Some(text) = self.path_edit.clone() else {
                    return Task::none();
                };
                let path = resolve_typed_path(&text, &self.current_dir(), &home_dir());
                if path.is_dir() {
                    self.cancel_path_edit();
                    self.navigate_active(path)
                } else if path.exists() {
                    // A file: open its folder with the file selected.
                    let Some(parent) = path.parent().map(Path::to_path_buf) else {
                        return Task::none();
                    };
                    self.cancel_path_edit();
                    if let Some(tab_state) = self.tabs.active_data_mut::<TabState>() {
                        tab_state.pending_select = Some(PendingSelect::Path(path));
                    }
                    self.navigate_active(parent)
                } else {
                    self.path_edit_error = Some(format!("Not found: {}", path.display()));
                    Task::none()
                }
            }
            PaneMessage::CancelPathEdit => {
                self.cancel_path_edit();
                Task::none()
            }
        }
    }

    /// Navigates the active tab to `path`, recording `current_dir` in its back history.
    fn navigate_active(&mut self, path: PathBuf) -> Task<Message> {
        if let Some(tab_state) = self.tabs.active_data_mut::<TabState>() {
            tab_state.back.push(tab_state.current_dir.clone());
            tab_state.forward.clear();
        }
        self.set_active_dir(path)
    }

    /// Sets the active tab's current directory (without touching history) and loads it.
    fn set_active_dir(&mut self, path: PathBuf) -> Task<Message> {
        self.cancel_path_edit();
        let tab_id = self.tabs.active();
        if let Some(tab_state) = self.tabs.data_mut::<TabState>(tab_id) {
            tab_state.current_dir = path.clone();
            tab_state.error = None;
            // A quick filter belongs to the folder it was typed in.
            tab_state.filter = None;
        }
        let _ = self.tabs.text_set(tab_id, tab_label(&path));
        load_dir(self.id, tab_id, path)
    }

    /// Hides this pane's selection while it's inactive (remembering it).
    pub fn deactivate(&mut self) {
        if let Some(tab_state) = self.tabs.active_data_mut::<TabState>() {
            tab_state.stash_selection();
        }
    }

    /// Shows the selection hidden by [`Self::deactivate`] again.
    pub fn activate(&mut self) {
        if let Some(tab_state) = self.tabs.active_data_mut::<TabState>() {
            tab_state.restore_selection();
        }
    }

    /// The active tab's view mode.
    pub fn view_mode(&self) -> ViewMode {
        self.tabs
            .active_data::<TabState>()
            .map(|tab_state| tab_state.view_mode)
            .unwrap_or_default()
    }

    /// Changes only the active tab's view mode.
    pub fn set_view_mode(&mut self, view_mode: ViewMode) {
        if let Some(tab_state) = self.tabs.active_data_mut::<TabState>() {
            tab_state.view_mode = view_mode;
        }
    }

    /// Leaves path editing (if active), showing the breadcrumbs again.
    /// Returns whether it was editing.
    pub fn cancel_path_edit(&mut self) -> bool {
        self.path_edit_error = None;
        self.path_edit.take().is_some()
    }

    /// Whether the active tab's quick filter bar is showing.
    pub fn filter_open(&self) -> bool {
        self.tabs
            .active_data::<TabState>()
            .is_some_and(|tab_state| tab_state.filter.is_some())
    }

    /// Hides the active tab's quick filter bar and lists everything again,
    /// keeping the selection.
    pub fn close_filter(&mut self) {
        let options = self.options.clone();
        if let Some(tab_state) = self.tabs.active_data_mut::<TabState>() {
            if tab_state.filter.take().is_some() {
                tab_state.rebuild(&options);
            }
        }
    }

    /// Scrolls the listing so row `position` (of `count`) is visible.
    ///
    /// Snapping to the row's *relative* position always keeps it on screen
    /// (first row at the top, last at the bottom) without knowing row heights.
    fn scroll_to_row(&self, selected: Option<(usize, usize)>) -> Task<Message> {
        let Some((position, count)) = selected else {
            return Task::none();
        };
        let y = if count > 1 {
            position as f32 / (count - 1) as f32
        } else {
            0.0
        };
        snap_to(
            self.scroll_id.clone(),
            RelativeOffset {
                x: None,
                y: Some(y),
            },
        )
    }

    /// Applies new display options (hidden files, icons) to every tab of this pane.
    pub fn set_options(&mut self, options: ListingOptions) -> Task<Message> {
        self.options = options;
        let tabs: Vec<TabId> = self.tabs.iter().collect();
        let mut tasks = Vec::new();
        for tab in tabs {
            if let Some(tab_state) = self.tabs.data_mut::<TabState>(tab) {
                tab_state.rebuild(&self.options);
            }
            // e.g. thumbnails were just switched on
            tasks.push(self.request_thumbnails(tab));
        }
        Task::batch(tasks)
    }

    /// Starts loading previews for `tab`'s previewable files that don't have
    /// one yet (from the shared cache, or made by a system thumbnailer).
    /// Runs in the background, a few at a time; each result updates the view.
    fn request_thumbnails(&mut self, tab: TabId) -> Task<Message> {
        if !self.options.show_thumbnails {
            return Task::none();
        }
        let Some(tab_state) = self.tabs.data::<TabState>(tab) else {
            return Task::none();
        };
        let mut wanted = Vec::new();
        for entity in tab_state.entries.iter() {
            let Some(item) = tab_state.entries.item(entity) else {
                continue;
            };
            let (Some(mime), Some(modified)) = (item.mime(), item.modified()) else {
                continue;
            };
            if !fs_ops::thumbnail::can_thumbnail(mime) {
                continue;
            }
            let key = (item.path.clone(), fs_ops::thumbnail::secs(modified));
            if !self.thumbnails_requested.contains(&key) {
                wanted.push((key, mime.to_string()));
            }
            if wanted.len() >= MAX_THUMBNAILS_PER_FOLDER {
                break;
            }
        }

        let pane = self.id;
        let tasks = wanted.into_iter().map(|(key, mime)| {
            self.thumbnails_requested.insert(key.clone());
            cosmic::task::future(async move {
                let _permit = thumbnail_workers().acquire().await.ok();
                let path = key.0.clone();
                let thumbnail = tokio::task::spawn_blocking(move || {
                    fs_ops::thumbnail::thumbnail(&path, &mime)
                })
                .await
                .ok()
                .flatten();
                Message::Pane(pane, PaneMessage::ThumbnailReady(key, thumbnail))
            })
        });
        Task::batch(tasks.collect::<Vec<_>>())
    }

    /// The picture for `item`: its content preview when thumbnails are on and
    /// one is loaded, else its file-type icon.
    fn item_icon(&self, item: &FileItem) -> widget::icon::Handle {
        if self.options.show_thumbnails {
            if let Some(modified) = item.modified() {
                let key = (item.path.clone(), fs_ops::thumbnail::secs(modified));
                if let Some(thumbnail) = self.thumbnails.get(&key) {
                    return thumbnail.clone();
                }
            }
        }
        item.icon().clone()
    }

    /// The active tab's current directory.
    pub fn current_dir(&self) -> PathBuf {
        self.tabs
            .active_data::<TabState>()
            .map(|tab_state| tab_state.current_dir.clone())
            .unwrap_or_else(|| PathBuf::from("/"))
    }

    /// Paths of the items selected in the active tab's file listing.
    pub fn selected_paths(&self) -> Vec<PathBuf> {
        let Some(tab_state) = self.tabs.active_data::<TabState>() else {
            return Vec::new();
        };
        tab_state
            .entries
            .active()
            .filter_map(|entity| tab_state.entries.item(entity))
            .map(|item| item.path.clone())
            .collect()
    }

    /// The single selected item's path, if exactly one directory is selected.
    pub fn selected_single_dir(&self) -> Option<PathBuf> {
        let tab_state = self.tabs.active_data::<TabState>()?;
        let mut selected = tab_state.entries.active();
        let item = tab_state.entries.item(selected.next()?)?;
        (selected.next().is_none() && item.is_dir()).then(|| item.path.clone())
    }

    /// A short status line for the active tab: total item count, or, when
    /// something is selected, how many are selected and their combined size.
    pub fn status_text(&self) -> String {
        let Some(tab_state) = self.tabs.active_data::<TabState>() else {
            return String::new();
        };
        let total_items = tab_state.entries.iter().count();
        let selected: Vec<_> = tab_state.entries.active().collect();
        if selected.is_empty() {
            let filtered = match tab_state.filter.as_deref() {
                Some(filter) if !filter.trim().is_empty() => " match the filter",
                _ => "",
            };
            format!(
                "{total_items} item{}{filtered}",
                if total_items == 1 { "" } else { "s" }
            )
        } else {
            let selected_size: u64 = selected
                .iter()
                .filter_map(|&entity| tab_state.entries.item(entity))
                .map(|item| item.size())
                .sum();
            format!(
                "{} of {total_items} selected ({})",
                selected.len(),
                format_size(selected_size)
            )
        }
    }

    /// Reloads the active tab's current directory (e.g. after a file operation
    /// elsewhere may have changed its contents), without touching history.
    pub fn reload(&self) -> Task<Message> {
        let tab_id = self.tabs.active();
        let Some(tab_state) = self.tabs.data::<TabState>(tab_id) else {
            return Task::none();
        };
        load_dir(self.id, tab_id, tab_state.current_dir.clone())
    }

    pub fn view(
        &self,
        is_active: bool,
        additive_select: bool,
        keybinds: &HashMap<KeyBind, Action>,
        can_paste: bool,
    ) -> Element<'_, PaneMessage> {
        let active = self.tabs.active();
        let tab_state = self.tabs.data::<TabState>(active);

        let tab_bar = widget::tab_bar::horizontal(&self.tabs)
            .on_activate(PaneMessage::SelectTab)
            .on_close(PaneMessage::CloseTab);

        let nav_button = |icon: widget::icon::Handle, tooltip: &'static str, message| {
            widget::button::icon(icon)
                .tooltip(tooltip)
                .on_press(message)
        };
        let header = widget::Row::new()
            .spacing(4)
            .align_y(cosmic::iced::Alignment::Center)
            .push(nav_button(
                widget::icon::from_name("go-previous-symbolic").handle(),
                "Back (Alt+←)",
                PaneMessage::GoBack,
            ))
            .push(nav_button(
                widget::icon::from_name("go-next-symbolic").handle(),
                "Forward (Alt+→)",
                PaneMessage::GoForward,
            ))
            .push(
                // Arrow + "[..]", like Total Commander's parent-folder entry.
                widget::button::text("[..]")
                    .leading_icon(parent_folder_icon())
                    .tooltip("Up one folder (Backspace)")
                    .on_press(PaneMessage::GoUp),
            )
            .push(nav_button(
                widget::icon::from_name("list-add-symbolic").handle(),
                "New tab (Ctrl+T)",
                PaneMessage::NewTab,
            ))
            .push(self.path_bar(tab_state, is_active));

        let content: Element<'_, PaneMessage> = match tab_state {
            None => widget::text("No tab open").into(),
            Some(t) => {
                if let Some(err) = &t.error {
                    widget::text(err.clone()).into()
                } else {
                    match t.view_mode {
                        ViewMode::List => widget::Column::new()
                            .push(list_header(t))
                            .push(widget::divider::horizontal::default())
                            .push(
                                widget::scrollable(list_view(
                                    self,
                                    t,
                                    additive_select,
                                    keybinds,
                                    can_paste,
                                ))
                                .id(self.scroll_id.clone())
                                .height(Length::Fill),
                            )
                            .into(),
                        ViewMode::Grid => {
                            // Needs the pane's width to know how many columns fit.
                            let keybinds = keybinds.clone();
                            let scroll_id = self.scroll_id.clone();
                            widget::responsive(move |size| {
                                widget::scrollable(grid_view(
                                    self,
                                    t,
                                    size.width,
                                    additive_select,
                                    &keybinds,
                                    can_paste,
                                ))
                                .id(scroll_id.clone())
                                .height(Length::Fill)
                                .into()
                            })
                            .into()
                        }
                    }
                }
            }
        };

        widget::Column::new()
            .spacing(8)
            .push(tab_bar)
            .push(header)
            .push(content)
            .push_maybe(tab_state.and_then(|t| t.filter.as_deref()).map(|filter| {
                widget::search_input("Filter (e.g. report or *.txt)", filter)
                    .id(self.filter_input_id.clone())
                    .on_input(PaneMessage::FilterChanged)
                    // Enter opens the selected match, like Total Commander.
                    .on_submit(|_| PaneMessage::Action(Action::Open))
                    .on_clear(PaneMessage::CloseFilter)
            }))
            .push(widget::text(self.status_text()))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl PaneState {
    /// The path bar: clickable breadcrumbs (one button per folder, highlighted
    /// on hover), or a text box while editing. Clicking the bar's empty space
    /// or the pencil starts editing.
    fn path_bar<'a>(&'a self, tab_state: Option<&'a TabState>, is_active: bool) -> Element<'a, PaneMessage> {
        if let Some(text) = &self.path_edit {
            let input = widget::text_input("Type a folder path", text.as_str())
                .id(self.path_input_id.clone())
                .on_input(PaneMessage::PathEditChanged)
                .on_submit(|_| PaneMessage::SubmitPathEdit)
                .width(Length::Fill);
            let cancel = widget::button::icon(widget::icon::from_name("window-close-symbolic"))
                .tooltip("Cancel (Esc)")
                .on_press(PaneMessage::CancelPathEdit);
            return widget::Column::new()
                .width(Length::Fill)
                .push(
                    widget::Row::new()
                        .spacing(4)
                        .align_y(cosmic::iced::Alignment::Center)
                        .push(input)
                        .push(cancel),
                )
                .push_maybe(
                    self.path_edit_error
                        .as_deref()
                        .map(|err| widget::text::caption(err.to_string())),
                )
                .into();
        }

        let Some(tab_state) = tab_state else {
            return widget::Space::new().width(Length::Fill).into();
        };

        let mut crumbs = widget::Row::new()
            .spacing(0)
            .align_y(cosmic::iced::Alignment::Center);
        let folders: Vec<&Path> = tab_state.current_dir.ancestors().collect();
        for (index, folder) in folders.iter().rev().enumerate() {
            if index > 0 {
                crumbs = crumbs.push(widget::icon::from_name("pan-end-symbolic").size(12));
            }
            let label = match folder.file_name() {
                Some(name) => name.to_string_lossy().into_owned(),
                None => folder.display().to_string(), // the root, "/"
            };
            crumbs = crumbs.push(
                widget::button::text(label)
                    .class(path_button_class(is_active))
                    .tooltip(folder.display().to_string())
                    .on_press(PaneMessage::Navigate(folder.to_path_buf())),
            );
        }

        let bar = widget::Row::new()
            .align_y(cosmic::iced::Alignment::Center)
            .push(
                widget::scrollable::horizontal(crumbs)
                    .width(Length::Fill),
            )
            .push(
                widget::button::icon(widget::icon::from_name("pencil-symbolic"))
                    .class(path_button_class(is_active))
                    .tooltip("Edit path (Ctrl+L)")
                    .on_press(PaneMessage::StartPathEdit),
            );
        // Folder buttons capture their own clicks, so only clicks on the
        // bar's empty space reach this and start editing.
        let bar = widget::mouse_area(bar).on_press(PaneMessage::StartPathEdit);
        // The active pane's path is highlighted like selected files (accent
        // background), so it's clear which pane the keyboard acts on.
        widget::container(bar)
            .width(Length::Fill)
            .class(cosmic::theme::Container::custom(move |theme| {
                if !is_active {
                    return widget::container::Style::default();
                }
                let cosmic = theme.cosmic();
                widget::container::Style {
                    icon_color: Some(cosmic.on_accent_color().into()),
                    text_color: Some(cosmic.on_accent_color().into()),
                    background: Some(cosmic::iced::Background::Color(
                        cosmic.accent_color().into(),
                    )),
                    border: cosmic::iced::Border {
                        radius: cosmic.radius_s().into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }))
            .into()
    }
}

/// Style for the path bar's buttons: no background of their own (so the
/// active pane's accent highlight shows through), text in the color that
/// reads on it, and a light tint on hover / press.
fn path_button_class(highlighted: bool) -> cosmic::theme::Button {
    fn style(theme: &cosmic::Theme, highlighted: bool, tint: f32) -> widget::button::Style {
        let cosmic = theme.cosmic();
        let foreground: cosmic::iced::Color = if highlighted {
            cosmic.on_accent_color()
        } else {
            cosmic.on_bg_color()
        }
        .into();
        let mut style = widget::button::Style::new();
        style.text_color = Some(foreground);
        style.icon_color = Some(foreground);
        style.border_radius = cosmic.radius_s().into();
        if tint > 0.0 {
            style.background = Some(cosmic::iced::Background::Color(cosmic::iced::Color {
                a: tint,
                ..foreground
            }));
        }
        style
    }
    cosmic::theme::Button::Custom {
        active: Box::new(move |_, theme| style(theme, highlighted, 0.0)),
        disabled: Box::new(move |theme| style(theme, highlighted, 0.0)),
        hovered: Box::new(move |_, theme| style(theme, highlighted, 0.15)),
        pressed: Box::new(move |_, theme| style(theme, highlighted, 0.3)),
    }
}

/// The listing's columns, left to right.
const LIST_COLUMNS: [Column; 3] = [Column::Name, Column::Size, Column::Modified];
const LIST_ICON_SIZE: u16 = 20;

/// Accent background for selected rows / cells (none otherwise).
fn selection_class(selected: bool) -> cosmic::theme::Container<'static> {
    cosmic::theme::Container::custom(move |theme| {
        if !selected {
            return widget::container::Style::default();
        }
        let cosmic = theme.cosmic();
        widget::container::Style {
            icon_color: Some(cosmic.on_accent_color().into()),
            text_color: Some(cosmic.on_accent_color().into()),
            background: Some(cosmic::iced::Background::Color(cosmic.accent_color().into())),
            border: cosmic::iced::Border {
                radius: cosmic.radius_s().into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
}

/// One line of text, cut with "…" when it doesn't fit its column.
fn one_line(text: String) -> widget::Text<'static, cosmic::Theme> {
    use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit, Wrapping};
    widget::text::body(text)
        .width(Length::Fill)
        .wrapping(Wrapping::WordOrGlyph)
        .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)))
}

/// The list view's column headers; clicking one sorts by it (again to
/// reverse), and the sort column shows an up/down arrow.
fn list_header(tab_state: &TabState) -> Element<'_, PaneMessage> {
    let (sort_column, ascending) = tab_state.sort();
    let mut header = widget::Row::new().padding([0, 8]);
    for column in LIST_COLUMNS {
        let mut button = widget::button::text(column.to_string())
            .class(path_button_class(false))
            .on_press(PaneMessage::SortBy(column));
        if column == sort_column {
            button = button.trailing_icon(widget::icon::from_name(if ascending {
                "pan-up-symbolic"
            } else {
                "pan-down-symbolic"
            }));
        }
        header = header.push(
            widget::container(button)
                .width(table::ItemCategory::width(&column))
                .clip(true),
        );
    }
    header.into()
}

/// List view: one row per entry with fixed columns; every cell stays on one
/// line and ends in "…" instead of running into the next column. Click
/// selects (Ctrl adds), double-click opens, right-click shows the menu.
fn list_view<'a>(
    pane: &'a PaneState,
    tab_state: &'a TabState,
    additive_select: bool,
    keybinds: &HashMap<KeyBind, Action>,
    can_paste: bool,
) -> Element<'a, PaneMessage> {
    use cosmic::widget::table::ItemInterface;

    let mut rows = widget::Column::new().width(Length::Fill);
    for entity in tab_state.entries.iter() {
        let Some(item) = tab_state.entries.item(entity) else {
            continue;
        };
        let mut row = widget::Row::new()
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center);
        for column in LIST_COLUMNS {
            let text = one_line(item.get_text(column).into_owned());
            let cell: Element<'a, PaneMessage> = if column == Column::Name {
                widget::Row::new()
                    .spacing(8)
                    .align_y(cosmic::iced::Alignment::Center)
                    .push(widget::icon(pane.item_icon(item)).size(LIST_ICON_SIZE))
                    .push(text)
                    .into()
            } else {
                text.into()
            };
            row = row.push(
                widget::container(cell)
                    .width(table::ItemCategory::width(&column))
                    .clip(true),
            );
        }
        let row = widget::container(row)
            .padding([4, 8])
            .width(Length::Fill)
            .class(selection_class(tab_state.entries.is_active(entity)));
        let area = widget::mouse_area(row)
            .on_press(PaneMessage::EntrySelected(entity, additive_select))
            .on_double_click(PaneMessage::EntryDoubleClicked(entity))
            .on_right_press(PaneMessage::EntryRightClicked(entity));
        rows = rows.push(widget::context_menu(area, item_menu(keybinds, item, can_paste)));
    }
    rows.into()
}

/// Width of one grid-view cell (icon + name), in pixels.
const GRID_CELL_WIDTH: f32 = 104.0;
const GRID_ICON_SIZE: u16 = 48;
/// Space between grid cells, both across and down.
const GRID_GAP: u16 = 8;

/// How many grid cells fit side by side in `width` pixels (at least one).
fn grid_columns(width: f32) -> usize {
    let gap = f32::from(GRID_GAP);
    (((width + gap) / (GRID_CELL_WIDTH + gap)).floor() as usize).max(1)
}

/// Grid view, like COSMIC Files: a large icon with the name below, cells
/// wrapping into rows to fill the pane. Same interactions as the list: click
/// selects (Ctrl adds), double-click opens, right-click shows the context menu.
fn grid_view<'a>(
    pane: &'a PaneState,
    tab_state: &'a TabState,
    width: f32,
    additive_select: bool,
    keybinds: &HashMap<KeyBind, Action>,
    can_paste: bool,
) -> Element<'a, PaneMessage> {
    use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit, Wrapping};

    let mut cells: Vec<Element<'a, PaneMessage>> = Vec::new();
    for entity in tab_state.entries.iter() {
        let Some(item) = tab_state.entries.item(entity) else {
            continue;
        };
        let selected = tab_state.entries.is_active(entity);

        let content = widget::Column::new()
            .spacing(4)
            .align_x(cosmic::iced::Alignment::Center)
            .width(Length::Fill)
            .push(widget::icon(pane.item_icon(item)).size(GRID_ICON_SIZE))
            .push(
                // Up to 3 lines, like COSMIC Files; long words break too
                // (with plain word wrapping a name without spaces spilled
                // over its neighbors), and "…" ends anything longer.
                widget::text::body(item.name().to_string())
                    .width(Length::Fill)
                    .align_x(cosmic::iced::alignment::Horizontal::Center)
                    .wrapping(Wrapping::WordOrGlyph)
                    .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(3))),
            );
        let cell = widget::container(content)
            .padding(6)
            .width(Length::Fixed(GRID_CELL_WIDTH))
            .clip(true)
            .class(selection_class(selected));
        let area = widget::mouse_area(cell)
            .on_press(PaneMessage::EntrySelected(entity, additive_select))
            .on_double_click(PaneMessage::EntryDoubleClicked(entity))
            .on_right_press(PaneMessage::EntryRightClicked(entity));
        cells.push(widget::context_menu(area, item_menu(keybinds, item, can_paste)).into());
    }

    // Plain rows of fixed-size cells: predictable spacing (a flex layout
    // spread wrapped rows far apart vertically).
    let columns = grid_columns(width);
    let mut grid = widget::Column::new().spacing(GRID_GAP).width(Length::Fill);
    let mut cells = cells.into_iter().peekable();
    while cells.peek().is_some() {
        let row = widget::Row::with_children(cells.by_ref().take(columns).collect::<Vec<_>>())
            .spacing(GRID_GAP)
            .align_y(cosmic::iced::Alignment::Start);
        grid = grid.push(row);
    }
    grid.into()
}

/// Turns what the user typed in the path bar into an absolute path:
/// `~` is the home folder, relative paths start at `current`, and `.`/`..`
/// are resolved (without touching the disk).
pub fn resolve_typed_path(input: &str, current: &Path, home: &Path) -> PathBuf {
    use std::path::Component;
    let input = input.trim();
    let raw = if input.is_empty() {
        current.to_path_buf()
    } else if input == "~" {
        home.to_path_buf()
    } else if let Some(rest) = input.strip_prefix("~/") {
        home.join(rest)
    } else {
        current.join(input) // `join` keeps `input` as-is when it's absolute
    };
    let mut resolved = PathBuf::new();
    for component in raw.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            other => resolved.push(other),
        }
    }
    resolved
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// "Up one level" arrow (left, then up). The icon theme has no such icon, so it ships
/// with the app; drawn as a symbolic icon so it follows the theme's colors.
fn parent_folder_icon() -> widget::icon::Handle {
    let mut handle = widget::icon::from_svg_bytes(
        &include_bytes!("../resources/icons/go-parent-folder-symbolic.svg")[..],
    );
    handle.symbolic = true;
    handle
}

/// Limits how many thumbnails are loaded or generated at the same time, so a
/// folder of photos doesn't start hundreds of thumbnailer processes at once.
fn thumbnail_workers() -> &'static tokio::sync::Semaphore {
    static WORKERS: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();
    WORKERS.get_or_init(|| tokio::sync::Semaphore::new(THUMBNAIL_WORKERS))
}

fn load_dir(pane: PaneId, tab: TabId, path: PathBuf) -> Task<Message> {
    cosmic::task::future(async move {
        let result = fs_ops::list_dir(&path).await.map_err(|err| err.to_string());
        Message::Pane(pane, PaneMessage::DirLoaded(tab, path, result))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_typed_paths() {
        let current = Path::new("/home/user/projects");
        let home = Path::new("/home/user");
        let resolve = |input| resolve_typed_path(input, current, home);

        assert_eq!(resolve("/etc"), PathBuf::from("/etc"));
        assert_eq!(resolve("  /etc/  "), PathBuf::from("/etc"));
        assert_eq!(resolve("~"), PathBuf::from("/home/user"));
        assert_eq!(resolve("~/Downloads"), PathBuf::from("/home/user/Downloads"));
        assert_eq!(resolve("app/src"), PathBuf::from("/home/user/projects/app/src"));
        assert_eq!(resolve(".."), PathBuf::from("/home/user"));
        assert_eq!(resolve("/usr/./lib/../share"), PathBuf::from("/usr/share"));
        assert_eq!(resolve("/.."), PathBuf::from("/"));
        assert_eq!(resolve(""), PathBuf::from("/home/user/projects"));
    }

    #[test]
    fn grid_fits_as_many_columns_as_the_width_allows() {
        assert_eq!(grid_columns(0.0), 1);
        assert_eq!(grid_columns(104.0), 1);
        assert_eq!(grid_columns(215.0), 1);
        assert_eq!(grid_columns(216.0), 2); // 104 + 8 + 104
        assert_eq!(grid_columns(1000.0), 9); // 9 * 104 + 8 * 8 = 1000 exactly
        assert_eq!(grid_columns(999.0), 8);
    }
}
