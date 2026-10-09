//! The "Connect to server" dialog (File → Connect to server, Ctrl+K, and
//! the Connections panel's Add/Edit): opens an SFTP, FTP or FTPS location
//! in the active panel, through 2fip's own connections (see `fs_ops::vfs`),
//! not a system mount. It can also save the connection, with its password
//! in the system keyring.

use std::path::Path;

use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;
use fs_ops::connections::SavedConnection;
use fs_ops::vfs::{ConnectError, ConnectParams, Protocol, RemoteLocation};

use crate::app::Message;
use crate::fl;

const DIALOG_WIDTH: f32 = 520.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectStatus {
    Editing,
    Connecting,
    Failed(String),
    /// SFTP: a server not in `~/.ssh/known_hosts`; the user checks the
    /// fingerprint and trusts it, or cancels.
    ConfirmHostKey {
        fingerprint: String,
    },
    /// SFTP: the server's key differs from the saved one. Never connects.
    HostKeyChanged {
        fingerprint: String,
    },
}

/// The dialog's fields, in Tab order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Host,
    Port,
    User,
    Password,
}

const FIELDS: [Field; 5] = [
    Field::Name,
    Field::Host,
    Field::Port,
    Field::User,
    Field::Password,
];

pub struct ConnectState {
    /// Shown in the Connections panel; empty: "user@host".
    pub name: String,
    /// Index into [`Protocol::ALL`].
    pub protocol: usize,
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub password_hidden: bool,
    pub status: ConnectStatus,
    pub focus: Field,
    /// Keep it in the Connections panel.
    pub save: bool,
    /// Keep the password in the system keyring (with `save`).
    pub remember_password: bool,
    /// The saved connection being edited (its ID).
    pub editing: Option<String>,
    /// The edited connection has a password in the keyring already: an
    /// empty password field keeps it.
    pub has_stored_password: bool,
    ids: [widget::Id; 5],
}

impl ConnectState {
    /// A new dialog, filled in from `current` if that's a server location
    /// (e.g. to reconnect after 2fip restarted).
    pub fn new(current: &Path) -> Self {
        let mut state = ConnectState {
            name: String::new(),
            protocol: 0,
            host: String::new(),
            port: Protocol::Sftp.default_port().to_string(),
            user: String::new(),
            password: String::new(),
            password_hidden: true,
            status: ConnectStatus::Editing,
            focus: Field::Host,
            save: false,
            remember_password: false,
            editing: None,
            has_stored_password: false,
            ids: std::array::from_fn(|_| widget::Id::unique()),
        };
        if let Some(location) = RemoteLocation::parse(current) {
            state.protocol = Protocol::ALL
                .iter()
                .position(|protocol| *protocol == location.protocol)
                .unwrap_or(0);
            state.host = location.host;
            state.port = location.port.to_string();
            state.user = location.user;
        }
        state
    }

    /// The dialog for a new saved connection (Connections → Add).
    pub fn new_saved() -> Self {
        let mut state = ConnectState::new(Path::new("/"));
        state.save = true;
        state.focus = Field::Name;
        state
    }

    /// The dialog for editing a saved connection.
    pub fn for_saved(connection: &SavedConnection) -> Self {
        let mut state = ConnectState::new(&connection.root());
        state.name = connection.name.clone();
        state.save = true;
        state.remember_password = connection.remember_password;
        state.has_stored_password = connection.remember_password;
        state.editing = Some(connection.id.clone());
        state.focus = Field::Name;
        state
    }

    /// The connection to save, under `id`.
    pub fn saved(&self, id: String) -> Result<SavedConnection, String> {
        let params = self.params(false)?;
        let name = match self.name.trim() {
            "" if params.user.is_empty() => params.host.clone(),
            "" => format!("{}@{}", params.user, params.host),
            name => name.to_string(),
        };
        Ok(SavedConnection {
            id,
            name,
            protocol: params.protocol,
            host: params.host,
            port: params.port,
            user: params.user,
            remember_password: self.remember_password,
        })
    }

    pub fn protocol(&self) -> Protocol {
        Protocol::ALL[self.protocol]
    }

    /// Switches the protocol, and the port with it unless the user typed
    /// a non-default one.
    pub fn set_protocol(&mut self, index: usize) {
        let old_default = self.protocol().default_port().to_string();
        self.protocol = index.min(Protocol::ALL.len() - 1);
        if self.port.trim().is_empty() || self.port.trim() == old_default {
            self.port = self.protocol().default_port().to_string();
        }
    }

    /// What to connect with; `Err` names the problem for the user.
    pub fn params(&self, trust_new_host_key: bool) -> Result<ConnectParams, String> {
        let host = self.host.trim();
        if host.is_empty() {
            return Err(fl!("connect-error-host"));
        }
        let port = match self.port.trim() {
            "" => self.protocol().default_port(),
            port => port.parse().map_err(|_| fl!("connect-error-port"))?,
        };
        Ok(ConnectParams {
            protocol: self.protocol(),
            host: host.to_string(),
            port,
            user: self.user.trim().to_string(),
            password: (!self.password.is_empty()).then(|| self.password.clone()),
            trust_new_host_key,
        })
    }

    /// What a failed attempt means for the dialog.
    pub fn failed(&mut self, error: ConnectError) {
        self.status = match error {
            ConnectError::UnknownHostKey { fingerprint } => {
                ConnectStatus::ConfirmHostKey { fingerprint }
            }
            ConnectError::HostKeyChanged { fingerprint } => {
                ConnectStatus::HostKeyChanged { fingerprint }
            }
            ConnectError::AuthenticationFailed => ConnectStatus::Failed(fl!("connect-error-login")),
            ConnectError::Other(message) => ConnectStatus::Failed(message),
        };
    }

    /// Moves the keyboard focus `step` fields (Tab / Shift+Tab).
    pub fn move_focus(&mut self, step: isize) -> cosmic::app::Task<Message> {
        let current = FIELDS
            .iter()
            .position(|field| *field == self.focus)
            .unwrap_or(0) as isize;
        let next = (current + step).rem_euclid(FIELDS.len() as isize) as usize;
        self.focus = FIELDS[next];
        self.focus_current()
    }

    pub fn focus_current(&self) -> cosmic::app::Task<Message> {
        let index = FIELDS
            .iter()
            .position(|field| *field == self.focus)
            .unwrap_or(0);
        widget::text_input::focus(self.ids[index].clone())
    }

    fn id(&self, field: Field) -> widget::Id {
        let index = FIELDS
            .iter()
            .position(|candidate| *candidate == field)
            .unwrap_or(0);
        self.ids[index].clone()
    }
}

/// The protocol names for the dropdown, in [`Protocol::ALL`] order.
pub fn protocol_names() -> Vec<String> {
    Protocol::ALL
        .iter()
        .map(|protocol| match protocol {
            Protocol::Sftp => "SFTP (SSH)".to_string(),
            Protocol::Ftp => "FTP".to_string(),
            Protocol::Ftps => "FTPS (FTP + TLS)".to_string(),
        })
        .collect()
}

pub fn view<'a>(state: &'a ConnectState, protocols: &'a [String]) -> Element<'a, Message> {
    match &state.status {
        ConnectStatus::ConfirmHostKey { fingerprint } => {
            return widget::dialog()
                .title(fl!("connect-unknown-host-title"))
                .body(fl!(
                    "connect-unknown-host-body",
                    host = state.host.trim().to_string()
                ))
                .control(
                    widget::text(fingerprint.as_str())
                        .font(cosmic::font::mono())
                        .size(13),
                )
                .primary_action(
                    widget::button::suggested(fl!("connect-trust")).on_press(Message::ConnectTrust),
                )
                .secondary_action(
                    widget::button::standard(fl!("cancel")).on_press(Message::ConnectCancel),
                )
                .into();
        }
        ConnectStatus::HostKeyChanged { fingerprint } => {
            return widget::dialog()
                .title(fl!("connect-key-changed-title"))
                .body(fl!(
                    "connect-key-changed-body",
                    host = state.host.trim().to_string()
                ))
                .control(
                    widget::text(fingerprint.as_str())
                        .font(cosmic::font::mono())
                        .size(13),
                )
                .primary_action(
                    widget::button::standard(fl!("close")).on_press(Message::ConnectCancel),
                )
                .into();
        }
        _ => {}
    }

    let connecting = state.status == ConnectStatus::Connecting;
    let name = widget::text_input(fl!("connect-name"), state.name.as_str())
        .id(state.id(Field::Name))
        .on_input(Message::ConnectName)
        .on_submit(|_| Message::ConnectSubmit);
    let server_row = widget::Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .push(widget::dropdown(
            protocols,
            Some(state.protocol),
            Message::ConnectProtocol,
        ))
        .push(
            widget::text_input(fl!("connect-host"), state.host.as_str())
                .id(state.id(Field::Host))
                .on_input(Message::ConnectHost)
                .on_submit(|_| Message::ConnectSubmit)
                .width(Length::Fill),
        )
        .push(
            widget::text_input(fl!("connect-port"), state.port.as_str())
                .id(state.id(Field::Port))
                .on_input(Message::ConnectPort)
                .on_submit(|_| Message::ConnectSubmit)
                .width(Length::Fixed(72.0)),
        );
    let user = widget::text_input(fl!("connect-user"), state.user.as_str())
        .id(state.id(Field::User))
        .on_input(Message::ConnectUser)
        .on_submit(|_| Message::ConnectSubmit);
    let password_placeholder = if state.has_stored_password {
        fl!("connect-password-stored")
    } else {
        fl!("connect-password")
    };
    let password = widget::secure_input(
        password_placeholder,
        state.password.as_str(),
        Some(Message::ConnectTogglePassword),
        state.password_hidden,
    )
    .id(state.id(Field::Password))
    .on_input(Message::ConnectPassword)
    .on_submit(|_| Message::ConnectSubmit);

    let hint = match state.protocol() {
        Protocol::Sftp => fl!("connect-hint-sftp"),
        Protocol::Ftp => fl!("connect-hint-ftp"),
        Protocol::Ftps => fl!("connect-hint-ftps"),
    };
    let status: Option<Element<'a, Message>> = match &state.status {
        ConnectStatus::Connecting => Some(widget::text::body(fl!("connect-connecting")).into()),
        ConnectStatus::Failed(message) => Some(
            widget::text::body(message.as_str())
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb(
                    0.85, 0.25, 0.25,
                )))
                .into(),
        ),
        _ => None,
    };

    // Saving: always when editing a saved connection; the password only
    // with it (it's kept under the saved connection's name).
    let save = (state.editing.is_none()).then(|| {
        widget::checkbox(state.save)
            .label(fl!("connect-save"))
            .on_toggle(Message::ConnectSave)
    });
    let remember = widget::checkbox(state.remember_password && state.save)
        .label(fl!("connect-remember-password"))
        .on_toggle_maybe(state.save.then_some(Message::ConnectRemember));

    let mut content = widget::Column::new()
        .spacing(12)
        .push_maybe(state.save.then_some(name))
        .push(server_row)
        .push(user)
        .push(password)
        .push(widget::text::caption(hint))
        .push_maybe(save)
        .push(remember);
    if let Some(status) = status {
        content = content.push(status);
    }

    let title = if state.editing.is_some() {
        fl!("connection-edit")
    } else if state.save {
        fl!("connection-add")
    } else {
        fl!("connect-title")
    };
    let ready = !connecting && !state.host.trim().is_empty();
    let mut dialog = widget::dialog()
        .title(title)
        .width(Length::Fixed(DIALOG_WIDTH))
        .control(content)
        .primary_action(
            widget::button::suggested(fl!("connect"))
                .on_press_maybe(ready.then_some(Message::ConnectSubmit)),
        )
        .secondary_action(widget::button::standard(fl!("cancel")).on_press(Message::ConnectCancel));
    if state.save {
        // Save without connecting (e.g. a server that's offline now).
        dialog = dialog.tertiary_action(
            widget::button::standard(fl!("connection-save-only"))
                .on_press_maybe(ready.then_some(Message::ConnectSaveOnly)),
        );
    }
    dialog.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_protocol_updates_a_default_port_only() {
        let mut state = ConnectState::new(Path::new("/home"));
        assert_eq!(state.port, "22");
        state.set_protocol(1); // FTP
        assert_eq!(state.port, "21");
        state.port = "2121".into();
        state.set_protocol(2); // FTPS keeps the custom port
        assert_eq!(state.port, "2121");
    }

    #[test]
    fn builds_a_saved_connection_with_a_default_name() {
        let mut state = ConnectState::new_saved();
        state.host = " files.example.com ".into();
        state.user = "me".into();
        state.remember_password = true;
        let saved = state.saved("id1".into()).unwrap();
        assert_eq!(saved.name, "me@files.example.com");
        assert_eq!(
            (saved.host.as_str(), saved.port, saved.remember_password),
            ("files.example.com", 22, true)
        );
        let edit = ConnectState::for_saved(&saved);
        assert_eq!(
            (edit.editing.as_deref(), edit.has_stored_password),
            (Some("id1"), true)
        );
    }

    #[test]
    fn prefills_from_a_server_location() {
        let state = ConnectState::new(Path::new("ftps://me@files.example.com:990/pub"));
        assert_eq!(state.protocol(), Protocol::Ftps);
        assert_eq!(
            (
                state.host.as_str(),
                state.port.as_str(),
                state.user.as_str()
            ),
            ("files.example.com", "990", "me")
        );
    }
}

/// What to do in the system keyring before connecting or saving.
#[derive(Debug, Clone, Default)]
pub struct KeyringPlan {
    /// The saved connection's ID; `None`: not saved, the keyring isn't used.
    pub id: Option<String>,
    /// What keyring managers show for the password.
    pub label: String,
    /// A new password to remember.
    pub store: Option<String>,
    /// Forget the stored password (the user unticked "Remember").
    pub forget: bool,
    /// No password typed: use the stored one.
    pub load: bool,
}

impl KeyringPlan {
    /// Stores or forgets the password as planned.
    pub async fn apply(&self) -> Result<(), String> {
        let Some(id) = &self.id else {
            return Ok(());
        };
        if let Some(password) = &self.store {
            fs_ops::connections::store_password(id, &self.label, password).await?;
        } else if self.forget {
            fs_ops::connections::delete_password(id).await?;
        }
        Ok(())
    }

    /// The stored password, when the plan says to use it.
    pub async fn stored_password(&self) -> Option<String> {
        let id = self.id.as_ref().filter(|_| self.load)?;
        fs_ops::connections::load_password(id).await.ok().flatten()
    }
}

impl ConnectState {
    /// The keyring work for this dialog's connection saved under `id`.
    pub fn keyring_plan(&self, id: Option<String>, name: &str) -> KeyringPlan {
        let Some(id) = id else {
            return KeyringPlan::default();
        };
        let typed = (!self.password.is_empty()).then(|| self.password.clone());
        KeyringPlan {
            label: format!("2fip: {name}"),
            store: typed.clone().filter(|_| self.remember_password),
            forget: !self.remember_password,
            load: self.remember_password && typed.is_none(),
            id: Some(id),
        }
    }
}
