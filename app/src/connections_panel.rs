//! The Connections side panel (header button): saved server connections and
//! open ones, with their status, and buttons to open, reconnect, disconnect,
//! edit and delete them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;
use fs_ops::connections::SavedConnection;

use crate::app::Message;
use crate::fl;

/// A connection's state, as last seen by the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    /// Being opened or checked right now.
    Busy,
    /// Answered the last check.
    Connected,
    /// Didn't answer (dropped by the server, network gone…).
    Lost(String),
}

/// What the panel shows for a connection root (`sftp://user@host:22`).
fn status(root: &Path, checks: &HashMap<PathBuf, Check>) -> (Check, bool) {
    let connected = fs_ops::vfs::is_connected(root);
    match checks.get(root) {
        Some(check) => (check.clone(), connected),
        None if connected => (Check::Connected, true),
        None => (Check::Lost(String::new()), false),
    }
}

/// The colored dot and label of a status.
fn status_line<'a>(check: &Check, connected: bool) -> Element<'a, Message> {
    let (color, label) = match check {
        Check::Busy => ((0.95, 0.68, 0.2), fl!("connection-status-busy")),
        Check::Connected if connected => ((0.3, 0.75, 0.4), fl!("connection-status-connected")),
        Check::Lost(reason) if connected => (
            (0.9, 0.3, 0.3),
            if reason.is_empty() {
                fl!("connection-status-lost")
            } else {
                format!("{} ({reason})", fl!("connection-status-lost"))
            },
        ),
        _ => ((0.55, 0.55, 0.55), fl!("connection-status-closed")),
    };
    let dot = widget::container(widget::Space::new().width(10).height(10)).class(
        cosmic::theme::Container::custom(move |_| widget::container::Style {
            background: Some(cosmic::iced::Background::Color(
                cosmic::iced::Color::from_rgb(color.0, color.1, color.2),
            )),
            border: cosmic::iced::Border {
                radius: 5.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }),
    );
    widget::Row::new()
        .spacing(6)
        .align_y(Alignment::Center)
        .push(dot)
        .push(widget::text::caption(label))
        .into()
}

fn tool<'a>(icon: &'static str, tooltip: String, message: Option<Message>) -> Element<'a, Message> {
    widget::button::icon(widget::icon::from_name(icon))
        .tooltip(tooltip)
        .on_press_maybe(message)
        .into()
}

/// One connection: name, address, status, buttons.
fn card<'a>(
    name: String,
    address: String,
    status: Element<'a, Message>,
    buttons: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut row = widget::Row::new().spacing(2).align_y(Alignment::Center);
    for button in buttons {
        row = row.push(button);
    }
    let card = widget::Column::new()
        .spacing(4)
        .push(widget::text::heading(name))
        .push(widget::text::caption(address))
        .push(status)
        .push(row);
    widget::container(card)
        .padding(10)
        .width(Length::Fill)
        .class(cosmic::theme::Container::Card)
        .into()
}

pub fn view<'a>(
    saved: &'a [SavedConnection],
    checks: &'a HashMap<PathBuf, Check>,
) -> Element<'a, Message> {
    let add = widget::button::standard(fl!("connection-add"))
        .leading_icon(widget::icon::from_name("list-add-symbolic"))
        .on_press(Message::ConnectionAdd);

    let mut saved_list = widget::Column::new().spacing(8);
    if saved.is_empty() {
        saved_list = saved_list.push(widget::text::body(fl!("connections-empty")));
    }
    for connection in saved {
        let root = connection.root();
        let (check, connected) = status(&root, checks);
        let busy = check == Check::Busy;
        let id = connection.id.clone();
        let buttons = vec![
            tool(
                "go-next-symbolic",
                fl!("connection-open"),
                (!busy).then(|| Message::ConnectionOpen(id.clone())),
            ),
            tool(
                "view-refresh-symbolic",
                fl!("connection-reconnect"),
                (!busy).then(|| Message::ConnectionReconnect(id.clone())),
            ),
            tool(
                "media-eject-symbolic",
                fl!("disconnect"),
                (connected && !busy).then(|| Message::ConnectionDisconnect(root.clone())),
            ),
            tool(
                "document-edit-symbolic",
                fl!("connection-edit"),
                Some(Message::ConnectionEdit(id.clone())),
            ),
            tool(
                "user-trash-symbolic",
                fl!("delete"),
                Some(Message::ConnectionDelete(id)),
            ),
        ];
        saved_list = saved_list.push(card(
            connection.name.clone(),
            connection.address(),
            status_line(&check, connected),
            buttons,
        ));
    }

    // Open connections that aren't saved (e.g. made with Ctrl+K).
    let unsaved: Vec<PathBuf> = fs_ops::vfs::open_connections()
        .into_iter()
        .filter(|root| !saved.iter().any(|connection| connection.root() == *root))
        .collect();
    let mut open_list = widget::Column::new().spacing(8);
    for root in &unsaved {
        let (check, connected) = status(root, checks);
        let buttons = vec![
            tool(
                "go-next-symbolic",
                fl!("connection-open"),
                Some(Message::ConnectionOpenRoot(root.clone())),
            ),
            tool(
                "media-eject-symbolic",
                fl!("disconnect"),
                Some(Message::ConnectionDisconnect(root.clone())),
            ),
            tool(
                "document-save-symbolic",
                fl!("connection-save"),
                Some(Message::ConnectionSaveOpen(root.clone())),
            ),
        ];
        let address = root.display().to_string();
        open_list = open_list.push(card(
            address.clone(),
            address,
            status_line(&check, connected),
            buttons,
        ));
    }

    let mut content = widget::Column::new()
        .spacing(12)
        .push(add)
        .push(widget::text::caption_heading(fl!("connections-saved")))
        .push(saved_list);
    if !unsaved.is_empty() {
        content = content
            .push(widget::text::caption_heading(fl!(
                "connections-open-unsaved"
            )))
            .push(open_list);
    }
    content
        .push(widget::text::caption(fl!("connections-hint")))
        .into()
}
