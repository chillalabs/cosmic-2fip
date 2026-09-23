use std::collections::HashMap;

use cosmic::widget::icon;
use cosmic::widget::menu::key_bind::KeyBind;
use cosmic::widget::menu::{self, Item as MenuItem};

use crate::file_item::FileItem;
use crate::keybinds::Action;
use crate::pane::PaneMessage;

/// The right-click menu for a file or folder in the listing. Actions apply to
/// the pane's selection (right-clicking an unselected item selects it first),
/// and shortcut hints come from `keybinds`.
pub fn item_menu(
    keybinds: &HashMap<KeyBind, Action>,
    item: &FileItem,
    can_paste: bool,
) -> Option<Vec<menu::Tree<PaneMessage>>> {
    let button = |label: &'static str, icon_name: &'static str, action: Action| {
        MenuItem::Button(label, Some(icon::from_name(icon_name).handle()), action)
    };

    let mut items = vec![button("Open", "document-open-symbolic", Action::Open)];
    if !item.is_dir() {
        items.push(button(
            "Open With…",
            "system-run-symbolic",
            Action::OpenWith,
        ));
        items.push(button("View", "document-viewer-symbolic", Action::View));
        items.push(button("Edit", "accessories-text-editor-symbolic", Action::Edit));
    }
    items.extend([
        MenuItem::Divider,
        button("Cut", "edit-cut-symbolic", Action::Cut),
        button("Copy", "edit-copy-symbolic", Action::CopyToClipboard),
        if can_paste {
            button("Paste", "edit-paste-symbolic", Action::Paste)
        } else {
            MenuItem::ButtonDisabled(
                "Paste",
                Some(icon::from_name("edit-paste-symbolic").handle()),
                Action::Paste,
            )
        },
        MenuItem::Divider,
        button("Rename…", "edit-symbolic", Action::Rename),
        button("Compress…", "package-x-generic-symbolic", Action::Compress),
        MenuItem::Divider,
        button("Delete", "user-trash-symbolic", Action::Delete),
        MenuItem::Divider,
        button(
            "Show Details",
            "document-properties-symbolic",
            Action::ShowDetails,
        ),
    ]);

    Some(menu::items(keybinds, items))
}
