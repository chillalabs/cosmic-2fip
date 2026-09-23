use std::collections::HashMap;

use cosmic::widget::icon;
use cosmic::widget::menu::action::MenuAction;
use cosmic::widget::menu::key_bind::KeyBind;
use cosmic::widget::menu::{self, Item as MenuItem, ItemHeight, ItemWidth};
use cosmic::widget::RcElementWrapper;
use cosmic::Element;
use fs_ops::settings::ViewMode;

use crate::app::Message;
use crate::keybinds::Action;

/// An [`Action`] fired from the top menu bar. `Action`'s own [`MenuAction`]
/// impl produces a pane message (for the file context menu); this one
/// produces an app-level message instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopMenuAction(Action);

impl MenuAction for TopMenuAction {
    type Message = Message;

    fn message(&self) -> Message {
        Message::Action(self.0)
    }
}

/// The File / Edit / View menu bar shown in the window header.
pub fn menu_bar<'a>(
    keybinds: &HashMap<KeyBind, Action>,
    can_paste: bool,
    view_mode: ViewMode,
) -> Element<'a, Message> {
    // Same shortcuts, re-keyed to the wrapper so the menu can show them.
    let keybinds: HashMap<KeyBind, TopMenuAction> = keybinds
        .iter()
        .map(|(bind, action)| (bind.clone(), TopMenuAction(*action)))
        .collect();

    let button = |label: &'static str, icon_name: &'static str, action: Action| {
        MenuItem::Button(
            label,
            Some(icon::from_name(icon_name).handle()),
            TopMenuAction(action),
        )
    };
    let root = |label: &'static str| RcElementWrapper::new(Element::from(menu::root(label)));

    let file = vec![
        button("New Tab", "tab-new-symbolic", Action::NewTab),
        button("New Folder", "folder-new-symbolic", Action::NewFolder),
        MenuItem::Divider,
        button("Close Tab", "window-close-symbolic", Action::CloseTab),
        button("Quit", "application-exit-symbolic", Action::Quit),
    ];

    let paste = if can_paste {
        button("Paste", "edit-paste-symbolic", Action::Paste)
    } else {
        MenuItem::ButtonDisabled(
            "Paste",
            Some(icon::from_name("edit-paste-symbolic").handle()),
            TopMenuAction(Action::Paste),
        )
    };
    let edit = vec![
        button("Cut", "edit-cut-symbolic", Action::Cut),
        button("Copy", "edit-copy-symbolic", Action::CopyToClipboard),
        paste,
        MenuItem::Divider,
        button("Select All", "edit-select-all-symbolic", Action::SelectAll),
        MenuItem::Divider,
        button("Rename…", "edit-symbolic", Action::Rename),
        button("Delete", "user-trash-symbolic", Action::Delete),
    ];

    let check = |label: &'static str, icon_name: &'static str, checked: bool, action: Action| {
        MenuItem::CheckBox(
            label,
            Some(icon::from_name(icon_name).handle()),
            checked,
            TopMenuAction(action),
        )
    };
    let view = vec![
        check(
            "List View",
            "view-list-symbolic",
            view_mode == ViewMode::List,
            Action::ListView,
        ),
        check(
            "Grid View",
            "view-grid-symbolic",
            view_mode == ViewMode::Grid,
            Action::GridView,
        ),
        MenuItem::Divider,
        button("Favorites", "starred-symbolic", Action::Favorites),
        MenuItem::Divider,
        button("Settings", "preferences-system-symbolic", Action::Settings),
    ];

    menu::bar(vec![
        menu::Tree::with_children(root("File"), menu::items(&keybinds, file)),
        menu::Tree::with_children(root("Edit"), menu::items(&keybinds, edit)),
        menu::Tree::with_children(root("View"), menu::items(&keybinds, view)),
    ])
    .item_height(ItemHeight::Dynamic(40))
    .item_width(ItemWidth::Uniform(260))
    .spacing(4.0)
    .into()
}
