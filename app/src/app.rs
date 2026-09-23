use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use cosmic::app::context_drawer::{self, ContextDrawer};
use cosmic::app::{Core, Task};
use cosmic::iced::event::{listen_raw, Status};
use cosmic::iced::keyboard::key::Named;
use cosmic::iced::keyboard::{Event as KeyEvent, Key, Modifiers};
use cosmic::iced::{Alignment, Event, Length, Subscription};
use cosmic::widget;
use cosmic::widget::menu::key_bind::KeyBind;
use cosmic::{Application, Element};
use tokio_stream::{Stream, StreamExt};

use fs_ops::details::Details;
use fs_ops::favorites::Favorite;
use fs_ops::ops::{CancelHandle, ConflictHandle, ConflictResolution, OpEvent};
use fs_ops::session::Session;
use fs_ops::settings::{IconStyle, Settings, ViewMode};
use fs_ops::user_dirs::UserDir;
use fs_ops::EntryKind;

use crate::file_item::{format_modified, format_size, ListingOptions};
use crate::keybinds::{default_keybinds, Action};
use crate::launch::{self, open_with_default_app, AppEntry, OpenMode};
use crate::menu_bar::menu_bar;
use crate::operation::{OpKind, OperationState};
use crate::pane::{home_dir, PaneMessage, PaneState, SelectMode};
use crate::tab::tab_label;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaneId {
    Left,
    Right,
}

impl PaneId {
    fn other(self) -> Self {
        match self {
            PaneId::Left => PaneId::Right,
            PaneId::Right => PaneId::Left,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Pane(PaneId, PaneMessage),
    /// Fired by the bottom function-key button bar; identical to the matching keybind.
    Action(Action),
    Operation(OpEvent),
    Key(Modifiers, Key),
    /// Every way of closing the app (Exit button, Ctrl+Q/Alt+F4, the window's
    /// close button, the compositor closing the window) ends here.
    ExitNow,
    /// Escape that a focused widget (e.g. the quick filter box) consumed.
    EscapeCaptured,
    ModifiersChanged(Modifiers),
    ConfirmDelete,
    CancelDelete,
    CancelOperation,
    RenameInputChanged(String),
    ConfirmRename,
    CancelRename,
    RenameFinished(Result<PathBuf, String>),
    NewFolderInputChanged(String),
    ConfirmNewFolder,
    CancelNewFolder,
    NewFolderFinished(Result<PathBuf, String>),
    ResolveConflict(ConflictResolution),
    FavoriteNameChanged(String),
    AddFavorite,
    RemoveFavorite(usize),
    /// Moves a favorite one place up (`true`) or down in the list.
    MoveFavorite(usize, bool),
    GoToFavorite(usize),
    CompressNameChanged(String),
    ConfirmCompress,
    CancelCompress,
    /// Apps available for the paths the "Open With" dialog was opened for.
    OpenWithLoaded(Vec<PathBuf>, Vec<AppEntry>),
    LaunchApp(usize),
    CancelOpenWith,
    Launched,
    DetailsLoaded(Vec<PathBuf>, Result<Details, String>),
    CloseDetails,
    HideHiddenFilesToggled(bool),
    ThumbnailsToggled(bool),
    /// Index into [`LANGUAGES`]. Stored only; not applied yet.
    LanguageSelected(usize),
    /// Index into [`ICON_STYLES`].
    IconStyleSelected(usize),
    CloseDrawer,
}

/// Index of "Skip" in `App::conflict_buttons`: the safe default focus.
const CONFLICT_SKIP: usize = 2;

/// Languages offered in Settings, as (code, display name).
const LANGUAGES: [(&str, &str); 1] = [("en", "English")];

/// Icon styles offered in Settings, with their display names.
const ICON_STYLES: [(IconStyle, &str); 2] = [
    (IconStyle::Colorful, "Colorful"),
    (IconStyle::Monochrome, "Monochrome"),
];

/// What the side panel (context drawer) is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DrawerPage {
    Settings,
    Favorites,
}

/// Paths put aside by Cut/Copy, waiting for a Paste.
struct Clipboard {
    paths: Vec<PathBuf>,
    /// Move (rather than copy) on paste.
    cut: bool,
}

struct CompressState {
    sources: Vec<PathBuf>,
    dir: PathBuf,
    name: String,
}

struct OpenWithState {
    paths: Vec<PathBuf>,
    /// `None` while the installed applications are still loading.
    apps: Option<Vec<AppEntry>>,
}

struct DetailsState {
    paths: Vec<PathBuf>,
    /// `None` while sizes are still being calculated.
    result: Option<Result<Details, String>>,
}

struct RenameState {
    path: PathBuf,
    name: String,
}

struct NewFolderState {
    name: String,
}

pub struct App {
    core: Core,
    left: PaneState,
    right: PaneState,
    active_pane: PaneId,
    keybinds: HashMap<KeyBind, Action>,
    operation: Option<OperationState>,
    confirm_delete: Option<Vec<PathBuf>>,
    rename: Option<RenameState>,
    new_folder: Option<NewFolderState>,
    /// Set while `self.operation` has an `OpEvent::Conflict` awaiting an answer.
    pending_conflict: Option<(PathBuf, PathBuf)>,
    favorites: Vec<Favorite>,
    /// The Favorites panel's name field; empty means "use the folder's name".
    favorite_name: String,
    /// The Delete dialog's buttons, so the keyboard can move focus between them.
    confirm_delete_button: widget::Id,
    cancel_delete_button: widget::Id,
    /// The Rename / New Folder / Compress dialogs' text field and buttons
    /// (only one dialog shows at a time), for moving focus with Tab.
    dialog_input: widget::Id,
    dialog_confirm: widget::Id,
    dialog_cancel: widget::Id,
    /// Which of those has keyboard focus: 0 text field, 1 confirm, 2 cancel.
    dialog_focus: usize,
    /// The "File Already Exists" buttons in on-screen order: Skip All,
    /// Replace All, Skip, Replace; and which one has keyboard focus.
    conflict_buttons: [widget::Id; 4],
    conflict_focus: usize,
    /// Which Delete dialog button has keyboard focus (true = Delete).
    delete_focus_on_confirm: bool,
    /// The favorite highlighted for keyboard use (↑/↓ to move, Enter to open)
    /// while the Favorites panel is open.
    favorite_cursor: Option<usize>,
    clipboard: Option<Clipboard>,
    compress: Option<CompressState>,
    open_with: Option<OpenWithState>,
    details: Option<DetailsState>,
    settings: Settings,
    /// Display names from [`LANGUAGES`] and [`ICON_STYLES`], for the Settings dropdowns.
    language_names: Vec<&'static str>,
    icon_style_names: Vec<&'static str>,
    user_dirs: Arc<HashMap<PathBuf, UserDir>>,
    drawer: Option<DrawerPage>,
    /// The session as last written to disk, to skip saving when unchanged.
    saved_session: Session,
    modifiers: Modifiers,
}

impl App {
    fn pane(&self, id: PaneId) -> &PaneState {
        match id {
            PaneId::Left => &self.left,
            PaneId::Right => &self.right,
        }
    }

    fn pane_mut(&mut self, id: PaneId) -> &mut PaneState {
        match id {
            PaneId::Left => &mut self.left,
            PaneId::Right => &mut self.right,
        }
    }

    /// Handles one message. `update` wraps this so the session is saved
    /// after every message, whichever way this returns.
    fn handle_message(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Pane(id, PaneMessage::Action(action)) => {
                self.set_active_pane(id);
                self.handle_action(action)
            }
            Message::Pane(id, msg) => {
                // A listing finishing its load isn't the user picking that
                // pane (after an operation both panes reload in the background).
                if !matches!(msg, PaneMessage::DirLoaded(..)) {
                    self.set_active_pane(id);
                }
                self.pane_mut(id).update(msg)
            }
            Message::Action(action) => self.handle_action(action),
            Message::ExitNow => self.exit_now(),
            Message::EscapeCaptured => {
                // Text boxes swallow Escape; still let one press close the
                // dialog they're in (e.g. Rename), the path editor or the filter.
                if self.dialog_open() {
                    return self.on_escape();
                }
                if !self.pane_mut(self.active_pane).cancel_path_edit()
                    && self.pane(self.active_pane).filter_open()
                {
                    self.pane_mut(self.active_pane).close_filter();
                }
                Task::none()
            }
            Message::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers;
                Task::none()
            }
            Message::Key(modifiers, key) => {
                self.modifiers = modifiers;
                if key == Key::Named(Named::Escape) {
                    return self.on_escape();
                }
                if self.confirm_delete.is_some()
                    && !self.dialog_open_besides_delete()
                    && modifiers.is_empty()
                {
                    if let Some(task) = self.delete_dialog_key(&key) {
                        return task;
                    }
                }
                // Tab / → and Shift+Tab / ← move focus between the "File Already
                // Exists" buttons; Enter runs the focused one (the button itself
                // handles Enter).
                if self.pending_conflict.is_some() {
                    let forward = match (&key, modifiers) {
                        (Key::Named(Named::Tab), m) if m.is_empty() => Some(true),
                        (Key::Named(Named::Tab), m) if m == Modifiers::SHIFT => Some(false),
                        (Key::Named(Named::ArrowRight), m) if m.is_empty() => Some(true),
                        (Key::Named(Named::ArrowLeft), m) if m.is_empty() => Some(false),
                        _ => None,
                    };
                    if let Some(forward) = forward {
                        let count = self.conflict_buttons.len();
                        let step = if forward { 1 } else { count - 1 };
                        return self.focus_conflict_button(self.conflict_focus + step);
                    }
                }
                // Tab / Shift+Tab move focus around the text dialogs (libcosmic's
                // own keyboard navigation is off, see `init`).
                if key == Key::Named(Named::Tab)
                    && (modifiers.is_empty() || modifiers == Modifiers::SHIFT)
                    && self.text_dialog_showing()
                {
                    let step = if modifiers.shift() { 2 } else { 1 };
                    return self.focus_dialog_field(self.dialog_focus + step);
                }
                if self.dialog_open() {
                    // A modal dialog is open; let its own controls (and `on_escape`)
                    // handle input instead of firing global shortcuts underneath it.
                    return Task::none();
                }
                if let Some(task) = self.favorites_key(modifiers, &key) {
                    return task;
                }
                let action = self.keybinds.iter().find_map(|(bind, action)| {
                    bind.matches(modifiers, &key, None).then_some(*action)
                });
                match action {
                    Some(action) => self.handle_action(action),
                    None => Task::none(),
                }
            }
            Message::Operation(event) => match event {
                OpEvent::Progress(progress) => {
                    if let Some(op) = &mut self.operation {
                        op.current_file = Some(progress.current_file);
                        op.files_done = progress.files_done;
                        op.files_total = progress.files_total;
                        op.bytes_done = progress.bytes_done;
                        op.bytes_total = progress.bytes_total;
                    }
                    Task::none()
                }
                OpEvent::Conflict { src, dest } => {
                    self.pending_conflict = Some((src, dest));
                    // Start on Skip: Enter then never overwrites anything.
                    self.focus_conflict_button(CONFLICT_SKIP)
                }
                OpEvent::Done => {
                    self.operation = None;
                    self.refresh_after_operation()
                }
                OpEvent::Cancelled => {
                    self.operation = None;
                    self.refresh_after_operation()
                }
                OpEvent::Error(err) => {
                    if let Some(op) = &mut self.operation {
                        op.error = Some(err);
                    }
                    Task::none()
                }
            },
            Message::ConfirmDelete => {
                let Some(sources) = self.confirm_delete.take() else {
                    return Task::none();
                };
                if self.operation.is_some() {
                    return Task::none();
                }
                self.spawn_operation(OpKind::Delete, |cancel, _| {
                    fs_ops::ops::delete_to_trash(sources, cancel)
                })
            }
            Message::CancelDelete => {
                self.confirm_delete = None;
                Task::none()
            }
            Message::CancelOperation => {
                self.pending_conflict = None;
                if let Some(op) = self.operation.take() {
                    op.cancel.cancel();
                    // Unblock the background task if it's mid-`conflict.ask()` -
                    // otherwise it would sit there forever, never seeing the
                    // cancellation, since it isn't polling `cancel` while waiting.
                    op.conflict.respond(ConflictResolution::SkipAll);
                }
                Task::none()
            }
            Message::RenameInputChanged(value) => {
                if let Some(rename) = &mut self.rename {
                    rename.name = value;
                }
                Task::none()
            }
            Message::ConfirmRename => {
                let Some(rename) = self.rename.take() else {
                    return Task::none();
                };
                cosmic::task::future(async move {
                    let result = fs_ops::ops::rename(rename.path, rename.name).await;
                    Message::RenameFinished(result)
                })
            }
            Message::CancelRename => {
                self.rename = None;
                Task::none()
            }
            Message::RenameFinished(result) => {
                if let Err(err) = result {
                    eprintln!("rename failed: {err}");
                }
                self.refresh_after_operation()
            }
            Message::NewFolderInputChanged(value) => {
                if let Some(new_folder) = &mut self.new_folder {
                    new_folder.name = value;
                }
                Task::none()
            }
            Message::ConfirmNewFolder => {
                let Some(new_folder) = self.new_folder.take() else {
                    return Task::none();
                };
                let parent = self.pane(self.active_pane).current_dir();
                cosmic::task::future(async move {
                    let result = fs_ops::ops::create_dir(parent, new_folder.name).await;
                    Message::NewFolderFinished(result)
                })
            }
            Message::CancelNewFolder => {
                self.new_folder = None;
                Task::none()
            }
            Message::NewFolderFinished(result) => {
                if let Err(err) = result {
                    eprintln!("failed to create folder: {err}");
                }
                self.refresh_after_operation()
            }
            Message::ResolveConflict(resolution) => {
                self.pending_conflict = None;
                if let Some(op) = &self.operation {
                    op.conflict.respond(resolution);
                }
                Task::none()
            }
            Message::FavoriteNameChanged(value) => {
                self.favorite_name = value;
                Task::none()
            }
            Message::AddFavorite => {
                let path = self.pane(self.active_pane).current_dir();
                let name = match self.favorite_name.trim() {
                    "" => tab_label(&path),
                    name => name.to_string(),
                };
                self.favorites.push(Favorite { name, path });
                self.persist_favorites();
                self.favorite_name.clear();
                self.favorite_cursor = Some(self.favorites.len() - 1);
                Task::none()
            }
            Message::RemoveFavorite(index) => {
                if index < self.favorites.len() {
                    self.favorites.remove(index);
                    self.persist_favorites();
                    // Keep the highlight on a row that still exists.
                    let last = self.favorites.len().checked_sub(1);
                    self.favorite_cursor = self.favorite_cursor.zip(last).map(|(c, l)| c.min(l));
                }
                Task::none()
            }
            Message::MoveFavorite(index, up) => {
                let target = if up {
                    index.checked_sub(1)
                } else {
                    Some(index + 1)
                };
                if let Some(target) = target.filter(|t| *t < self.favorites.len()) {
                    self.favorites.swap(index, target);
                    self.persist_favorites();
                    // The highlight follows the moved favorite.
                    if self.favorite_cursor == Some(index) {
                        self.favorite_cursor = Some(target);
                    }
                }
                Task::none()
            }
            Message::GoToFavorite(index) => {
                let Some(favorite) = self.favorites.get(index) else {
                    return Task::none();
                };
                let path = favorite.path.clone();
                self.close_drawer();
                self.pane_mut(self.active_pane).update(PaneMessage::Navigate(path))
            }
            Message::CompressNameChanged(value) => {
                if let Some(compress) = &mut self.compress {
                    compress.name = value;
                }
                Task::none()
            }
            Message::ConfirmCompress => {
                if self.operation.is_some() {
                    // Keep the dialog open; the user can retry once it finishes.
                    return Task::none();
                }
                let Some(compress) = self.compress.take() else {
                    return Task::none();
                };
                let mut name = compress.name.trim().to_string();
                if name.is_empty() {
                    self.compress = Some(compress);
                    return Task::none();
                }
                if !name.to_lowercase().ends_with(".zip") {
                    name.push_str(".zip");
                }
                let archive = compress.dir.join(name);
                let sources = compress.sources;
                self.spawn_operation(OpKind::Compress, |cancel, _| {
                    fs_ops::ops::compress_zip(sources, archive, cancel)
                })
            }
            Message::CancelCompress => {
                self.compress = None;
                Task::none()
            }
            Message::OpenWithLoaded(paths, apps) => {
                if let Some(open_with) = &mut self.open_with {
                    // Ignore a slow load for a dialog that's since been reopened.
                    if open_with.paths == paths {
                        open_with.apps = Some(apps);
                    }
                }
                Task::none()
            }
            Message::LaunchApp(index) => {
                let Some(open_with) = self.open_with.take() else {
                    return Task::none();
                };
                let Some(app) = open_with.apps.and_then(|apps| apps.into_iter().nth(index)) else {
                    return Task::none();
                };
                let paths = open_with.paths;
                cosmic::task::future(async move {
                    launch::launch(app, paths).await;
                    Message::Launched
                })
            }
            Message::CancelOpenWith => {
                self.open_with = None;
                Task::none()
            }
            Message::Launched => Task::none(),
            Message::DetailsLoaded(paths, result) => {
                if let Some(details) = &mut self.details {
                    if details.paths == paths {
                        details.result = Some(result);
                    }
                }
                Task::none()
            }
            Message::CloseDetails => {
                self.details = None;
                Task::none()
            }
            Message::HideHiddenFilesToggled(hide) => self.set_hide_hidden_files(hide),
            Message::ThumbnailsToggled(show) => {
                self.settings.show_thumbnails = show;
                self.apply_settings()
            }
            Message::LanguageSelected(index) => {
                // Only English exists for now; the choice is saved but not applied.
                if let Some((code, _)) = LANGUAGES.get(index) {
                    self.settings.language = code.to_string();
                    self.persist_settings();
                }
                Task::none()
            }
            Message::IconStyleSelected(index) => {
                if let Some((style, _)) = ICON_STYLES.get(index) {
                    self.settings.icon_style = *style;
                    return self.apply_settings();
                }
                Task::none()
            }
            Message::CloseDrawer => {
                self.close_drawer();
                Task::none()
            }
        }
    }

    /// The panes' current state, as saved for the next launch.
    fn session(&self) -> Session {
        Session {
            left: self.left.session(),
            right: self.right.session(),
            right_active: self.active_pane == PaneId::Right,
        }
    }

    /// Saves the session if it changed since the last save. Called after
    /// every message, so it survives the app being killed, not just a clean quit.
    fn save_session_if_changed(&mut self) {
        let session = self.session();
        if session == self.saved_session {
            return;
        }
        if let Err(err) = fs_ops::session::save(&session) {
            eprintln!("failed to save session: {err}");
        }
        self.saved_session = session;
    }

    /// Saves the session and ends the process right away.
    ///
    /// libcosmic's normal shutdown tears down the Wayland connection while a
    /// background thread (the Wayland event loop / clipboard) may still use
    /// it, which intermittently crashed with "Bad file descriptor" and a
    /// segfault in libwayland-client. Everything worth keeping (session,
    /// settings, favorites) is already on disk, so skipping that teardown
    /// loses nothing. The compositor cleans up the window when we exit.
    fn exit_now(&mut self) -> ! {
        self.save_session_if_changed();
        std::process::exit(0)
    }

    /// Makes `id` the active pane. The other pane's selection is hidden (and
    /// remembered), and `id`'s own hidden selection comes back.
    fn set_active_pane(&mut self, id: PaneId) {
        if id == self.active_pane {
            return;
        }
        self.pane_mut(self.active_pane).deactivate();
        self.active_pane = id;
        self.pane_mut(id).activate();
    }

    fn refresh_after_operation(&mut self) -> Task<Message> {
        Task::batch(vec![self.left.reload(), self.right.reload()])
    }

    fn handle_action(&mut self, action: Action) -> Task<Message> {
        match action {
            Action::SwitchPane => {
                self.set_active_pane(self.active_pane.other());
                Task::none()
            }
            Action::NextTab => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::CycleTab(true)),
            Action::PreviousTab => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::CycleTab(false)),
            Action::ListView | Action::GridView => {
                // Only the active pane's active tab changes; the session saves it.
                let view_mode = if action == Action::GridView {
                    ViewMode::Grid
                } else {
                    ViewMode::List
                };
                self.pane_mut(self.active_pane).set_view_mode(view_mode);
                Task::none()
            }
            Action::EditPath => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::StartPathEdit),
            Action::View | Action::Edit => {
                let paths = self.pane(self.active_pane).selected_paths();
                if paths.is_empty() {
                    return Task::none();
                }
                let mode = if action == Action::View {
                    OpenMode::View
                } else {
                    OpenMode::Edit
                };
                cosmic::task::future(async move {
                    launch::view_or_edit(paths, mode).await;
                    Message::Launched
                })
            }
            Action::Terminal => {
                launch::open_terminal(&self.pane(self.active_pane).current_dir());
                Task::none()
            }
            Action::Copy => self.start_operation(OpKind::Copy),
            Action::Move => self.start_operation(OpKind::Move),
            Action::Delete => {
                let sources = self.pane(self.active_pane).selected_paths();
                if sources.is_empty() {
                    return Task::none();
                }
                self.confirm_delete = Some(sources);
                // Focus "Delete" so Enter confirms (Escape still cancels).
                self.focus_delete_button(true)
            }
            Action::Rename => {
                let mut sources = self.pane(self.active_pane).selected_paths();
                if sources.len() != 1 {
                    // Renaming multiple items at once needs a batch-rename tool; out of v1 scope.
                    return Task::none();
                }
                let path = sources.pop().unwrap();
                let name = tab_label(&path);
                self.rename = Some(RenameState { path, name });
                self.focus_dialog_field(0)
            }
            Action::NewFolder => {
                self.new_folder = Some(NewFolderState {
                    name: "New folder".to_string(),
                });
                self.focus_dialog_field(0)
            }
            Action::NewTab => self.pane_mut(self.active_pane).update(PaneMessage::NewTab),
            Action::CloseTab => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::CloseActiveTab),
            Action::GoBack => self.pane_mut(self.active_pane).update(PaneMessage::GoBack),
            Action::GoForward => self.pane_mut(self.active_pane).update(PaneMessage::GoForward),
            Action::GoUp => self.pane_mut(self.active_pane).update(PaneMessage::GoUp),
            Action::SelectNext => self.pane_mut(self.active_pane).update(PaneMessage::SelectNext),
            Action::SelectFirst
            | Action::SelectLast
            | Action::ExtendSelectionToFirst
            | Action::ExtendSelectionToLast => {
                let last = matches!(action, Action::SelectLast | Action::ExtendSelectionToLast);
                let extend = matches!(
                    action,
                    Action::ExtendSelectionToFirst | Action::ExtendSelectionToLast
                );
                self.pane_mut(self.active_pane)
                    .update(PaneMessage::SelectEdge { last, extend })
            }
            Action::ExtendSelectionDown => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::ExtendSelection(true)),
            Action::ExtendSelectionUp => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::ExtendSelection(false)),
            Action::SelectPrevious => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::SelectPrevious),
            Action::QuickFilter => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::OpenFilter),
            Action::EnterSelectedFolder => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::EnterSelectedFolder),
            Action::Favorites => {
                self.toggle_drawer(DrawerPage::Favorites);
                Task::none()
            }
            Action::Open => {
                if let Some(dir) = self.pane(self.active_pane).selected_single_dir() {
                    return self
                        .pane_mut(self.active_pane)
                        .update(PaneMessage::Navigate(dir));
                }
                // Folders in a multi-selection are skipped: we can't navigate
                // into several at once, and handing them to `xdg-open` would
                // open them in a different file manager.
                for path in self.pane(self.active_pane).selected_paths() {
                    if !path.is_dir() {
                        open_with_default_app(&path);
                    }
                }
                Task::none()
            }
            Action::OpenWith => {
                let paths = self.pane(self.active_pane).selected_paths();
                let Some(first) = paths.first().cloned() else {
                    return Task::none();
                };
                self.open_with = Some(OpenWithState {
                    paths: paths.clone(),
                    apps: None,
                });
                cosmic::task::future(async move {
                    let apps = tokio::task::spawn_blocking(move || {
                        let mime = fs_ops::mime::mime_type(&first);
                        launch::load_apps(mime.as_deref())
                    })
                    .await
                    .unwrap_or_default();
                    Message::OpenWithLoaded(paths, apps)
                })
            }
            Action::Cut | Action::CopyToClipboard => {
                let paths = self.pane(self.active_pane).selected_paths();
                if !paths.is_empty() {
                    self.clipboard = Some(Clipboard {
                        paths,
                        cut: action == Action::Cut,
                    });
                }
                Task::none()
            }
            Action::Paste => self.paste(),
            Action::Compress => {
                let sources = self.pane(self.active_pane).selected_paths();
                if sources.is_empty() {
                    return Task::none();
                }
                let dir = self.pane(self.active_pane).current_dir();
                let name = default_archive_name(&sources, &dir);
                self.compress = Some(CompressState { sources, dir, name });
                self.focus_dialog_field(0)
            }
            Action::ShowDetails => {
                let paths = self.pane(self.active_pane).selected_paths();
                if paths.is_empty() {
                    return Task::none();
                }
                self.details = Some(DetailsState {
                    paths: paths.clone(),
                    result: None,
                });
                cosmic::task::future(async move {
                    let result = fs_ops::details::details(paths.clone()).await;
                    Message::DetailsLoaded(paths, result)
                })
            }
            Action::SelectAll => self.pane_mut(self.active_pane).update(PaneMessage::SelectAll),
            Action::Settings => {
                self.toggle_drawer(DrawerPage::Settings);
                Task::none()
            }
            Action::ToggleHiddenFiles => {
                self.set_hide_hidden_files(!self.settings.hide_hidden_files)
            }
            Action::Quit => self.exit_now(),
        }
    }

    fn listing_options(&self) -> ListingOptions {
        ListingOptions {
            hide_hidden: self.settings.hide_hidden_files,
            icon_style: self.settings.icon_style,
            user_dirs: self.user_dirs.clone(),
            show_thumbnails: self.settings.show_thumbnails,
        }
    }

    /// Saves the settings and re-applies them to both panes' listings.
    /// Saves the settings and re-applies them to both panes' listings
    /// (returning any thumbnail loading that newly needs to start).
    fn apply_settings(&mut self) -> Task<Message> {
        let options = self.listing_options();
        let left = self.left.set_options(options.clone());
        let right = self.right.set_options(options);
        self.persist_settings();
        Task::batch([left, right])
    }

    fn set_hide_hidden_files(&mut self, hide: bool) -> Task<Message> {
        self.settings.hide_hidden_files = hide;
        self.apply_settings()
    }

    fn persist_settings(&self) {
        if let Err(err) = fs_ops::settings::save(&self.settings) {
            eprintln!("failed to save settings: {err}");
        }
    }

    /// Opens the side panel on `page`, or closes it if it's already showing `page`.
    fn toggle_drawer(&mut self, page: DrawerPage) {
        if self.drawer == Some(page) {
            self.close_drawer();
        } else {
            self.drawer = Some(page);
            self.core.set_show_context(true);
            if page == DrawerPage::Favorites {
                // Start on the current folder's favorite, if it has one.
                let current = self.pane(self.active_pane).current_dir();
                self.favorite_cursor = self
                    .favorites
                    .iter()
                    .position(|favorite| favorite.path == current)
                    .or((!self.favorites.is_empty()).then_some(0));
            }
        }
    }

    /// While the Favorites panel is open, ↑/↓ move its highlight and Enter
    /// opens the highlighted favorite. Returns `None` for any other key, so it
    /// falls through to the normal shortcuts.
    fn favorites_key(&mut self, modifiers: Modifiers, key: &Key) -> Option<Task<Message>> {
        if self.drawer != Some(DrawerPage::Favorites) {
            return None;
        }
        // Ctrl+↑/↓ reorders the highlighted favorite.
        if modifiers == Modifiers::CTRL {
            let up = match key {
                Key::Named(Named::ArrowUp) => true,
                Key::Named(Named::ArrowDown) => false,
                _ => return None,
            };
            let index = self.favorite_cursor?;
            return Some(self.update(Message::MoveFavorite(index, up)));
        }
        if !modifiers.is_empty() {
            return None;
        }
        let last = self.favorites.len().checked_sub(1);
        match key {
            Key::Named(Named::ArrowDown) => {
                self.favorite_cursor = match (self.favorite_cursor, last) {
                    (_, None) => None,
                    (Some(cursor), Some(last)) => Some((cursor + 1).min(last)),
                    (None, Some(_)) => Some(0),
                };
                Some(Task::none())
            }
            Key::Named(Named::ArrowUp) => {
                self.favorite_cursor = match (self.favorite_cursor, last) {
                    (_, None) => None,
                    (Some(cursor), Some(_)) => Some(cursor.saturating_sub(1)),
                    (None, Some(_)) => Some(0),
                };
                Some(Task::none())
            }
            Key::Named(Named::Enter) => Some(match self.favorite_cursor {
                Some(index) => self.update(Message::GoToFavorite(index)),
                None => Task::none(),
            }),
            _ => None,
        }
    }

    fn close_drawer(&mut self) {
        self.drawer = None;
        self.core.set_show_context(false);
    }

    fn settings_page(&self) -> Element<'_, Message> {
        let selected_language = LANGUAGES
            .iter()
            .position(|(code, _)| *code == self.settings.language)
            .or(Some(0));
        let selected_icon_style = ICON_STYLES
            .iter()
            .position(|(style, _)| *style == self.settings.icon_style);

        let general = widget::settings::section()
            .title("General")
            .add(
                widget::settings::item::builder("Hide hidden files")
                    .description("Files and folders whose name starts with a dot")
                    .checkbox(self.settings.hide_hidden_files, Message::HideHiddenFilesToggled),
            )
            .add(
                widget::settings::item::builder("Show thumbnails")
                    .description("Previews of images, PDFs, videos and fonts")
                    .checkbox(self.settings.show_thumbnails, Message::ThumbnailsToggled),
            )
            .add(widget::settings::item(
                "Language",
                widget::dropdown(
                    self.language_names.as_slice(),
                    selected_language,
                    Message::LanguageSelected,
                ),
            ));

        let theme = widget::settings::section().title("Theme").add(
            widget::settings::item::builder("Icon style")
                .description("Colorful matches the COSMIC Files app")
                .control(widget::dropdown(
                    self.icon_style_names.as_slice(),
                    selected_icon_style,
                    Message::IconStyleSelected,
                )),
        );

        widget::settings::view_column(vec![general.into(), theme.into()]).into()
    }

    fn favorites_page(&self) -> Element<'_, Message> {
        let mut saved = widget::settings::section().title("Saved folders");
        if self.favorites.is_empty() {
            saved = saved.add(widget::text("No favorites saved yet."));
        }
        for (index, favorite) in self.favorites.iter().enumerate() {
            let is_first = index == 0;
            let is_last = index + 1 == self.favorites.len();
            let buttons = widget::Row::new()
                .spacing(4)
                .align_y(Alignment::Center)
                .push(
                    widget::button::icon(widget::icon::from_name("go-up-symbolic"))
                        .tooltip("Move up (Ctrl+↑)")
                        .on_press_maybe((!is_first).then_some(Message::MoveFavorite(index, true))),
                )
                .push(
                    widget::button::icon(widget::icon::from_name("go-down-symbolic"))
                        .tooltip("Move down (Ctrl+↓)")
                        .on_press_maybe((!is_last).then_some(Message::MoveFavorite(index, false))),
                )
                .push(widget::button::standard("Open").on_press(Message::GoToFavorite(index)))
                .push(
                    widget::button::icon(widget::icon::from_name("user-trash-symbolic"))
                        .on_press(Message::RemoveFavorite(index)),
                );
            let row = widget::settings::item::builder(favorite.name.clone())
                .description(favorite.path.display().to_string())
                .control(buttons);
            let highlighted = self.favorite_cursor == Some(index);
            saved = saved.add(
                widget::container(row)
                    .padding([4, 8])
                    .class(cosmic::theme::Container::custom(move |theme| {
                        highlight_style(theme, highlighted)
                    })),
            );
        }

        let current = self.pane(self.active_pane).current_dir();
        let add = widget::settings::section().title("Add current folder").add(
            widget::Column::new()
                .spacing(8)
                .push(widget::text::caption(current.display().to_string()))
                .push(
                    widget::Row::new()
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(
                            widget::text_input(tab_label(&current), self.favorite_name.as_str())
                                .on_input(Message::FavoriteNameChanged)
                                .on_submit(|_| Message::AddFavorite)
                                .width(Length::Fill),
                        )
                        .push(widget::button::suggested("Add").on_press(Message::AddFavorite)),
                ),
        );

        widget::settings::view_column(vec![saved.into(), add.into()]).into()
    }

    /// Copies (or, after a Cut, moves) the clipboard's paths into the active
    /// pane's current folder.
    fn paste(&mut self) -> Task<Message> {
        if self.operation.is_some() {
            return Task::none();
        }
        let Some(clipboard) = &self.clipboard else {
            return Task::none();
        };
        let dest = self.pane(self.active_pane).current_dir();
        if !clipboard.cut {
            let sources = clipboard.paths.clone();
            return self.spawn_operation(OpKind::Copy, |cancel, conflict| {
                fs_ops::ops::copy(sources, dest, cancel, conflict)
            });
        }
        // Moving something into the folder it's already in is a no-op.
        let sources: Vec<PathBuf> = clipboard
            .paths
            .iter()
            .filter(|path| path.parent() != Some(dest.as_path()))
            .cloned()
            .collect();
        if sources.is_empty() {
            return Task::none();
        }
        // Cut items only move once; they're gone from their old place after.
        self.clipboard = None;
        self.spawn_operation(OpKind::Move, |cancel, conflict| {
            fs_ops::ops::move_paths(sources, dest, cancel, conflict)
        })
    }

    /// Focuses button `index` of the "File Already Exists" dialog.
    fn focus_conflict_button(&mut self, index: usize) -> Task<Message> {
        self.conflict_focus = index % self.conflict_buttons.len();
        widget::button::focus(self.conflict_buttons[self.conflict_focus].clone())
    }

    /// Whether a dialog with a text field (Rename, New Folder, Compress) is
    /// the one showing.
    fn text_dialog_showing(&self) -> bool {
        self.pending_conflict.is_none()
            && (self.rename.is_some() || self.new_folder.is_some() || self.compress.is_some())
    }

    /// Focuses the text dialog's text field (0, with its text selected),
    /// confirm button (1) or Cancel (2).
    fn focus_dialog_field(&mut self, index: usize) -> Task<Message> {
        self.dialog_focus = index % 3;
        match self.dialog_focus {
            0 => Task::batch([
                widget::text_input::focus(self.dialog_input.clone()),
                widget::text_input::select_all(self.dialog_input.clone()),
            ]),
            1 => widget::button::focus(self.dialog_confirm.clone()),
            _ => widget::button::focus(self.dialog_cancel.clone()),
        }
    }

    /// Moves keyboard focus to the Delete dialog's Delete (`true`) or Cancel button.
    fn focus_delete_button(&mut self, confirm: bool) -> Task<Message> {
        self.delete_focus_on_confirm = confirm;
        let id = if confirm {
            &self.confirm_delete_button
        } else {
            &self.cancel_delete_button
        };
        widget::button::focus(id.clone())
    }

    /// Keyboard handling for the Delete dialog: ← focuses Cancel (the left
    /// button), → focuses Delete, Tab switches, and Enter runs the focused one.
    fn delete_dialog_key(&mut self, key: &Key) -> Option<Task<Message>> {
        match key {
            Key::Named(Named::ArrowLeft) => Some(self.focus_delete_button(false)),
            Key::Named(Named::ArrowRight) => Some(self.focus_delete_button(true)),
            Key::Named(Named::Tab) => Some(self.focus_delete_button(!self.delete_focus_on_confirm)),
            // A focused button consumes Enter itself, so this only runs if focus
            // was lost (e.g. a click elsewhere in the dialog) - never twice.
            Key::Named(Named::Enter) => Some(if self.delete_focus_on_confirm {
                self.update(Message::ConfirmDelete)
            } else {
                self.update(Message::CancelDelete)
            }),
            _ => None,
        }
    }

    /// True while a dialog other than the Delete confirmation is showing
    /// (e.g. a file conflict raised by an operation that's still running).
    fn dialog_open_besides_delete(&self) -> bool {
        self.rename.is_some()
            || self.new_folder.is_some()
            || self.pending_conflict.is_some()
            || self.compress.is_some()
            || self.open_with.is_some()
            || self.details.is_some()
    }

    /// True while any modal dialog is showing.
    fn dialog_open(&self) -> bool {
        self.confirm_delete.is_some() || self.dialog_open_besides_delete()
    }

    fn persist_favorites(&self) {
        if let Err(err) = fs_ops::favorites::save(&self.favorites) {
            eprintln!("failed to save favorites: {err}");
        }
    }

    /// Starts a copy or move of the active pane's selection into the other
    /// pane's current directory. No-ops if an operation is already running
    /// (v1 runs at most one at a time) or nothing is selected.
    fn start_operation(&mut self, kind: OpKind) -> Task<Message> {
        if self.operation.is_some() {
            return Task::none();
        }
        let sources = self.pane(self.active_pane).selected_paths();
        if sources.is_empty() {
            return Task::none();
        }
        let dest = self.pane(self.active_pane.other()).current_dir();
        match kind {
            OpKind::Copy => self.spawn_operation(kind, |cancel, conflict| {
                fs_ops::ops::copy(sources, dest, cancel, conflict)
            }),
            OpKind::Move => self.spawn_operation(kind, |cancel, conflict| {
                fs_ops::ops::move_paths(sources, dest, cancel, conflict)
            }),
            OpKind::Delete | OpKind::Compress => Task::none(),
        }
    }

    /// Tracks a new background operation and forwards its events to `update`.
    fn spawn_operation<S>(
        &mut self,
        kind: OpKind,
        start: impl FnOnce(CancelHandle, ConflictHandle) -> S,
    ) -> Task<Message>
    where
        S: Stream<Item = OpEvent> + Send + 'static,
    {
        let cancel = CancelHandle::new();
        let conflict = ConflictHandle::new();
        self.operation = Some(OperationState::new(kind, cancel.clone(), conflict.clone()));
        cosmic::task::stream(start(cancel, conflict).map(Message::Operation))
    }
}

impl Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "dev.gonzalo.CosmicCommander";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, _flags: Self::Flags) -> (Self, Task<Message>) {
        // libcosmic's built-in keyboard navigation moves widget focus on Tab,
        // which fights our Tab = "switch pane" (and a focused widget then
        // swallows later Tabs). Turn it off; Escape is handled in `Message::Key`.
        core.set_keyboard_nav(false);
        let home = home_dir();
        let settings = fs_ops::settings::load();
        let user_dirs = Arc::new(fs_ops::user_dirs::load());
        let options = ListingOptions {
            hide_hidden: settings.hide_hidden_files,
            icon_style: settings.icon_style,
            user_dirs: user_dirs.clone(),
            show_thumbnails: settings.show_thumbnails,
        };
        // Reopen the panes as they were when the app was last used.
        let session = fs_ops::session::load();
        let (left_dirs, left_active) = session.left.restore(&home);
        let (right_dirs, right_active) = session.right.restore(&home);
        let (left, left_task) = PaneState::new(PaneId::Left, left_dirs, left_active, options.clone());
        let (right, right_task) =
            PaneState::new(PaneId::Right, right_dirs, right_active, options);
        let app = App {
            core,
            left,
            right,
            active_pane: if session.right_active {
                PaneId::Right
            } else {
                PaneId::Left
            },
            keybinds: default_keybinds(),
            operation: None,
            confirm_delete: None,
            rename: None,
            new_folder: None,
            pending_conflict: None,
            favorites: fs_ops::favorites::load(),
            favorite_name: String::new(),
            favorite_cursor: None,
            confirm_delete_button: widget::Id::unique(),
            cancel_delete_button: widget::Id::unique(),
            dialog_input: widget::Id::unique(),
            dialog_confirm: widget::Id::unique(),
            dialog_cancel: widget::Id::unique(),
            dialog_focus: 0,
            conflict_buttons: std::array::from_fn(|_| widget::Id::unique()),
            conflict_focus: CONFLICT_SKIP,
            delete_focus_on_confirm: true,
            clipboard: None,
            compress: None,
            open_with: None,
            details: None,
            settings,
            language_names: LANGUAGES.iter().map(|(_, name)| *name).collect(),
            icon_style_names: ICON_STYLES.iter().map(|(_, name)| *name).collect(),
            user_dirs,
            drawer: None,
            saved_session: session,
            modifiers: Modifiers::empty(),
        };
        (app, Task::batch(vec![left_task, right_task]))
    }

    fn update(&mut self, message: Self::Message) -> Task<Message> {
        let task = self.handle_message(message);
        self.save_session_if_changed();
        task
    }

    fn on_app_exit(&mut self) -> Option<Message> {
        Some(Message::ExitNow)
    }

    fn on_close_requested(&self, id: cosmic::iced::window::Id) -> Option<Message> {
        (self.core.main_window_id() == Some(id)).then_some(Message::ExitNow)
    }

    fn header_start(&self) -> Vec<Element<'_, Message>> {
        vec![menu_bar(
            &self.keybinds,
            self.clipboard.is_some(),
            self.pane(self.active_pane).view_mode(),
        )]
    }

    fn context_drawer(&self) -> Option<ContextDrawer<'_, Message>> {
        let drawer = match self.drawer? {
            DrawerPage::Settings => {
                context_drawer::context_drawer(self.settings_page(), Message::CloseDrawer)
                    .title("Settings")
            }
            DrawerPage::Favorites => {
                context_drawer::context_drawer(self.favorites_page(), Message::CloseDrawer)
                    .title("Favorites")
            }
        };
        Some(drawer)
    }

    fn on_escape(&mut self) -> Task<Message> {
        if self.rename.take().is_some() {
            return Task::none();
        }
        if self.confirm_delete.take().is_some() {
            return Task::none();
        }
        if self.new_folder.take().is_some() {
            return Task::none();
        }
        if self.compress.take().is_some()
            || self.open_with.take().is_some()
            || self.details.take().is_some()
        {
            return Task::none();
        }
        if self.pending_conflict.take().is_some() {
            // Skip is the safe default: unlike Cancel, it can't destroy data.
            if let Some(op) = &self.operation {
                op.conflict.respond(ConflictResolution::Skip);
            }
            return Task::none();
        }
        if self.pane_mut(self.active_pane).cancel_path_edit() {
            return Task::none();
        }
        if self.pane(self.active_pane).filter_open() {
            self.pane_mut(self.active_pane).close_filter();
            return Task::none();
        }
        if let Some(op) = self.operation.take() {
            op.cancel.cancel();
            return Task::none();
        }
        if self.drawer.is_some() {
            self.close_drawer();
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        key_subscription()
    }

    fn dialog(&self) -> Option<Element<'_, Message>> {
        if let Some((src, dest)) = &self.pending_conflict {
            let body = format!(
                "\"{}\" already exists in the destination. Replace it with \"{}\"?",
                tab_label(dest),
                tab_label(src)
            );
            let [skip_all_id, replace_all_id, skip_id, replace_id] = self.conflict_buttons.clone();
            let all_buttons = widget::Row::new()
                .spacing(8)
                .push(
                    widget::button::standard("Skip All")
                        .id(skip_all_id)
                        .on_press(Message::ResolveConflict(ConflictResolution::SkipAll)),
                )
                .push(
                    widget::button::standard("Replace All")
                        .id(replace_all_id)
                        .on_press(Message::ResolveConflict(ConflictResolution::ReplaceAll)),
                );
            return Some(
                widget::dialog()
                    .title("File Already Exists")
                    .body(body)
                    .control(all_buttons)
                    .primary_action(
                        widget::button::destructive("Replace")
                            .id(replace_id)
                            .on_press(Message::ResolveConflict(ConflictResolution::Replace)),
                    )
                    .secondary_action(
                        widget::button::standard("Skip")
                            .id(skip_id)
                            .on_press(Message::ResolveConflict(ConflictResolution::Skip)),
                    )
                    .into(),
            );
        }

        if let Some(compress) = &self.compress {
            let body = match compress.sources.as_slice() {
                [single] => format!("Compress \"{}\" into a zip archive.", tab_label(single)),
                many => format!("Compress {} items into a zip archive.", many.len()),
            };
            return Some(
                widget::dialog()
                    .title("Compress")
                    .body(body)
                    .control(
                        widget::text_input("Archive name", compress.name.as_str())
                            .id(self.dialog_input.clone())
                            .on_input(Message::CompressNameChanged)
                            .on_submit(|_| Message::ConfirmCompress),
                    )
                    .primary_action(
                        widget::button::suggested("Compress")
                            .id(self.dialog_confirm.clone())
                            .on_press(Message::ConfirmCompress),
                    )
                    .secondary_action(
                        widget::button::standard("Cancel")
                            .id(self.dialog_cancel.clone())
                            .on_press(Message::CancelCompress),
                    )
                    .into(),
            );
        }

        if let Some(open_with) = &self.open_with {
            return Some(open_with_dialog(open_with));
        }

        if let Some(details) = &self.details {
            return Some(details_dialog(details));
        }

        if let Some(new_folder) = &self.new_folder {
            return Some(
                widget::dialog()
                    .title("New Folder")
                    .control(
                        widget::text_input("Folder name", new_folder.name.as_str())
                            .id(self.dialog_input.clone())
                            .on_input(Message::NewFolderInputChanged)
                            .on_submit(|_| Message::ConfirmNewFolder),
                    )
                    .primary_action(
                        widget::button::suggested("Create")
                            .id(self.dialog_confirm.clone())
                            .on_press(Message::ConfirmNewFolder),
                    )
                    .secondary_action(
                        widget::button::standard("Cancel")
                            .id(self.dialog_cancel.clone())
                            .on_press(Message::CancelNewFolder),
                    )
                    .into(),
            );
        }

        if let Some(rename) = &self.rename {
            return Some(
                widget::dialog()
                    .title("Rename")
                    .body(format!("Renaming \"{}\"", tab_label(&rename.path)))
                    .control(
                        widget::text_input("New name", rename.name.as_str())
                            .id(self.dialog_input.clone())
                            .on_input(Message::RenameInputChanged)
                            .on_submit(|_| Message::ConfirmRename),
                    )
                    .primary_action(
                        widget::button::suggested("Rename")
                            .id(self.dialog_confirm.clone())
                            .on_press(Message::ConfirmRename),
                    )
                    .secondary_action(
                        widget::button::standard("Cancel")
                            .id(self.dialog_cancel.clone())
                            .on_press(Message::CancelRename),
                    )
                    .into(),
            );
        }

        if let Some(sources) = &self.confirm_delete {
            let body = match sources.as_slice() {
                [single] => format!("Move \"{}\" to the trash?", tab_label(single)),
                many => format!("Move {} items to the trash?", many.len()),
            };
            return Some(
                widget::dialog()
                    .title("Delete")
                    .body(body)
                    .primary_action(
                        widget::button::destructive("Delete")
                            .id(self.confirm_delete_button.clone())
                            .on_press(Message::ConfirmDelete),
                    )
                    .secondary_action(
                        widget::button::standard("Cancel")
                            .id(self.cancel_delete_button.clone())
                            .on_press(Message::CancelDelete),
                    )
                    .into(),
            );
        }

        None
    }

    /// The progress panel of a running copy / move / delete / compress: a
    /// strong accent-colored bar (red on error) with a thick progress line,
    /// the percentage, how much is done and the current file.
    fn footer(&self) -> Option<Element<'_, Message>> {
        let op = self.operation.as_ref()?;
        let failed = op.error.is_some();
        let percent = (op.fraction() * 100.0).round() as u32;

        let title = match &op.error {
            Some(_) => format!("{} failed", op.kind.label()),
            None => format!("{} {percent}%", op.kind.label()),
        };
        let mut amounts = Vec::new();
        if op.files_total > 0 {
            amounts.push(format!("{} of {} files", op.files_done, op.files_total));
        }
        if op.bytes_total > 0 {
            amounts.push(format!(
                "{} of {}",
                format_size(op.bytes_done),
                format_size(op.bytes_total)
            ));
        }
        let detail = match (&op.error, &op.current_file) {
            (Some(err), _) => err.clone(),
            (None, Some(file)) => format!("{}  —  {}", amounts.join("  ·  "), tab_label(file)),
            (None, None) => "Preparing…".to_string(),
        };

        // The theme's colors, resolved now: the bar's style takes plain colors.
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();
        let background: cosmic::iced::Color = if failed {
            cosmic.destructive_color().into()
        } else {
            cosmic.accent_color().into()
        };
        let foreground: cosmic::iced::Color = cosmic.on_accent_color().into();

        let header = widget::Row::new()
            .spacing(12)
            .align_y(Alignment::Center)
            .push(widget::text::title4(title))
            .push(
                widget::text::body(detail)
                    .width(Length::Fill)
                    .wrapping(cosmic::iced::core::text::Wrapping::None),
            )
            .push(
                widget::button::standard(if failed { "Close" } else { "Cancel" })
                    .on_press(Message::CancelOperation),
            );
        let bar = widget::determinate_linear(op.fraction())
            .width(Length::Fill)
            .girth(10)
            .class(
                cosmic::widget::progress_bar::style::Class::default()
                    .bar_color(foreground)
                    .track_color(cosmic::iced::Color {
                        a: 0.3,
                        ..foreground
                    }),
            );

        let panel = widget::Column::new().spacing(8).push(header).push(bar);
        Some(
            widget::container(panel)
                .padding([10, 14])
                .width(Length::Fill)
                .class(cosmic::theme::Container::custom(move |theme| {
                    widget::container::Style {
                        text_color: Some(foreground),
                        icon_color: Some(foreground),
                        background: Some(cosmic::iced::Background::Color(background)),
                        border: cosmic::iced::Border {
                            radius: theme.cosmic().radius_s().into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }))
                .into(),
        )
    }

    fn view(&self) -> Element<'_, Message> {
        // How a click on a file changes the selection (Ctrl / Shift held).
        let select_mode = SelectMode::from_modifiers(self.modifiers);
        let can_paste = self.clipboard.is_some();
        let left = self
            .left
            .view(
                self.active_pane == PaneId::Left,
                select_mode,
                &self.keybinds,
                can_paste,
            )
            .map(|msg| Message::Pane(PaneId::Left, msg));
        let right = self
            .right
            .view(
                self.active_pane == PaneId::Right,
                select_mode,
                &self.keybinds,
                can_paste,
            )
            .map(|msg| Message::Pane(PaneId::Right, msg));

        let panes = widget::Row::new()
            .spacing(PANE_GAP)
            .push(pane_frame(left, self.active_pane == PaneId::Left))
            .push(pane_frame(right, self.active_pane == PaneId::Right))
            .width(Length::Fill)
            .height(Length::Fill);

        widget::Column::new()
            .push(panes)
            .push(function_key_bar())
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

/// Total-Commander-style function-key button row, always visible at the
/// bottom of the window as a mouse-accessible mirror of the matching keybinds.
fn function_key_bar() -> Element<'static, Message> {
    // `button::standard(..).width(Fill)` only widens the label inside the
    // button, so build the button around a centered label and make the button
    // itself fill: every key then gets an equal share of the window's width.
    let key_button = |label: &'static str, action: Action| {
        widget::button::custom(
            widget::text(label)
                .width(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center),
        )
        .class(cosmic::theme::Button::Standard)
        .padding([6, 4])
        .width(Length::Fill)
        .on_press(Message::Action(action))
    };

    widget::Row::new()
        .spacing(4)
        .padding(4)
        .push(key_button("F2 Rename", Action::Rename))
        .push(key_button("F3 View", Action::View))
        .push(key_button("F4 Edit", Action::Edit))
        .push(key_button("F5 Copy", Action::Copy))
        .push(key_button("F6 Move", Action::Move))
        .push(key_button("F7 MkDir", Action::NewFolder))
        .push(key_button("F8 Delete", Action::Delete))
        .push(key_button("F9 Terminal", Action::Terminal))
        .push(key_button("Alt+F4 Exit", Action::Quit))
        .width(Length::Fill)
        .into()
}

fn key_subscription() -> Subscription<Message> {
    listen_raw(|event, status, _window| match event {
        // Skip keys a focused widget already handled, e.g. typing Delete or
        // Ctrl+A in the Favorites name field mustn't act on the file listing.
        Event::Keyboard(KeyEvent::KeyPressed { key, modifiers, .. })
            if status == Status::Ignored =>
        {
            Some(Message::Key(modifiers, key))
        }
        Event::Keyboard(KeyEvent::KeyPressed {
            key: Key::Named(Named::Escape),
            ..
        }) => Some(Message::EscapeCaptured),
        Event::Keyboard(KeyEvent::ModifiersChanged(modifiers)) => {
            Some(Message::ModifiersChanged(modifiers))
        }
        _ => None,
    })
}

fn open_with_dialog(state: &OpenWithState) -> Element<'_, Message> {
    let body = match state.paths.as_slice() {
        [single] => format!("Choose an application to open \"{}\".", tab_label(single)),
        many => format!("Choose an application to open {} items.", many.len()),
    };

    let control: Element<'_, Message> = match &state.apps {
        None => widget::text("Loading applications…").into(),
        Some(apps) if apps.is_empty() => widget::text("No applications found.").into(),
        Some(apps) => {
            let mut list = widget::Column::new().spacing(2);
            let mut shown_recommended_header = false;
            let mut shown_other_header = false;
            for (index, app) in apps.iter().enumerate() {
                if app.recommended && !shown_recommended_header {
                    list = list.push(widget::text::heading("Recommended Applications"));
                    shown_recommended_header = true;
                } else if !app.recommended && !app.is_default && !shown_other_header {
                    list = list.push(widget::text::heading("Other Applications"));
                    shown_other_header = true;
                }
                let mut row = widget::Row::new()
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(widget::icon(app.icon.clone()).size(24))
                    .push(widget::text(app.name.clone()).width(Length::Fill));
                if app.is_default {
                    row = row.push(widget::text::caption("Default"));
                }
                list = list.push(
                    widget::button::custom(row)
                        .class(cosmic::theme::Button::MenuItem)
                        .width(Length::Fill)
                        .on_press(Message::LaunchApp(index)),
                );
            }
            widget::scrollable(list).height(Length::Fixed(360.0)).into()
        }
    };

    widget::dialog()
        .title("Open With")
        .body(body)
        .control(control)
        .secondary_action(widget::button::standard("Cancel").on_press(Message::CancelOpenWith))
        .into()
}

fn details_dialog(state: &DetailsState) -> Element<'_, Message> {
    let mut rows: Vec<(&'static str, String)> = Vec::new();
    let location = state
        .paths
        .first()
        .and_then(|path| path.parent())
        .map(|parent| parent.display().to_string())
        .unwrap_or_default();

    let title = match state.paths.as_slice() {
        [single] => tab_label(single),
        many => format!("{} items", many.len()),
    };

    match &state.result {
        None => rows.push(("Size", "Calculating…".to_string())),
        Some(Err(err)) => rows.push(("Error", err.clone())),
        Some(Ok(details)) => {
            let contents = if details.contained_files + details.contained_dirs > 0 {
                format!(
                    " — {} files, {} folders inside",
                    details.contained_files, details.contained_dirs
                )
            } else {
                String::new()
            };
            let size = format!(
                "{} ({} bytes){contents}",
                format_size(details.total_size),
                details.total_size
            );

            match &details.single {
                None => {
                    rows.push(("Items", details.item_count.to_string()));
                    rows.push(("Location", location));
                    rows.push(("Total size", size));
                }
                Some(entry) => {
                    let kind = match entry.kind {
                        EntryKind::Dir => "Folder",
                        EntryKind::File => "File",
                        EntryKind::Symlink => "Symbolic link",
                    };
                    let kind = match &entry.mime_type {
                        Some(mime) => format!("{kind} ({mime})"),
                        None => kind.to_string(),
                    };
                    rows.push(("Name", tab_label(&entry.path)));
                    rows.push(("Type", kind));
                    rows.push(("Location", location));
                    if let Some(target) = &entry.symlink_target {
                        rows.push(("Link target", target.display().to_string()));
                    }
                    rows.push(("Size", size));
                    let time = |t: Option<std::time::SystemTime>| {
                        t.map(format_modified).unwrap_or_else(|| "Unknown".to_string())
                    };
                    rows.push(("Modified", time(entry.modified)));
                    rows.push(("Accessed", time(entry.accessed)));
                    rows.push(("Created", time(entry.created)));
                    rows.push((
                        "Permissions",
                        format!(
                            "{} ({:o})",
                            fs_ops::details::format_mode(entry.mode),
                            entry.mode
                        ),
                    ));
                    rows.push(("Owner", entry.owner.clone()));
                    rows.push(("Group", entry.group.clone()));
                }
            }
        }
    }

    let mut grid = widget::Column::new().spacing(6);
    for (label, value) in rows {
        grid = grid.push(
            widget::Row::new()
                .spacing(12)
                .push(widget::text::heading(label).width(Length::Fixed(110.0)))
                .push(widget::text(value).width(Length::Fill)),
        );
    }

    widget::dialog()
        .title(title)
        .control(grid)
        .primary_action(widget::button::standard("Close").on_press(Message::CloseDetails))
        .into()
}

/// `<name>.zip` for a single item (dropping a file's own extension), else
/// `Archive.zip`, numbered if that name is already taken in `dir`.
fn default_archive_name(sources: &[PathBuf], dir: &std::path::Path) -> String {
    let stem = match sources {
        [single] if single.is_dir() => tab_label(single),
        [single] => single
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| tab_label(single)),
        _ => "Archive".to_string(),
    };
    let mut name = format!("{stem}.zip");
    let mut n = 2;
    while dir.join(&name).exists() {
        name = format!("{stem} ({n}).zip");
        n += 1;
    }
    name
}

/// Space between the two panes, in pixels.
const PANE_GAP: u16 = 5;
/// Width of the border drawn around each pane, in pixels.
const PANE_BORDER: f32 = 2.0;

/// Encloses a pane in a rounded rectangle. The active pane's border uses the
/// accent color; the other uses the text color (white in a dark theme).
fn pane_frame(content: Element<'_, Message>, is_active: bool) -> Element<'_, Message> {
    widget::container(content)
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .class(cosmic::theme::Container::custom(move |theme| {
            let cosmic = theme.cosmic();
            let color = if is_active {
                cosmic.accent_color()
            } else {
                cosmic.on_bg_color()
            };
            widget::container::Style {
                border: cosmic::iced::Border {
                    color: color.into(),
                    width: PANE_BORDER,
                    radius: cosmic.radius_s().into(),
                },
                ..Default::default()
            }
        }))
        .into()
}

/// The keyboard highlight for a list row: the accent color, like a selected
/// file in the listing.
fn highlight_style(theme: &cosmic::Theme, highlighted: bool) -> widget::container::Style {
    if !highlighted {
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
}

