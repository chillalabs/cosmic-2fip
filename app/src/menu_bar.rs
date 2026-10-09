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

/// The header's left side: the File / Edit / View / Help menus, a separator, and
/// the Refresh and Find buttons.
pub fn menu_bar<'a>(
    keybinds: &HashMap<KeyBind, Action>,
    can_paste: bool,
    view_mode: ViewMode,
    on_server: bool,
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
        button(
            fl!("connect-to-server"),
            "network-server-symbolic",
            Action::ConnectToServer,
        ),
        button(
            fl!("connections"),
            "network-workgroup-symbolic",
            Action::Connections,
        ),
        if on_server {
            button(
                fl!("disconnect"),
                "network-offline-symbolic",
                Action::Disconnect,
            )
        } else {
            MenuItem::ButtonDisabled(
                fl!("disconnect"),
                Some(icon::from_name("network-offline-symbolic").handle()),
                TopMenuAction(Action::Disconnect),
            )
        },
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
        button(
            fl!("delete-permanently"),
            "edit-delete-symbolic",
            Action::DeletePermanently,
        ),
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
    ];

    let help = vec![
        button(fl!("help"), "help-browser-symbolic", Action::Help),
        MenuItem::Divider,
        button(fl!("about-2fip"), "help-about-symbolic", Action::About),
    ];

    let bar = menu::bar(vec![
        menu::Tree::with_children(root(fl!("menu-file")), menu::items(&keybinds, file)),
        menu::Tree::with_children(root(fl!("menu-edit")), menu::items(&keybinds, edit)),
        menu::Tree::with_children(root(fl!("menu-view")), menu::items(&keybinds, view)),
        menu::Tree::with_children(root(fl!("menu-help")), menu::items(&keybinds, help)),
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
        .class(cosmic::theme::Container::custom(tooltip_style))
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
        tool(
            "network-server-symbolic",
            fl!("tooltip-connections"),
            Action::Connections,
        )
        .into(),
    ]
}

/// libcosmic's tooltip look, plus a readable text color. Its own style leaves
/// the text color to the surroundings (the title bar's, here), and its
/// background isn't tied to the theme's text color: with COSMIC's light theme
/// the background is near-black while the text is dark. So the text color is
/// picked from the background itself.
fn tooltip_style(theme: &cosmic::Theme) -> widget::container::Style {
    let cosmic = theme.cosmic();
    let background = cosmic.palette.neutral_2;
    let text = contrasting_text(background.red, background.green, background.blue);
    widget::container::Style {
        icon_color: Some(text),
        text_color: Some(text),
        background: Some(cosmic::iced::Background::Color(background.into())),
        border: cosmic::iced::Border {
            radius: cosmic.corner_radii.radius_l.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Near-white text on a dark background, near-black on a light one
/// (by the background's relative luminance; channels are sRGB, 0–1).
fn contrasting_text(red: f32, green: f32, blue: f32) -> cosmic::iced::Color {
    let linear = |c: f32| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue);
    // 0.18: where white and black text have equal contrast (WCAG).
    if luminance < 0.18 {
        cosmic::iced::Color::from_rgb(0.95, 0.95, 0.95)
    } else {
        cosmic::iced::Color::from_rgb(0.08, 0.08, 0.08)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_text_contrasts_with_its_background() {
        // Near-black (what the light theme shows) and the dark palette's #161616.
        assert!(contrasting_text(0.1, 0.1, 0.1).r > 0.9);
        assert!(contrasting_text(0.086, 0.086, 0.086).r > 0.9);
        // The light palette's #BEBEBE and white.
        assert!(contrasting_text(0.745, 0.745, 0.745).r < 0.1);
        assert!(contrasting_text(1.0, 1.0, 1.0).r < 0.1);
    }
}
