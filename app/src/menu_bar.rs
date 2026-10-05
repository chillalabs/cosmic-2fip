use std::collections::HashMap;

use cosmic::iced::{Alignment, Length};
use cosmic::widget::menu::action::MenuAction;
use cosmic::widget::menu::key_bind::KeyBind;
use cosmic::widget::menu::{self, Item as MenuItem, ItemHeight, ItemWidth};
use cosmic::widget::RcElementWrapper;
use cosmic::widget::{self, icon};
use cosmic::Element;
use fs_ops::settings::ViewMode;

use crate::app::Message;
use crate::fl;
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

/// The header's left side: the File / Edit / View menus, a separator, and
/// the Refresh and Find buttons.
pub fn menu_bar<'a>(
    keybinds: &HashMap<KeyBind, Action>,
    can_paste: bool,
    view_mode: ViewMode,
) -> Vec<Element<'a, Message>> {
    // Same shortcuts, re-keyed to the wrapper so the menu can show them.
    let keybinds: HashMap<KeyBind, TopMenuAction> = keybinds
        .iter()
        .map(|(bind, action)| (bind.clone(), TopMenuAction(*action)))
        .collect();

    let button = |label: String, icon_name: &'static str, action: Action| {
        MenuItem::Button(
            label,
            Some(icon::from_name(icon_name).handle()),
            TopMenuAction(action),
        )
    };
    let root = |label: String| RcElementWrapper::new(Element::from(menu::root(label)));

    let file = vec![
        button(fl!("new-tab"), "tab-new-symbolic", Action::NewTab),
        button(fl!("new-folder"), "folder-new-symbolic", Action::NewFolder),
        MenuItem::Divider,
        button(fl!("close-tab"), "window-close-symbolic", Action::CloseTab),
        button(fl!("quit"), "application-exit-symbolic", Action::Quit),
    ];

    let paste = if can_paste {
        button(fl!("paste"), "edit-paste-symbolic", Action::Paste)
    } else {
        MenuItem::ButtonDisabled(
            fl!("paste"),
            Some(icon::from_name("edit-paste-symbolic").handle()),
            TopMenuAction(Action::Paste),
        )
    };
    let edit = vec![
        button(fl!("cut"), "edit-cut-symbolic", Action::Cut),
        button(fl!("copy"), "edit-copy-symbolic", Action::CopyToClipboard),
        paste,
        MenuItem::Divider,
        button(
            fl!("select-all"),
            "edit-select-all-symbolic",
            Action::SelectAll,
        ),
        MenuItem::Divider,
        button(fl!("rename-ellipsis"), "edit-symbolic", Action::Rename),
        button(fl!("delete"), "user-trash-symbolic", Action::Delete),
    ];

    let check = |label: String, icon_name: &'static str, checked: bool, action: Action| {
        MenuItem::CheckBox(
            label,
            Some(icon::from_name(icon_name).handle()),
            checked,
            TopMenuAction(action),
        )
    };
    let view = vec![
        check(
            fl!("list-view"),
            "view-list-symbolic",
            view_mode == ViewMode::List,
            Action::ListView,
        ),
        check(
            fl!("grid-view"),
            "view-grid-symbolic",
            view_mode == ViewMode::Grid,
            Action::GridView,
        ),
        MenuItem::Divider,
        button(fl!("favorites"), "starred-symbolic", Action::Favorites),
        MenuItem::Divider,
        button(
            fl!("settings"),
            "preferences-system-symbolic",
            Action::Settings,
        ),
        MenuItem::Divider,
        button(fl!("about-2fip"), "help-about-symbolic", Action::About),
    ];

    let bar = menu::bar(vec![
        menu::Tree::with_children(root(fl!("menu-file")), menu::items(&keybinds, file)),
        menu::Tree::with_children(root(fl!("menu-edit")), menu::items(&keybinds, edit)),
        menu::Tree::with_children(root(fl!("menu-view")), menu::items(&keybinds, view)),
    ])
    .item_height(ItemHeight::Dynamic(40))
    .item_width(ItemWidth::Uniform(260))
    .spacing(4.0);
    let separator = widget::container(widget::divider::vertical::default())
        .height(Length::Fixed(20.0))
        .padding([0, 4]);
    // The tooltip goes below: the buttons sit at the top of the window, and
    // libcosmic's default (above) would cover the title bar.
    let tool = |icon_name: &'static str, tooltip: String, action: Action| {
        widget::tooltip(
            widget::button::icon(icon::from_name(icon_name)).on_press(Message::Action(action)),
            widget::text(tooltip),
            widget::tooltip::Position::Bottom,
        )
    };

    vec![
        bar.into(),
        widget::container(separator)
            .height(Length::Fill)
            .align_y(Alignment::Center)
            .into(),
        tool(
            "view-refresh-symbolic",
            fl!("tooltip-refresh"),
            Action::Refresh,
        )
        .into(),
        tool("system-search-symbolic", fl!("tooltip-find"), Action::Find).into(),
    ]
}
