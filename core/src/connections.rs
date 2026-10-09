//! Saved server connections (the Connections panel): name, protocol,
//! server, port and user in `~/.config/2fip/connections.json`. Passwords are
//! never written there: when the user asks to remember one, it goes to the
//! system keyring (Secret Service: GNOME Keyring, KWallet; the Secret portal
//! inside the Flatpak), encrypted and unlocked with the user's login.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::vfs::{ConnectParams, Protocol, RemoteLocation};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedConnection {
    /// Stable, unique: keys the password in the keyring.
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub user: String,
    /// Whether a password is kept in the keyring for it.
    #[serde(default)]
    pub remember_password: bool,
}

impl SavedConnection {
    /// The connection's root path (`sftp://user@host:22`), which also names
    /// its open connection in [`crate::vfs`].
    pub fn root(&self) -> PathBuf {
        self.params(None).location_root()
    }

    /// What to connect with; `password` from the keyring or the user.
    pub fn params(&self, password: Option<String>) -> ConnectParams {
        ConnectParams {
            protocol: self.protocol,
            host: self.host.clone(),
            port: self.port,
            user: self.user.clone(),
            password,
            trust_new_host_key: false,
        }
    }

    /// `sftp://user@host:22`, for showing under the name.
    pub fn address(&self) -> String {
        self.root().display().to_string()
    }
}

/// A new unique ID for a saved connection.
pub fn new_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos());
    format!("{nanos:x}-{:x}", std::process::id())
}

/// The saved connections; empty if there are none or the file is unreadable.
pub fn load() -> Vec<SavedConnection> {
    load_from(&connections_path())
}

pub fn save(connections: &[SavedConnection]) -> Result<(), String> {
    save_to(&connections_path(), connections)
}

fn load_from(path: &Path) -> Vec<SavedConnection> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn save_to(path: &Path, connections: &[SavedConnection]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(connections)
        .map_err(|err| format!("failed to serialize connections: {err}"))?;
    fs::write(path, json).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn connections_path() -> PathBuf {
    crate::settings::config_dir().join("connections.json")
}

/// The keyring attributes that find a connection's password.
#[cfg(unix)]
fn attributes(id: &str) -> [(&'static str, &str); 2] {
    [
        ("application", "io.github.chillalabs.TwoFip"),
        ("connection", id),
    ]
}

/// Stores the password of the connection `id` in the system keyring,
/// replacing an older one. `label` is what keyring managers (e.g. Seahorse)
/// show.
#[cfg(unix)]
pub async fn store_password(id: &str, label: &str, password: &str) -> Result<(), String> {
    let keyring = oo7::Keyring::new().await.map_err(keyring_error)?;
    keyring.unlock().await.map_err(keyring_error)?;
    keyring
        .create_item(label, &attributes(id), password, true)
        .await
        .map_err(keyring_error)
}

/// The password stored for the connection `id`, if any.
#[cfg(unix)]
pub async fn load_password(id: &str) -> Result<Option<String>, String> {
    let keyring = oo7::Keyring::new().await.map_err(keyring_error)?;
    keyring.unlock().await.map_err(keyring_error)?;
    let items = keyring
        .search_items(&attributes(id))
        .await
        .map_err(keyring_error)?;
    let Some(item) = items.first() else {
        return Ok(None);
    };
    item.unlock().await.map_err(keyring_error)?;
    let secret = item.secret().await.map_err(keyring_error)?;
    Ok(Some(
        String::from_utf8_lossy(secret.as_bytes()).into_owned(),
    ))
}

/// Forgets the password of the connection `id` (no error if there's none).
#[cfg(unix)]
pub async fn delete_password(id: &str) -> Result<(), String> {
    let keyring = oo7::Keyring::new().await.map_err(keyring_error)?;
    keyring.unlock().await.map_err(keyring_error)?;
    keyring.delete(&attributes(id)).await.map_err(keyring_error)
}

#[cfg(unix)]
fn keyring_error(err: oo7::Error) -> String {
    format!("system keyring: {err}")
}

// No system keyring outside Linux yet (Windows: Credential Manager, later),
// so passwords are only kept for the session.
#[cfg(not(unix))]
pub async fn store_password(_id: &str, _label: &str, _password: &str) -> Result<(), String> {
    Err("no system keyring on this platform yet".to_string())
}

#[cfg(not(unix))]
pub async fn load_password(_id: &str) -> Result<Option<String>, String> {
    Ok(None)
}

#[cfg(not(unix))]
pub async fn delete_password(_id: &str) -> Result<(), String> {
    Ok(())
}

/// Whether the active location `path` belongs to `connection`.
pub fn is_on(connection: &SavedConnection, path: &Path) -> bool {
    RemoteLocation::parse(path)
        .is_some_and(|location| connection.root() == Path::new(&location.root()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SavedConnection {
        SavedConnection {
            id: "abc".into(),
            name: "Work server".into(),
            protocol: Protocol::Sftp,
            host: "example.com".into(),
            port: 2222,
            user: "me".into(),
            remember_password: true,
        }
    }

    #[test]
    fn round_trips_without_passwords() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/connections.json");
        assert!(load_from(&path).is_empty());
        save_to(&path, &[sample()]).unwrap();
        assert_eq!(load_from(&path), vec![sample()]);
        let json = fs::read_to_string(&path).unwrap();
        assert!(json.contains("\"protocol\": \"sftp\""));
        assert!(!json.contains("\"password\""), "no password field: {json}");
    }

    #[test]
    fn knows_its_address_and_paths() {
        let connection = sample();
        assert_eq!(connection.address(), "sftp://me@example.com:2222");
        assert!(is_on(
            &connection,
            Path::new("sftp://me@example.com:2222/home/me")
        ));
        assert!(!is_on(
            &connection,
            Path::new("sftp://other@example.com:2222/")
        ));
        assert!(!is_on(&connection, Path::new("/home/me")));
    }

    #[tokio::test]
    #[ignore = "uses the real system keyring"]
    async fn stores_reads_and_forgets_a_password() {
        let id = format!("test-{}", new_id());
        store_password(&id, "2fip test password (safe to delete)", "s3cret")
            .await
            .unwrap();
        assert_eq!(load_password(&id).await.unwrap().as_deref(), Some("s3cret"));
        delete_password(&id).await.unwrap();
        assert_eq!(load_password(&id).await.unwrap(), None);
    }
}
