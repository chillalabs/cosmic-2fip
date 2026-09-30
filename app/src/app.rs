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
use fs_ops::search::SearchEvent;
use fs_ops::session::Session;
use fs_ops::settings::{ColorTheme, FontSize, IconStyle, Settings, ViewMode};
use fs_ops::user_dirs::UserDir;
use fs_ops::EntryKind;

use crate::file_item::{format_modified, format_size, ListingOptions};
use crate::find::{FindState, FindStatus};
use crate::fl;
use crate::keybinds::{default_keybinds, Action};
use crate::launch::{self, open_with_default_app, AppEntry, OpenMode};
use crate::localize;
use crate::menu_bar::menu_bar;
use crate::operation::{OpKind, OperationState};
use crate::pane::{home_dir, PaneMessage, PaneState, SelectMode};
use crate::tab::{matches_filter, tab_label};
use crate::themes;

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
    /// A click on a pane's empty space: make it the active pane.
    ActivatePane(PaneId),
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
    ShowHiddenFilesToggled(bool),
    ThumbnailsToggled(bool),
    SeparateExtensionToggled(bool),
    /// Index into [`localize::LANGUAGES`]; applied immediately.
    LanguageSelected(usize),
    /// Index into [`ICON_STYLES`].
    IconStyleSelected(usize),
    /// Index into [`FONT_SIZES`].
    FontSizeSelected(usize),
    /// Index into [`COLOR_THEMES`].
    ColorThemeSelected(usize),
    /// A click on a Settings row: give it the keyboard focus.
    FocusSettingsRow(SettingsRow),
    /// A click on a favorite: highlight it for the keyboard.
    FocusFavorite(usize),
    CloseDrawer,
    FindPatternChanged(String),
    FindStart,
    FindStop,
    /// Results of the search with this generation (see `FindState`).
    FindEvent(u64, SearchEvent),
    FindHighlight(usize),
    FindGoTo(usize),
    FindClose,
}

/// Index of "Skip" in `App::conflict_buttons`: the safe default focus.
const CONFLICT_SKIP: usize = 2;

/// File name sizes offered in Settings, in dropdown order.
const FONT_SIZES: [FontSize; 4] = [
    FontSize::Default,
    FontSize::Small,
    FontSize::Smaller,
    FontSize::Tiny,
];

/// Where the Favorites dialog's keyboard focus is: on the list (see
/// `favorite_cursor`), the new favorite's name field, or a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FavoriteFocus {
    List,
    NameField,
    AddButton,
    CloseButton,
}

/// Text sizes in the Favorites dialog: a step below libcosmic's body (14)
/// and caption (12).
const FAVORITES_NAME_SIZE: f32 = 13.0;
const FAVORITES_PATH_SIZE: f32 = 11.0;
const FAVORITES_DIALOG_WIDTH: f32 = 620.0;
const FAVORITES_LIST_MAX_HEIGHT: f32 = 360.0;

/// The Settings panel's rows, top to bottom, for keyboard navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsRow {
    ShowHidden,
    SeparateExtension,
    Thumbnails,
    Language,
    ColorTheme,
    IconStyle,
    FontSize,
}

const SETTINGS_ROWS: [SettingsRow; 7] = [
    SettingsRow::ShowHidden,
    SettingsRow::SeparateExtension,
    SettingsRow::Thumbnails,
    SettingsRow::Language,
    SettingsRow::ColorTheme,
    SettingsRow::IconStyle,
    SettingsRow::FontSize,
];

/// `index` moved by `step` (±1) within `len` options, wrapping around.
fn step_index(index: usize, step: isize, len: usize) -> usize {
    (index as isize + step).rem_euclid(len as isize) as usize
}

/// Color themes offered in Settings, in dropdown order.
const COLOR_THEMES: [ColorTheme; 15] = [
    ColorTheme::System,
    ColorTheme::Light,
    ColorTheme::Dark,
    // 2fip's own palettes, alphabetically.
    ColorTheme::AyuDark,
    ColorTheme::CatppuccinMacchiato,
    ColorTheme::CatppuccinMocha,
    ColorTheme::Dracula,
    ColorTheme::Everforest,
    ColorTheme::GruvboxDark,
    ColorTheme::GruvboxMaterial,
    ColorTheme::Matrix,
    ColorTheme::MonokaiPro,
    ColorTheme::Nord,
    ColorTheme::SolarizedDark,
    ColorTheme::TokyoNightStorm,
];

fn color_theme_name(theme: ColorTheme) -> String {
    match theme {
        ColorTheme::System => fl!("color-theme-system"),
        ColorTheme::Light => fl!("color-theme-light"),
        ColorTheme::Dark => fl!("color-theme-dark"),
        // Proper names: the same in every language.
        ColorTheme::Dracula => "Dracula".to_string(),
        ColorTheme::Everforest => "Everforest".to_string(),
        ColorTheme::GruvboxMaterial => "Gruvbox Material".to_string(),
        ColorTheme::Nord => "Nord".to_string(),
        ColorTheme::TokyoNightStorm => "Tokyo Night Storm".to_string(),
        ColorTheme::CatppuccinMocha => "Catppuccin Mocha".to_string(),
        ColorTheme::CatppuccinMacchiato => "Catppuccin Macchiato".to_string(),
        ColorTheme::AyuDark => "Ayu Dark".to_string(),
        ColorTheme::Matrix => "Matrix".to_string(),
        ColorTheme::MonokaiPro => "Monokai Pro".to_string(),
        ColorTheme::SolarizedDark => "Solarized Dark".to_string(),
        ColorTheme::GruvboxDark => "Gruvbox Dark".to_string(),
    }
}

/// Icon styles offered in Settings, in dropdown order.
const ICON_STYLES: [IconStyle; 2] = [IconStyle::Colorful, IconStyle::Monochrome];

/// Dropdown label for a file name size, e.g. "Small (13 px)".
fn font_size_name(size: FontSize) -> String {
    let px = size.px();
    match size {
        FontSize::Default => fl!("font-size-default", px = px),
        FontSize::Small => fl!("font-size-small", px = px),
        FontSize::Smaller => fl!("font-size-smaller", px = px),
        FontSize::Tiny => fl!("font-size-tiny", px = px),
    }
}

fn icon_style_name(style: IconStyle) -> String {
    match style {
        IconStyle::Colorful => fl!("icon-style-colorful"),
        IconStyle::Monochrome => fl!("icon-style-monochrome"),
    }
}

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
    /// Where the Favorites dialog's keyboard focus is.
    favorite_focus: FavoriteFocus,
    /// The Find Files dialog, while open.
    find: Option<FindState>,
    favorite_add_button: widget::Id,
    favorite_close_button: widget::Id,
    favorite_name_input: widget::Id,
    /// The Settings row with keyboard focus (index into [`SETTINGS_ROWS`]).
    settings_cursor: usize,
    clipboard: Option<Clipboard>,
    compress: Option<CompressState>,
    open_with: Option<OpenWithState>,
    details: Option<DetailsState>,
    settings: Settings,
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
            Message::Pane(
                id,
                PaneMessage::DropFiles {
                    dest,
                    paths,
                    is_move,
                },
            ) => {
                self.set_active_pane(id);
                self.left.clear_drop_hover();
                self.right.clear_drop_hover();
                self.drop_files(dest, paths, is_move)
            }
            Message::Pane(id, msg) => {
                // A listing finishing its load isn't the user picking that
                // pane (after an operation both panes reload in the background),
                // and neither is a drag passing over it.
                if !matches!(
                    msg,
                    PaneMessage::DirLoaded(..)
                        | PaneMessage::DropHover(..)
                        | PaneMessage::DropLeave { .. }
                ) {
                    self.set_active_pane(id);
                }
                self.pane_mut(id).update(msg)
            }
            Message::Action(action) => self.handle_action(action),
            Message::ExitNow => self.exit_now(),
            Message::ActivatePane(id) => {
                self.set_active_pane(id);
                Task::none()
            }
            Message::EscapeCaptured => {
                // Text boxes swallow Escape; still let one press close the
                // dialog they're in (e.g. Rename), the side panel (e.g. the
                // Favorites name field), the path editor or the filter.
                if self.dialog_open() || self.find.is_some() {
                    return self.on_escape();
                }
                if self.drawer.is_some() {
                    self.close_drawer();
                    return Task::none();
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
                if self.find.is_some() {
                    // Find Files is modal: only Ctrl+F (to close it) and
                    // Quit get past it.
                    let action = self.keybinds.iter().find_map(|(bind, action)| {
                        bind.matches(modifiers, &key, None).then_some(*action)
                    });
                    if let Some(action @ (Action::Find | Action::Quit)) = action {
                        return self.handle_action(action);
                    }
                    return self.find_key(modifiers, &key);
                }
                if let Some(task) = self.favorites_key(modifiers, &key) {
                    return task;
                }
                if let Some(task) = self.settings_key(modifiers, &key) {
                    return task;
                }
                let action = self.keybinds.iter().find_map(|(bind, action)| {
                    bind.matches(modifiers, &key, None).then_some(*action)
                });
                // A side panel keeps the keyboard: keys it doesn't use must not
                // reach the file panels behind it (e.g. Delete trashing files).
                // Only the shortcuts that switch panels or quit still work.
                if self.drawer.is_some()
                    && !matches!(
                        action,
                        Some(Action::Favorites | Action::Settings | Action::Quit)
                    )
                {
                    return Task::none();
                }
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
                    self.reload_both_panes()
                }
                OpEvent::Cancelled => {
                    self.operation = None;
                    self.reload_both_panes()
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
                self.reload_both_panes()
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
                self.reload_both_panes()
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
                self.pane_mut(self.active_pane)
                    .update(PaneMessage::Navigate(path))
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
            Message::ShowHiddenFilesToggled(show) => {
                self.focus_settings_row(SettingsRow::ShowHidden);
                self.set_hide_hidden_files(!show)
            }
            Message::SeparateExtensionToggled(separate) => {
                self.focus_settings_row(SettingsRow::SeparateExtension);
                self.settings.separate_extension = separate;
                self.apply_settings()
            }
            Message::ThumbnailsToggled(show) => {
                self.focus_settings_row(SettingsRow::Thumbnails);
                self.settings.show_thumbnails = show;
                self.apply_settings()
            }
            Message::LanguageSelected(index) => {
                self.focus_settings_row(SettingsRow::Language);
                if let Some((code, _)) = localize::LANGUAGES.get(index) {
                    self.settings.language = code.to_string();
                    // Every view reads its text through `fl!`, so the next
                    // redraw is already in the new language.
                    localize::set_language(code);
                    self.persist_settings();
                }
                Task::none()
            }
            Message::FocusFavorite(index) => {
                self.favorite_cursor = Some(index);
                self.favorite_focus = FavoriteFocus::List;
                unfocus_text_fields()
            }
            Message::FocusSettingsRow(row) => {
                self.focus_settings_row(row);
                Task::none()
            }
            Message::ColorThemeSelected(index) => {
                self.focus_settings_row(SettingsRow::ColorTheme);
                if let Some(theme) = COLOR_THEMES.get(index) {
                    self.settings.color_theme = *theme;
                    self.persist_settings();
                    return self.apply_color_theme();
                }
                Task::none()
            }
            Message::FontSizeSelected(index) => {
                self.focus_settings_row(SettingsRow::FontSize);
                if let Some(size) = FONT_SIZES.get(index) {
                    self.settings.name_font_size = *size;
                    return self.apply_settings();
                }
                Task::none()
            }
            Message::IconStyleSelected(index) => {
                self.focus_settings_row(SettingsRow::IconStyle);
                if let Some(style) = ICON_STYLES.get(index) {
                    self.settings.icon_style = *style;
                    return self.apply_settings();
                }
                Task::none()
            }
            Message::CloseDrawer => {
                self.close_drawer();
                Task::none()
            }
            Message::FindPatternChanged(pattern) => {
                if let Some(find) = &mut self.find {
                    find.pattern = pattern;
                    find.input_focused = true;
                }
                Task::none()
            }
            Message::FindStart => self.start_find(),
            Message::FindStop => {
                if let Some(find) = &self.find {
                    find.cancel.cancel();
                }
                Task::none()
            }
            Message::FindEvent(generation, event) => {
                let Some(find) = &mut self.find else {
                    return Task::none();
                };
                if find.generation != generation {
                    return Task::none(); // From a search since replaced.
                }
                match event {
                    SearchEvent::Found(hits) => find.hits.extend(hits),
                    SearchEvent::Done { truncated } => {
                        find.status = FindStatus::Done { truncated };
                    }
                }
                Task::none()
            }
            Message::FindHighlight(index) => {
                if let Some(find) = &mut self.find {
                    find.cursor = Some(index);
                    find.input_focused = false;
                }
                unfocus_text_fields()
            }
            Message::FindGoTo(index) => {
                let Some(find) = self.find.take() else {
                    return Task::none();
                };
                find.cancel.cancel();
                match find.hits.into_iter().nth(index) {
                    Some(hit) => self.pane_mut(self.active_pane).reveal(hit.path),
                    None => Task::none(),
                }
            }
            Message::FindClose => {
                self.close_find();
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

    fn reload_both_panes(&mut self) -> Task<Message> {
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
                    name: fl!("new-folder-default-name"),
                });
                self.focus_dialog_field(0)
            }
            Action::NewTab => self.pane_mut(self.active_pane).update(PaneMessage::NewTab),
            Action::CloseTab => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::CloseActiveTab),
            Action::GoBack => self.pane_mut(self.active_pane).update(PaneMessage::GoBack),
            Action::GoForward => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::GoForward),
            Action::GoUp => self.pane_mut(self.active_pane).update(PaneMessage::GoUp),
            Action::SelectNext => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::SelectNext),
            Action::CalculateSize => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::CalculateDirSizes),
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
            Action::SelectAll => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::SelectAll),
            Action::Settings => {
                self.toggle_drawer(DrawerPage::Settings);
                Task::none()
            }
            Action::Refresh => self.reload_both_panes(),
            Action::Find => self.toggle_find(),
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
            font_size: self.settings.name_font_size,
            separate_extension: self.settings.separate_extension,
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

    /// Switches 2fip's colors to the chosen theme; "System" goes back to the
    /// desktop's current theme (libcosmic keeps it up to date meanwhile).
    fn apply_color_theme(&self) -> Task<Message> {
        let theme = match self.settings.color_theme {
            ColorTheme::System => self.core.system_theme().clone(),
            ColorTheme::Light => cosmic::Theme::light(),
            ColorTheme::Dark => cosmic::Theme::dark(),
            own => themes::build(own).unwrap_or_else(cosmic::Theme::dark),
        };
        cosmic::command::set_theme(theme)
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
            // Settings slides in from the right; Favorites is a centered
            // dialog (see `dialog`).
            self.core.set_show_context(page == DrawerPage::Settings);
            if page == DrawerPage::Settings {
                self.settings_cursor = 0;
            }
            if page == DrawerPage::Favorites {
                self.favorite_focus = FavoriteFocus::List;
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

    /// While the Settings panel is open it has the keyboard: Tab/↓ and
    /// Shift+Tab/↑ move between settings, Space/Enter toggle a checkbox (or
    /// pick the next option), →/← pick the next/previous option. Returns
    /// `None` for other keys (e.g. Escape closes the panel as usual).
    fn settings_key(&mut self, modifiers: Modifiers, key: &Key) -> Option<Task<Message>> {
        if self.drawer != Some(DrawerPage::Settings) {
            return None;
        }
        let rows = SETTINGS_ROWS.len();
        let plain = modifiers.is_empty();
        let row = SETTINGS_ROWS[self.settings_cursor.min(rows - 1)];
        match key {
            Key::Named(Named::Tab) if plain => {
                self.settings_cursor = step_index(self.settings_cursor, 1, rows);
            }
            Key::Named(Named::Tab) if modifiers == Modifiers::SHIFT => {
                self.settings_cursor = step_index(self.settings_cursor, -1, rows);
            }
            Key::Named(Named::ArrowDown) if plain => {
                self.settings_cursor = step_index(self.settings_cursor, 1, rows);
            }
            Key::Named(Named::ArrowUp) if plain => {
                self.settings_cursor = step_index(self.settings_cursor, -1, rows);
            }
            Key::Named(Named::ArrowRight | Named::Enter) if plain => {
                return Some(self.change_setting(row, 1));
            }
            Key::Character(c) if plain && c.as_str() == " " => {
                return Some(self.change_setting(row, 1));
            }
            Key::Named(Named::ArrowLeft) if plain => return Some(self.change_setting(row, -1)),
            _ => return None,
        }
        Some(Task::none())
    }

    /// Moves the Settings keyboard focus to `row`.
    fn focus_settings_row(&mut self, row: SettingsRow) {
        if let Some(index) = SETTINGS_ROWS.iter().position(|r| *r == row) {
            self.settings_cursor = index;
        }
    }

    /// Toggles a checkbox setting, or moves a list setting `step` options.
    fn change_setting(&mut self, row: SettingsRow, step: isize) -> Task<Message> {
        let message = match row {
            SettingsRow::ShowHidden => {
                // Shown now ⇔ hidden before: flip it.
                Message::ShowHiddenFilesToggled(self.settings.hide_hidden_files)
            }
            SettingsRow::SeparateExtension => {
                Message::SeparateExtensionToggled(!self.settings.separate_extension)
            }
            SettingsRow::Thumbnails => Message::ThumbnailsToggled(!self.settings.show_thumbnails),
            SettingsRow::Language => {
                let current = localize::LANGUAGES
                    .iter()
                    .position(|(code, _)| *code == self.settings.language)
                    .unwrap_or(0);
                Message::LanguageSelected(step_index(current, step, localize::LANGUAGES.len()))
            }
            SettingsRow::ColorTheme => {
                let current = COLOR_THEMES
                    .iter()
                    .position(|theme| *theme == self.settings.color_theme)
                    .unwrap_or(0);
                Message::ColorThemeSelected(step_index(current, step, COLOR_THEMES.len()))
            }
            SettingsRow::IconStyle => {
                let current = ICON_STYLES
                    .iter()
                    .position(|style| *style == self.settings.icon_style)
                    .unwrap_or(0);
                Message::IconStyleSelected(step_index(current, step, ICON_STYLES.len()))
            }
            SettingsRow::FontSize => {
                let current = FONT_SIZES
                    .iter()
                    .position(|size| *size == self.settings.name_font_size)
                    .unwrap_or(0);
                Message::FontSizeSelected(step_index(current, step, FONT_SIZES.len()))
            }
        };
        self.update(message)
    }

    /// Wraps a Settings row, outlined in the accent color when it has the
    /// keyboard focus.
    fn settings_row<'a>(
        &self,
        row: SettingsRow,
        content: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let focused = SETTINGS_ROWS.get(self.settings_cursor) == Some(&row);
        let framed = widget::container(content)
            .padding(4)
            .class(cosmic::theme::Container::custom(move |theme| {
                if !focused {
                    return widget::container::Style::default();
                }
                // A tinted background plus a border, so the focus stands out
                // in every color theme.
                let cosmic = theme.cosmic();
                let accent: cosmic::iced::Color = cosmic.accent_color().into();
                widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(cosmic::iced::Color {
                        a: 0.18,
                        ..accent
                    })),
                    border: cosmic::iced::Border {
                        color: accent,
                        width: 2.0,
                        radius: cosmic.radius_s().into(),
                    },
                    ..Default::default()
                }
            }));
        // A click on the row's title or description (anything that doesn't
        // handle the click itself) moves the focus here.
        widget::mouse_area(framed)
            .on_press(Message::FocusSettingsRow(row))
            .into()
    }

    /// A Settings checkbox row: the checkbox before the title and description
    /// (like libcosmic's own checkbox rows), toggled by a click anywhere on
    /// the row, with the keyboard focus outline.
    fn checkbox_row<'a>(
        &self,
        row: SettingsRow,
        title: String,
        description: String,
        checked: bool,
        on_toggle: fn(bool) -> Message,
    ) -> Element<'a, Message> {
        let content = widget::settings::item::builder(title)
            .description(description)
            .icon(widget::checkbox(checked).on_toggle(on_toggle))
            .control(widget::Space::new());
        // The whole row toggles (and so also takes the focus, see `update`).
        self.settings_row(
            row,
            widget::mouse_area(content).on_press(on_toggle(!checked)),
        )
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
        let count = self.favorites.len();
        let forward = match (key, modifiers) {
            (Key::Named(Named::ArrowDown), m) if m.is_empty() => Some(true),
            (Key::Named(Named::Tab), m) if m.is_empty() => Some(true),
            (Key::Named(Named::ArrowUp), m) if m.is_empty() => Some(false),
            (Key::Named(Named::Tab), m) if m == Modifiers::SHIFT => Some(false),
            _ => None,
        };
        if let Some(forward) = forward {
            return Some(self.move_favorite_focus(forward));
        }
        if !modifiers.is_empty() {
            return None;
        }
        match self.favorite_focus {
            FavoriteFocus::List => {}
            // The name field handles its own typing (and Enter adds).
            FavoriteFocus::NameField => return None,
            // A focused button runs on Enter or Space by itself; this is the
            // fallback if it didn't take the key.
            FavoriteFocus::AddButton | FavoriteFocus::CloseButton => {
                let message = match self.favorite_focus {
                    FavoriteFocus::AddButton => Message::AddFavorite,
                    _ => Message::CloseDrawer,
                };
                return match key {
                    Key::Named(Named::Enter) => Some(self.update(message)),
                    Key::Character(c) if c.as_str() == " " => Some(self.update(message)),
                    _ => None,
                };
            }
        }
        match key {
            Key::Named(Named::Home) if count > 0 => {
                self.favorite_cursor = Some(0);
                Some(Task::none())
            }
            Key::Named(Named::End) if count > 0 => {
                self.favorite_cursor = Some(count - 1);
                Some(Task::none())
            }
            Key::Named(Named::Enter) => Some(match self.favorite_cursor {
                Some(index) => self.update(Message::GoToFavorite(index)),
                None => Task::none(),
            }),
            Key::Named(Named::Delete) => Some(match self.favorite_cursor {
                Some(index) => self.update(Message::RemoveFavorite(index)),
                None => Task::none(),
            }),
            _ => None,
        }
    }

    /// Moves the Favorites keyboard focus one step: through the favorites,
    /// then the "Add current folder" name field, the Add button and Close,
    /// wrapping around.
    fn move_favorite_focus(&mut self, forward: bool) -> Task<Message> {
        let count = self.favorites.len();
        // Positions 0..count are favorites, then the name field, Add, Close.
        let current = match self.favorite_focus {
            FavoriteFocus::List => self.favorite_cursor.unwrap_or(count),
            FavoriteFocus::NameField => count,
            FavoriteFocus::AddButton => count + 1,
            FavoriteFocus::CloseButton => count + 2,
        };
        let target = step_index(current, if forward { 1 } else { -1 }, count + 3);
        if target < count {
            self.favorite_focus = FavoriteFocus::List;
            self.favorite_cursor = Some(target);
            return unfocus_text_fields();
        }
        self.favorite_cursor = None;
        match target - count {
            0 => {
                self.favorite_focus = FavoriteFocus::NameField;
                widget::text_input::focus(self.favorite_name_input.clone())
            }
            1 => {
                self.favorite_focus = FavoriteFocus::AddButton;
                widget::button::focus(self.favorite_add_button.clone())
            }
            _ => {
                self.favorite_focus = FavoriteFocus::CloseButton;
                widget::button::focus(self.favorite_close_button.clone())
            }
        }
    }

    fn close_drawer(&mut self) {
        self.drawer = None;
        self.core.set_show_context(false);
    }

    /// Opens the Find Files dialog on the active panel's folder, or closes it.
    fn toggle_find(&mut self) -> Task<Message> {
        if self.find.is_some() {
            self.close_find();
            return Task::none();
        }
        self.close_drawer();
        let find = FindState::new(self.pane(self.active_pane).current_dir());
        let focus = widget::text_input::focus(find.input_id.clone());
        self.find = Some(find);
        focus
    }

    fn close_find(&mut self) {
        if let Some(find) = self.find.take() {
            find.cancel.cancel();
        }
    }

    /// Starts searching for the typed name (stopping any search still running).
    fn start_find(&mut self) -> Task<Message> {
        let include_hidden = !self.settings.hide_hidden_files;
        let Some(find) = &mut self.find else {
            return Task::none();
        };
        let pattern = find.pattern.trim().to_string();
        if pattern.is_empty() {
            return Task::none();
        }
        find.cancel.cancel();
        find.cancel = CancelHandle::new();
        find.generation += 1;
        find.hits.clear();
        find.cursor = None;
        find.status = FindStatus::Searching;
        let generation = find.generation;
        let search = fs_ops::search::search(
            find.root.clone(),
            move |name| matches_filter(name, &pattern),
            include_hidden,
            crate::find::RESULT_LIMIT,
            find.cancel.clone(),
        );
        cosmic::task::stream(search.map(move |event| Message::FindEvent(generation, event)))
    }

    /// Keys while Find Files is open: Tab switches between the name field
    /// and the results; in the results ↑/↓, PageUp/PageDown and Home/End
    /// move the highlight and Enter opens it. The name field handles its
    /// own typing (Enter there searches).
    fn find_key(&mut self, modifiers: Modifiers, key: &Key) -> Task<Message> {
        let Some(find) = &mut self.find else {
            return Task::none();
        };
        let tab = *key == Key::Named(Named::Tab)
            && (modifiers.is_empty() || modifiers == Modifiers::SHIFT);
        let down_from_input =
            find.input_focused && modifiers.is_empty() && *key == Key::Named(Named::ArrowDown);
        if tab || down_from_input {
            if find.input_focused && !find.hits.is_empty() {
                find.input_focused = false;
                find.cursor = find.cursor.or(Some(0));
                return Task::batch([unfocus_text_fields(), find.move_cursor(0)]);
            }
            if !find.input_focused {
                find.input_focused = true;
                return widget::text_input::focus(find.input_id.clone());
            }
            return Task::none();
        }
        if find.input_focused || !modifiers.is_empty() {
            return Task::none();
        }
        let page = crate::find::page_rows();
        match key {
            Key::Named(Named::ArrowUp) if find.cursor == Some(0) => {
                // Up from the first result goes back to the name field.
                find.input_focused = true;
                widget::text_input::focus(find.input_id.clone())
            }
            Key::Named(Named::ArrowDown) => find.move_cursor(1),
            Key::Named(Named::ArrowUp) => find.move_cursor(-1),
            Key::Named(Named::PageDown) => find.move_cursor(page),
            Key::Named(Named::PageUp) => find.move_cursor(-page),
            Key::Named(Named::Home) => find.move_cursor(isize::MIN / 2),
            Key::Named(Named::End) => find.move_cursor(isize::MAX / 2),
            Key::Named(Named::Enter) => match find.cursor {
                Some(index) => self.update(Message::FindGoTo(index)),
                None => Task::none(),
            },
            _ => Task::none(),
        }
    }

    fn settings_page(&self) -> Element<'_, Message> {
        // Option names are built on every draw, so they follow the language.
        let language_names: Vec<String> = localize::LANGUAGES
            .iter()
            .map(|(code, name)| match *code {
                "system" => fl!("language-system"),
                _ => name.to_string(),
            })
            .collect();
        let selected_language = localize::LANGUAGES
            .iter()
            .position(|(code, _)| *code == self.settings.language)
            .or(Some(0));
        let icon_style_names: Vec<String> = ICON_STYLES.into_iter().map(icon_style_name).collect();
        let selected_icon_style = ICON_STYLES
            .iter()
            .position(|style| *style == self.settings.icon_style);
        let font_size_names: Vec<String> = FONT_SIZES.into_iter().map(font_size_name).collect();
        let color_theme_names: Vec<String> =
            COLOR_THEMES.into_iter().map(color_theme_name).collect();
        let selected_color_theme = COLOR_THEMES
            .iter()
            .position(|theme| *theme == self.settings.color_theme);
        let selected_font_size = FONT_SIZES
            .iter()
            .position(|size| *size == self.settings.name_font_size);

        let general = widget::settings::section()
            .title(fl!("settings-general"))
            .add(self.checkbox_row(
                SettingsRow::ShowHidden,
                fl!("settings-show-hidden"),
                fl!("settings-show-hidden-description"),
                !self.settings.hide_hidden_files,
                Message::ShowHiddenFilesToggled,
            ))
            .add(self.checkbox_row(
                SettingsRow::SeparateExtension,
                fl!("settings-separate-ext"),
                fl!("settings-separate-ext-description"),
                self.settings.separate_extension,
                Message::SeparateExtensionToggled,
            ))
            .add(self.checkbox_row(
                SettingsRow::Thumbnails,
                fl!("settings-thumbnails"),
                fl!("settings-thumbnails-description"),
                self.settings.show_thumbnails,
                Message::ThumbnailsToggled,
            ))
            .add(self.settings_row(
                SettingsRow::Language,
                widget::settings::item(
                    fl!("settings-language"),
                    widget::dropdown(language_names, selected_language, Message::LanguageSelected),
                ),
            ));

        let theme = widget::settings::section()
            .title(fl!("settings-theme"))
            .add(
                self.settings_row(
                    SettingsRow::ColorTheme,
                    widget::settings::item::builder(fl!("settings-color-theme"))
                        .description(fl!("settings-color-theme-description"))
                        .control(widget::dropdown(
                            color_theme_names,
                            selected_color_theme,
                            Message::ColorThemeSelected,
                        )),
                ),
            )
            .add(
                self.settings_row(
                    SettingsRow::IconStyle,
                    widget::settings::item::builder(fl!("settings-icon-style"))
                        .description(fl!("settings-icon-style-description"))
                        .control(widget::dropdown(
                            icon_style_names,
                            selected_icon_style,
                            Message::IconStyleSelected,
                        )),
                ),
            )
            .add(
                self.settings_row(
                    SettingsRow::FontSize,
                    widget::settings::item::builder(fl!("settings-font-size"))
                        .description(fl!("settings-font-size-description"))
                        .control(widget::dropdown(
                            font_size_names,
                            selected_font_size,
                            Message::FontSizeSelected,
                        )),
                ),
            );

        widget::settings::view_column(vec![general.into(), theme.into()]).into()
    }

    /// The Favorites dialog, centered over the window. Its text is a step
    /// smaller than libcosmic's dialog and settings defaults, so long folder
    /// paths fit.
    fn favorites_dialog(&self) -> Element<'_, Message> {
        let mut saved = widget::Column::new().spacing(2);
        if self.favorites.is_empty() {
            saved = saved.push(widget::text(fl!("favorites-empty")).size(FAVORITES_NAME_SIZE));
        }
        for (index, favorite) in self.favorites.iter().enumerate() {
            let is_first = index == 0;
            let is_last = index + 1 == self.favorites.len();
            let buttons = widget::Row::new()
                .spacing(4)
                .align_y(Alignment::Center)
                .push(
                    widget::button::icon(widget::icon::from_name("go-up-symbolic"))
                        .tooltip(fl!("favorites-move-up"))
                        .on_press_maybe((!is_first).then_some(Message::MoveFavorite(index, true))),
                )
                .push(
                    widget::button::icon(widget::icon::from_name("go-down-symbolic"))
                        .tooltip(fl!("favorites-move-down"))
                        .on_press_maybe((!is_last).then_some(Message::MoveFavorite(index, false))),
                )
                .push(widget::button::standard(fl!("open")).on_press(Message::GoToFavorite(index)))
                .push(
                    widget::button::icon(widget::icon::from_name("user-trash-symbolic"))
                        .on_press(Message::RemoveFavorite(index)),
                );
            let names = widget::Column::new()
                .spacing(2)
                .width(Length::Fill)
                .push(widget::text(favorite.name.as_str()).size(FAVORITES_NAME_SIZE))
                .push(widget::text(favorite.path.display().to_string()).size(FAVORITES_PATH_SIZE));
            let row = widget::Row::new()
                .spacing(8)
                .align_y(Alignment::Center)
                .push(names)
                .push(buttons);
            let highlighted = self.favorite_cursor == Some(index);
            saved = saved.push(
                // A click on the row (not its buttons) highlights it.
                widget::mouse_area(widget::container(row).padding([4, 8]).class(
                    cosmic::theme::Container::custom(move |theme| {
                        highlight_style(theme, highlighted)
                    }),
                ))
                .on_press(Message::FocusFavorite(index)),
            );
        }

        let current = self.pane(self.active_pane).current_dir();
        let add = widget::Column::new()
            .spacing(8)
            .push(widget::text::caption_heading(fl!("favorites-add-current")))
            .push(widget::text(current.display().to_string()).size(FAVORITES_PATH_SIZE))
            .push(
                widget::Row::new()
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(
                        widget::text_input(tab_label(&current), self.favorite_name.as_str())
                            .id(self.favorite_name_input.clone())
                            .on_input(Message::FavoriteNameChanged)
                            .on_submit(|_| Message::AddFavorite)
                            .size(FAVORITES_NAME_SIZE)
                            .width(Length::Fill),
                    )
                    .push(
                        widget::button::suggested(fl!("favorites-add"))
                            .id(self.favorite_add_button.clone())
                            .on_press(Message::AddFavorite),
                    )
                    // Close sits right of Add, in Tab order.
                    .push(
                        widget::button::standard(fl!("close"))
                            .id(self.favorite_close_button.clone())
                            .on_press(Message::CloseDrawer),
                    ),
            );

        let content = widget::Column::new()
            .spacing(12)
            .push(widget::text::title4(fl!("favorites")))
            .push(widget::text::caption_heading(fl!("favorites-saved")))
            .push(
                widget::container(widget::scrollable(saved)).max_height(FAVORITES_LIST_MAX_HEIGHT),
            )
            .push(widget::divider::horizontal::light())
            .push(add);

        widget::dialog()
            .width(Length::Fixed(FAVORITES_DIALOG_WIDTH))
            .control(content)
            .into()
    }

    /// Files dropped into `dest` (drag and drop, from 2fip or another app):
    /// copied, or moved if the drag asked for it or Shift is held. Files
    /// already in `dest`, and folders dropped into themselves, are skipped.
    fn drop_files(&mut self, dest: PathBuf, paths: Vec<PathBuf>, is_move: bool) -> Task<Message> {
        if self.operation.is_some() {
            return Task::none();
        }
        let sources = drop_sources(&dest, paths);
        if sources.is_empty() {
            return Task::none();
        }
        if is_move || self.modifiers.shift() {
            self.spawn_operation(OpKind::Move, |cancel, conflict| {
                fs_ops::ops::move_paths(sources, dest, cancel, conflict)
            })
        } else {
            self.spawn_operation(OpKind::Copy, |cancel, conflict| {
                fs_ops::ops::copy(sources, dest, cancel, conflict)
            })
        }
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
    const APP_ID: &'static str = "io.github.gonzaloism.TwoFip";

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
        // Settings saved under the app's old name (Cosmic Commander) move to
        // ~/.config/2fip the first time 2fip starts.
        fs_ops::settings::migrate_legacy_config();
        let settings = fs_ops::settings::load();
        // Before anything is drawn: every text is looked up in this language.
        localize::set_language(&settings.language);
        let user_dirs = Arc::new(fs_ops::user_dirs::load());
        let options = ListingOptions {
            hide_hidden: settings.hide_hidden_files,
            icon_style: settings.icon_style,
            user_dirs: user_dirs.clone(),
            show_thumbnails: settings.show_thumbnails,
            font_size: settings.name_font_size,
            separate_extension: settings.separate_extension,
        };
        // Reopen the panes as they were when the app was last used.
        let session = fs_ops::session::load();
        let (left_dirs, left_active) = session.left.restore(&home);
        let (right_dirs, right_active) = session.right.restore(&home);
        let (left, left_task) =
            PaneState::new(PaneId::Left, left_dirs, left_active, options.clone());
        let (right, right_task) = PaneState::new(PaneId::Right, right_dirs, right_active, options);
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
            favorite_focus: FavoriteFocus::List,
            find: None,
            favorite_add_button: widget::Id::unique(),
            favorite_close_button: widget::Id::unique(),
            favorite_name_input: widget::Id::unique(),
            settings_cursor: 0,
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
            user_dirs,
            drawer: None,
            saved_session: session,
            modifiers: Modifiers::empty(),
        };
        // A theme of 2fip's own replaces the desktop's from the first frame on
        // ("System" is libcosmic's default, nothing to do).
        let theme_task = match app.settings.color_theme {
            ColorTheme::System => Task::none(),
            _ => app.apply_color_theme(),
        };
        (app, Task::batch(vec![left_task, right_task, theme_task]))
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
        menu_bar(
            &self.keybinds,
            self.clipboard.is_some(),
            self.pane(self.active_pane).view_mode(),
        )
    }

    fn context_drawer(&self) -> Option<ContextDrawer<'_, Message>> {
        let drawer = match self.drawer? {
            DrawerPage::Settings => {
                context_drawer::context_drawer(self.settings_page(), Message::CloseDrawer)
                    .title(fl!("settings"))
            }
            DrawerPage::Favorites => return None,
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
        if self.find.is_some() {
            self.close_find();
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
            let body = fl!(
                "conflict-body",
                existing = tab_label(dest),
                new = tab_label(src)
            );
            let [skip_all_id, replace_all_id, skip_id, replace_id] = self.conflict_buttons.clone();
            let all_buttons = widget::Row::new()
                .spacing(8)
                .push(
                    widget::button::standard(fl!("skip-all"))
                        .id(skip_all_id)
                        .on_press(Message::ResolveConflict(ConflictResolution::SkipAll)),
                )
                .push(
                    widget::button::standard(fl!("replace-all"))
                        .id(replace_all_id)
                        .on_press(Message::ResolveConflict(ConflictResolution::ReplaceAll)),
                );
            return Some(
                widget::dialog()
                    .title(fl!("conflict-title"))
                    .body(body)
                    .control(all_buttons)
                    .primary_action(
                        widget::button::destructive(fl!("replace"))
                            .id(replace_id)
                            .on_press(Message::ResolveConflict(ConflictResolution::Replace)),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("skip"))
                            .id(skip_id)
                            .on_press(Message::ResolveConflict(ConflictResolution::Skip)),
                    )
                    .into(),
            );
        }

        if let Some(compress) = &self.compress {
            let body = match compress.sources.as_slice() {
                [single] => fl!("compress-one", name = tab_label(single)),
                many => fl!("compress-many", count = many.len()),
            };
            return Some(
                widget::dialog()
                    .title(fl!("compress"))
                    .body(body)
                    .control(
                        widget::text_input(fl!("archive-name"), compress.name.as_str())
                            .id(self.dialog_input.clone())
                            .on_input(Message::CompressNameChanged)
                            .on_submit(|_| Message::ConfirmCompress),
                    )
                    .primary_action(
                        widget::button::suggested(fl!("compress"))
                            .id(self.dialog_confirm.clone())
                            .on_press(Message::ConfirmCompress),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("cancel"))
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

        if let Some(find) = &self.find {
            return Some(crate::find::view(find));
        }

        if self.drawer == Some(DrawerPage::Favorites) {
            return Some(self.favorites_dialog());
        }

        if let Some(new_folder) = &self.new_folder {
            return Some(
                widget::dialog()
                    .title(fl!("new-folder"))
                    .control(
                        widget::text_input(fl!("folder-name"), new_folder.name.as_str())
                            .id(self.dialog_input.clone())
                            .on_input(Message::NewFolderInputChanged)
                            .on_submit(|_| Message::ConfirmNewFolder),
                    )
                    .primary_action(
                        widget::button::suggested(fl!("create"))
                            .id(self.dialog_confirm.clone())
                            .on_press(Message::ConfirmNewFolder),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("cancel"))
                            .id(self.dialog_cancel.clone())
                            .on_press(Message::CancelNewFolder),
                    )
                    .into(),
            );
        }

        if let Some(rename) = &self.rename {
            return Some(
                widget::dialog()
                    .title(fl!("rename"))
                    .body(fl!("renaming", name = tab_label(&rename.path)))
                    .control(
                        widget::text_input(fl!("new-name"), rename.name.as_str())
                            .id(self.dialog_input.clone())
                            .on_input(Message::RenameInputChanged)
                            .on_submit(|_| Message::ConfirmRename),
                    )
                    .primary_action(
                        widget::button::suggested(fl!("rename"))
                            .id(self.dialog_confirm.clone())
                            .on_press(Message::ConfirmRename),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("cancel"))
                            .id(self.dialog_cancel.clone())
                            .on_press(Message::CancelRename),
                    )
                    .into(),
            );
        }

        if let Some(sources) = &self.confirm_delete {
            let body = match sources.as_slice() {
                [single] => fl!("delete-one", name = tab_label(single)),
                many => fl!("delete-many", count = many.len()),
            };
            return Some(
                widget::dialog()
                    .title(fl!("delete"))
                    .body(body)
                    .primary_action(
                        widget::button::destructive(fl!("delete"))
                            .id(self.confirm_delete_button.clone())
                            .on_press(Message::ConfirmDelete),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("cancel"))
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
            Some(_) => op.kind.failed_label(),
            None => fl!(
                "op-progress",
                operation = op.kind.label(),
                percent = percent
            ),
        };
        let mut amounts = Vec::new();
        if op.files_total > 0 {
            amounts.push(fl!(
                "op-files-done",
                done = op.files_done,
                total = op.files_total
            ));
        }
        if op.bytes_total > 0 {
            amounts.push(fl!(
                "op-bytes-done",
                done = format_size(op.bytes_done),
                total = format_size(op.bytes_total)
            ));
        }
        let detail = match (&op.error, &op.current_file) {
            (Some(err), _) => err.clone(),
            (None, Some(file)) => format!("{}  —  {}", amounts.join("  ·  "), tab_label(file)),
            (None, None) => fl!("op-preparing"),
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
                widget::button::standard(if failed { fl!("close") } else { fl!("cancel") })
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
            .push(pane_frame(
                left,
                PaneId::Left,
                self.active_pane == PaneId::Left,
            ))
            .push(pane_frame(
                right,
                PaneId::Right,
                self.active_pane == PaneId::Right,
            ))
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
    // "F2" etc. stay as they are; only the action's name is translated.
    let key_button = |key: &str, label: String, action: Action| {
        let label = format!("{key} {label}");
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
        .push(key_button("F2", fl!("fkey-rename"), Action::Rename))
        .push(key_button("F3", fl!("fkey-view"), Action::View))
        .push(key_button("F4", fl!("fkey-edit"), Action::Edit))
        .push(key_button("F5", fl!("fkey-copy"), Action::Copy))
        .push(key_button("F6", fl!("fkey-move"), Action::Move))
        .push(key_button("F7", fl!("fkey-mkdir"), Action::NewFolder))
        .push(key_button("F8", fl!("fkey-delete"), Action::Delete))
        .push(key_button("F9", fl!("fkey-terminal"), Action::Terminal))
        .push(key_button("Alt+F4", fl!("fkey-exit"), Action::Quit))
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
        [single] => fl!("open-with-one", name = tab_label(single)),
        many => fl!("open-with-many", count = many.len()),
    };

    let control: Element<'_, Message> = match &state.apps {
        None => widget::text(fl!("loading-apps")).into(),
        Some(apps) if apps.is_empty() => widget::text(fl!("no-apps")).into(),
        Some(apps) => {
            let mut list = widget::Column::new().spacing(2);
            let mut shown_recommended_header = false;
            let mut shown_other_header = false;
            for (index, app) in apps.iter().enumerate() {
                if app.recommended && !shown_recommended_header {
                    list = list.push(widget::text::heading(fl!("recommended-apps")));
                    shown_recommended_header = true;
                } else if !app.recommended && !app.is_default && !shown_other_header {
                    list = list.push(widget::text::heading(fl!("other-apps")));
                    shown_other_header = true;
                }
                let mut row = widget::Row::new()
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(widget::icon(app.icon.clone()).size(24))
                    .push(widget::text(app.name.clone()).width(Length::Fill));
                if app.is_default {
                    row = row.push(widget::text::caption(fl!("default-app")));
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
        .title(fl!("open-with"))
        .body(body)
        .control(control)
        .secondary_action(widget::button::standard(fl!("cancel")).on_press(Message::CancelOpenWith))
        .into()
}

fn details_dialog(state: &DetailsState) -> Element<'_, Message> {
    let mut rows: Vec<(String, String)> = Vec::new();
    let location = state
        .paths
        .first()
        .and_then(|path| path.parent())
        .map(|parent| parent.display().to_string())
        .unwrap_or_default();

    let title = match state.paths.as_slice() {
        [single] => tab_label(single),
        many => fl!("details-items-title", count = many.len()),
    };

    match &state.result {
        None => rows.push((fl!("details-size-label"), fl!("details-calculating"))),
        Some(Err(err)) => rows.push((fl!("details-error"), err.clone())),
        Some(Ok(details)) => {
            let contents = if details.contained_files + details.contained_dirs > 0 {
                format!(
                    " — {}",
                    fl!(
                        "details-contents",
                        files = details.contained_files,
                        folders = details.contained_dirs
                    )
                )
            } else {
                String::new()
            };
            let size = format!(
                "{}{contents}",
                fl!(
                    "details-size",
                    size = format_size(details.total_size),
                    bytes = details.total_size
                )
            );

            match &details.single {
                None => {
                    rows.push((fl!("details-items"), details.item_count.to_string()));
                    rows.push((fl!("details-location"), location));
                    rows.push((fl!("details-total-size"), size));
                }
                Some(entry) => {
                    let kind = match entry.kind {
                        EntryKind::Dir => fl!("kind-folder"),
                        EntryKind::File => fl!("kind-file"),
                        EntryKind::Symlink => fl!("kind-symlink"),
                    };
                    let kind = match &entry.mime_type {
                        Some(mime) => format!("{kind} ({mime})"),
                        None => kind.to_string(),
                    };
                    rows.push((fl!("details-name"), tab_label(&entry.path)));
                    rows.push((fl!("details-type"), kind));
                    rows.push((fl!("details-location"), location));
                    if let Some(target) = &entry.symlink_target {
                        rows.push((fl!("details-link-target"), target.display().to_string()));
                    }
                    rows.push((fl!("details-size-label"), size));
                    let time = |t: Option<std::time::SystemTime>| {
                        t.map(format_modified)
                            .unwrap_or_else(|| fl!("details-unknown"))
                    };
                    rows.push((fl!("details-modified"), time(entry.modified)));
                    rows.push((fl!("details-accessed"), time(entry.accessed)));
                    rows.push((fl!("details-created"), time(entry.created)));
                    rows.push((
                        fl!("details-permissions"),
                        format!(
                            "{} ({:o})",
                            fs_ops::details::format_mode(entry.mode),
                            entry.mode
                        ),
                    ));
                    rows.push((fl!("details-owner"), entry.owner.clone()));
                    rows.push((fl!("details-group"), entry.group.clone()));
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
        .primary_action(widget::button::standard(fl!("close")).on_press(Message::CloseDetails))
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
        _ => fl!("default-archive-name"),
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
/// A click anywhere in it (even on empty space) makes it the active pane;
/// files, buttons, etc. handle their own clicks first.
fn pane_frame(content: Element<'_, Message>, id: PaneId, is_active: bool) -> Element<'_, Message> {
    let frame = widget::container(content)
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
        }));
    widget::mouse_area(frame)
        .on_press(Message::ActivatePane(id))
        .on_right_press(Message::ActivatePane(id))
        .into()
}

/// The dropped `paths` that can go into `dest`: not the ones already in it,
/// and not a folder into itself or one of its own subfolders.
fn drop_sources(dest: &std::path::Path, paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths
        .into_iter()
        .filter(|path| path.parent() != Some(dest) && !dest.starts_with(path))
        .collect()
}

/// Takes the keyboard focus away from whichever text field has it.
fn unfocus_text_fields() -> Task<Message> {
    cosmic::iced::runtime::task::widget(cosmic::iced::core::widget::operation::focusable::unfocus())
}

/// The keyboard highlight for a list row: the accent color, like a selected
/// file in the listing.
pub(crate) fn highlight_style(
    theme: &cosmic::Theme,
    highlighted: bool,
) -> widget::container::Style {
    if !highlighted {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepping_through_options_wraps_around() {
        assert_eq!(step_index(0, 1, 3), 1);
        assert_eq!(step_index(2, 1, 3), 0);
        assert_eq!(step_index(0, -1, 3), 2);
        assert_eq!(step_index(1, -1, 3), 0);
    }

    #[test]
    fn drops_skip_files_already_there_and_folders_into_themselves() {
        let dest = std::path::Path::new("/home/me/docs");
        let dropped = vec![
            PathBuf::from("/home/me/docs/a.txt"),  // already in docs
            PathBuf::from("/home/me/docs"),        // docs into itself
            PathBuf::from("/home/me"),             // parent into its child
            PathBuf::from("/home/me/music/b.mp3"), // fine
            PathBuf::from("/tmp/c"),               // fine
        ];
        assert_eq!(
            drop_sources(dest, dropped),
            [
                PathBuf::from("/home/me/music/b.mp3"),
                PathBuf::from("/tmp/c")
            ]
        );
    }

    #[test]
    fn every_settings_row_is_listed_once() {
        for row in SETTINGS_ROWS {
            assert_eq!(SETTINGS_ROWS.iter().filter(|r| **r == row).count(), 1);
        }
    }
}
