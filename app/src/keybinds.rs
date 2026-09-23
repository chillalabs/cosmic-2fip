use std::collections::HashMap;

use cosmic::iced::keyboard::key::Named;
use cosmic::iced::keyboard::Key;
use cosmic::widget::menu::action::MenuAction;
use cosmic::widget::menu::key_bind::{KeyBind, Modifier};

use crate::pane::PaneMessage;

/// Total-Commander-style actions triggerable by a global keyboard shortcut
/// and/or the file context menu.
///
/// Deliberately excludes anything dialog-local (rename text entry, delete
/// confirmation) — those are driven by their own dedicated messages since
/// they only make sense while their dialog is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    SwitchPane,
    /// Copies the selection into the *other* pane (F5).
    Copy,
    /// Moves the selection into the *other* pane (F6).
    Move,
    Delete,
    Rename,
    NewFolder,
    NewTab,
    CloseTab,
    GoBack,
    GoForward,
    /// Goes to the parent folder.
    GoUp,
    SelectNext,
    SelectPrevious,
    /// Shift+↓ / Shift+↑: grow or shrink the selection by one row.
    ExtendSelectionDown,
    ExtendSelectionUp,
    /// Home / End: first / last item; with Shift, extend the selection there.
    SelectFirst,
    SelectLast,
    ExtendSelectionToFirst,
    ExtendSelectionToLast,
    EnterSelectedFolder,
    /// Total Commander's quick filter: list only names matching typed text.
    QuickFilter,
    /// Opens the default terminal in the active pane's folder.
    Terminal,
    /// F3: opens the selected files with their default app.
    View,
    /// Ctrl+L: type a path into the active pane's path bar.
    EditPath,
    ListView,
    GridView,
    /// F4: opens the selected files in the text editor.
    Edit,
    /// Shows the active pane's next / previous tab (wrapping around).
    NextTab,
    PreviousTab,
    Favorites,
    Open,
    OpenWith,
    /// Puts the selection on the internal clipboard, to be moved on paste.
    Cut,
    /// Puts the selection on the internal clipboard, to be copied on paste.
    CopyToClipboard,
    /// Copies/moves the clipboard's contents into the active pane's folder.
    Paste,
    Compress,
    ShowDetails,
    SelectAll,
    /// Opens or closes the Settings side panel.
    Settings,
    ToggleHiddenFiles,
    Quit,
}

impl MenuAction for Action {
    type Message = PaneMessage;

    fn message(&self) -> PaneMessage {
        PaneMessage::Action(*self)
    }
}

pub fn default_keybinds() -> HashMap<KeyBind, Action> {
    let mut map = HashMap::new();

    let mut bind = |modifiers: &[Modifier], key: Key, action: Action| {
        map.insert(
            KeyBind {
                modifiers: modifiers.to_vec(),
                key,
            },
            action,
        );
    };

    bind(&[], Key::Named(Named::Tab), Action::SwitchPane);
    bind(&[], Key::Named(Named::F5), Action::Copy);
    bind(&[], Key::Named(Named::F6), Action::Move);
    bind(&[], Key::Named(Named::F8), Action::Delete);
    bind(&[], Key::Named(Named::F2), Action::Rename);
    bind(&[], Key::Named(Named::F7), Action::NewFolder);
    bind(&[Modifier::Ctrl], Key::Character("t".into()), Action::NewTab);
    bind(&[Modifier::Ctrl], Key::Character("w".into()), Action::CloseTab);
    bind(&[Modifier::Alt], Key::Named(Named::ArrowLeft), Action::GoBack);
    bind(&[Modifier::Alt], Key::Named(Named::ArrowRight), Action::GoForward);
    bind(&[], Key::Named(Named::Backspace), Action::GoUp);
    bind(&[], Key::Named(Named::ArrowLeft), Action::GoUp);
    bind(&[], Key::Named(Named::ArrowRight), Action::EnterSelectedFolder);
    bind(&[], Key::Named(Named::ArrowDown), Action::SelectNext);
    bind(&[], Key::Named(Named::ArrowUp), Action::SelectPrevious);
    bind(&[Modifier::Shift], Key::Named(Named::ArrowDown), Action::ExtendSelectionDown);
    bind(&[Modifier::Shift], Key::Named(Named::ArrowUp), Action::ExtendSelectionUp);
    bind(&[], Key::Named(Named::Home), Action::SelectFirst);
    bind(&[], Key::Named(Named::End), Action::SelectLast);
    bind(&[Modifier::Shift], Key::Named(Named::Home), Action::ExtendSelectionToFirst);
    bind(&[Modifier::Shift], Key::Named(Named::End), Action::ExtendSelectionToLast);
    bind(&[Modifier::Ctrl], Key::Character("s".into()), Action::QuickFilter);
    bind(&[], Key::Named(Named::F9), Action::Terminal);
    bind(&[], Key::Named(Named::F3), Action::View);
    bind(&[Modifier::Ctrl], Key::Character("l".into()), Action::EditPath);
    bind(&[Modifier::Ctrl], Key::Character("1".into()), Action::ListView);
    bind(&[Modifier::Ctrl], Key::Character("2".into()), Action::GridView);
    bind(&[], Key::Named(Named::F4), Action::Edit);
    // Alt+Tab belongs to the desktop's window switcher, so tabs use Ctrl+Tab
    // (as in Total Commander and web browsers).
    bind(&[Modifier::Ctrl], Key::Named(Named::Tab), Action::NextTab);
    bind(&[Modifier::Ctrl, Modifier::Shift], Key::Named(Named::Tab), Action::PreviousTab);
    // COSMIC itself closes the window on Alt+F4; this covers desktops that don't.
    bind(&[Modifier::Alt], Key::Named(Named::F4), Action::Quit);
    bind(&[Modifier::Ctrl], Key::Character("d".into()), Action::Favorites);
    bind(&[], Key::Named(Named::Delete), Action::Delete);
    bind(&[], Key::Named(Named::Enter), Action::Open);
    bind(&[Modifier::Ctrl], Key::Character("x".into()), Action::Cut);
    bind(&[Modifier::Ctrl], Key::Character("c".into()), Action::CopyToClipboard);
    bind(&[Modifier::Ctrl], Key::Character("v".into()), Action::Paste);
    bind(&[Modifier::Alt], Key::Named(Named::Enter), Action::ShowDetails);
    bind(&[Modifier::Ctrl], Key::Character("a".into()), Action::SelectAll);
    bind(&[Modifier::Ctrl], Key::Character(",".into()), Action::Settings);
    bind(&[Modifier::Ctrl], Key::Character("h".into()), Action::ToggleHiddenFiles);
    bind(&[Modifier::Ctrl], Key::Character("q".into()), Action::Quit);

    map
}
