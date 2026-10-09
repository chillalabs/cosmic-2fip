//! 2fip's virtual file system: network locations (`sftp://`, `ftp://`,
//! `ftps://`) that panels browse and operations read and write like local
//! folders, without mounting anything on the system (`gio mount`,
//! `mount.cifs`). The connections live inside 2fip, so this works the same on
//! every distribution and inside the Flatpak or AppImage.
//!
//! A remote location is a path written like a URL,
//! `sftp://user@host:22/home/user`, so it travels through the app (tabs,
//! history, selection) like any local path. [`RemoteLocation::parse`] tells
//! the two apart; local paths keep using `std::fs`.

mod ftp;
mod sftp;
pub mod transfer;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use crate::entry::{DirEntry, EntryKind};

/// The network protocols 2fip speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Sftp,
    Ftp,
    /// FTP with explicit TLS (`AUTH TLS`).
    Ftps,
}

impl Protocol {
    pub const ALL: [Protocol; 3] = [Protocol::Sftp, Protocol::Ftp, Protocol::Ftps];

    pub fn scheme(self) -> &'static str {
        match self {
            Protocol::Sftp => "sftp",
            Protocol::Ftp => "ftp",
            Protocol::Ftps => "ftps",
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            Protocol::Sftp => 22,
            Protocol::Ftp | Protocol::Ftps => 21,
        }
    }

    fn from_scheme(scheme: &str) -> Option<Protocol> {
        Protocol::ALL
            .into_iter()
            .find(|protocol| protocol.scheme().eq_ignore_ascii_case(scheme))
    }
}

/// A place on a server: `protocol://user@host:port/path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteLocation {
    pub protocol: Protocol,
    pub user: String,
    pub host: String,
    pub port: u16,
    /// Absolute, `/`-separated, without a trailing `/` (except the root).
    pub path: String,
}

impl RemoteLocation {
    /// Reads a remote path (or URL); `None` for local paths. The port and
    /// user may be left out (default port; for FTP, anonymous login).
    pub fn parse(path: &Path) -> Option<RemoteLocation> {
        let text = path.to_str()?;
        let (scheme, rest) = text.split_once("://")?;
        let protocol = Protocol::from_scheme(scheme)?;
        let (authority, path) = match rest.find('/') {
            Some(slash) => (&rest[..slash], &rest[slash..]),
            None => (rest, "/"),
        };
        let (user, host_port) = match authority.rsplit_once('@') {
            Some((user, host_port)) => (user.to_string(), host_port),
            None => (String::new(), authority),
        };
        let (host, port) = split_host_port(host_port, protocol.default_port())?;
        if host.is_empty() {
            return None;
        }
        Some(RemoteLocation {
            protocol,
            user,
            host,
            port,
            path: normalize(path),
        })
    }

    /// The connection this location belongs to: `sftp://user@host:22`.
    pub fn root(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host) // IPv6
        } else {
            self.host.clone()
        };
        let user = if self.user.is_empty() {
            String::new()
        } else {
            format!("{}@", self.user)
        };
        format!("{}://{user}{host}:{}", self.protocol.scheme(), self.port)
    }

    /// Back to a path the app can store: `sftp://user@host:22/home/user`
    /// (the root has no trailing `/`, so `Path::parent` lands on it).
    pub fn to_path(&self) -> PathBuf {
        if self.path == "/" {
            PathBuf::from(self.root())
        } else {
            PathBuf::from(format!("{}{}", self.root(), self.path))
        }
    }

    fn with_path(&self, path: String) -> RemoteLocation {
        RemoteLocation {
            path: normalize(&path),
            ..self.clone()
        }
    }

    /// The location of `name` inside this folder.
    pub fn join(&self, name: &str) -> RemoteLocation {
        if self.path == "/" {
            self.with_path(format!("/{name}"))
        } else {
            self.with_path(format!("{}/{name}", self.path))
        }
    }

    /// The containing folder; `None` at the server's root.
    pub fn parent(&self) -> Option<RemoteLocation> {
        if self.path == "/" {
            return None;
        }
        let parent = match self.path.rfind('/') {
            Some(0) | None => "/".to_string(),
            Some(slash) => self.path[..slash].to_string(),
        };
        Some(self.with_path(parent))
    }

    /// The last path segment ("" at the root).
    pub fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or("")
    }
}

fn split_host_port(host_port: &str, default_port: u16) -> Option<(String, u16)> {
    if let Some(rest) = host_port.strip_prefix('[') {
        // [IPv6]:port
        let (host, after) = rest.split_once(']')?;
        let port = match after.strip_prefix(':') {
            Some(port) => port.parse().ok()?,
            None => default_port,
        };
        return Some((host.to_string(), port));
    }
    match host_port.rsplit_once(':') {
        Some((host, port)) => Some((host.to_string(), port.parse().ok()?)),
        None => Some((host_port.to_string(), default_port)),
    }
}

/// `"//a/./b/../c/"` → `"/a/c"`.
fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    format!("/{}", parts.join("/"))
}

/// Whether `path` is on a server rather than this computer.
pub fn is_remote(path: &Path) -> bool {
    RemoteLocation::parse(path).is_some()
}

/// The folder containing `path` (local or remote); `None` at a root.
pub fn parent(path: &Path) -> Option<PathBuf> {
    match RemoteLocation::parse(path) {
        Some(location) => location.parent().map(|parent| parent.to_path()),
        None => path.parent().map(Path::to_path_buf),
    }
}

/// The folders from the root down to `path`, each with its label, for the
/// path bar: `[("/", /), ("home", /home)]`, or for a server
/// `[("user@host", sftp://user@host:22), ("home", …)]`.
pub fn breadcrumbs(path: &Path) -> Vec<(String, PathBuf)> {
    if let Some(location) = RemoteLocation::parse(path) {
        let mut crumbs = Vec::new();
        let mut current = Some(location);
        while let Some(location) = current {
            let label = if location.path == "/" {
                let user = if location.user.is_empty() {
                    String::new()
                } else {
                    format!("{}@", location.user)
                };
                format!("{user}{}", location.host)
            } else {
                location.name().to_string()
            };
            crumbs.push((label, location.to_path()));
            current = location.parent();
        }
        crumbs.reverse();
        return crumbs;
    }
    let mut crumbs: Vec<(String, PathBuf)> = path
        .ancestors()
        .map(|folder| {
            let label = match folder.file_name() {
                Some(name) => name.to_string_lossy().into_owned(),
                None => folder.display().to_string(), // the root, "/"
            };
            (label, folder.to_path_buf())
        })
        .collect();
    crumbs.reverse();
    crumbs
}

/// How to reach a server, as typed in the "Connect to server" dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectParams {
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    /// Empty: the local user name for SFTP, anonymous for FTP.
    pub user: String,
    /// For SFTP, tried after the SSH agent and key files.
    pub password: Option<String>,
    /// SFTP: add an unknown server's key to `~/.ssh/known_hosts` (the user
    /// confirmed its fingerprint).
    pub trust_new_host_key: bool,
}

impl ConnectParams {
    /// The root path of the connection these parameters open
    /// (`sftp://user@host:22`).
    pub fn location_root(&self) -> PathBuf {
        PathBuf::from(self.location("/".to_string()).root())
    }

    fn effective_user(&self) -> String {
        if !self.user.is_empty() {
            return self.user.clone();
        }
        match self.protocol {
            Protocol::Sftp => std::env::var("USER").unwrap_or_else(|_| "root".to_string()),
            Protocol::Ftp | Protocol::Ftps => String::new(),
        }
    }

    fn location(&self, path: String) -> RemoteLocation {
        RemoteLocation {
            protocol: self.protocol,
            user: self.effective_user(),
            host: self.host.trim().to_string(),
            port: self.port,
            path: normalize(&path),
        }
    }
}

/// Why a connection failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectError {
    /// SFTP: the server isn't in `~/.ssh/known_hosts` yet. Show the
    /// fingerprint and connect again with `trust_new_host_key` if the user
    /// trusts it.
    UnknownHostKey {
        fingerprint: String,
    },
    /// SFTP: the server's key differs from the one in `~/.ssh/known_hosts`
    /// (a reinstalled server, or someone in between). Never connected.
    HostKeyChanged {
        fingerprint: String,
    },
    /// The server rejected the user name, keys and password.
    AuthenticationFailed,
    Other(String),
}

impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectError::UnknownHostKey { fingerprint } => {
                write!(f, "unknown server key {fingerprint}")
            }
            ConnectError::HostKeyChanged { fingerprint } => write!(
                f,
                "the server's key changed ({fingerprint}); it doesn't match ~/.ssh/known_hosts"
            ),
            ConnectError::AuthenticationFailed => write!(f, "login failed"),
            ConnectError::Other(message) => f.write_str(message),
        }
    }
}

/// One open connection. SFTP handles requests concurrently; FTP has a single
/// control channel, so its requests take turns.
enum Session {
    Sftp(sftp::SftpConnection),
    Ftp(tokio::sync::Mutex<ftp::FtpConnection>),
}

struct Connection {
    params: ConnectParams,
    /// The folder shown after connecting (the user's home on the server).
    home: PathBuf,
    session: tokio::sync::RwLock<Session>,
}

/// Open connections by root (`sftp://user@host:22`), shared by all panels.
fn registry() -> &'static Mutex<HashMap<String, Arc<Connection>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, Arc<Connection>>>> = OnceLock::new();
    REGISTRY.get_or_init(Default::default)
}

async fn open_session(params: &ConnectParams) -> Result<(Session, String), ConnectError> {
    match params.protocol {
        Protocol::Sftp => {
            let (connection, home) = sftp::SftpConnection::connect(params).await?;
            Ok((Session::Sftp(connection), home))
        }
        Protocol::Ftp | Protocol::Ftps => {
            let (connection, home) = ftp::FtpConnection::connect(params).await?;
            Ok((Session::Ftp(tokio::sync::Mutex::new(connection)), home))
        }
    }
}

/// Connects to a server and returns the path of the folder to show (the
/// user's home on the server). Replaces an older connection to the same
/// place. The password stays in memory only, to reconnect if the server
/// drops an idle connection.
pub async fn connect(params: ConnectParams) -> Result<PathBuf, ConnectError> {
    let (session, home) = open_session(&params).await?;
    let location = params.location(home);
    let connection = Arc::new(Connection {
        params,
        home: location.to_path(),
        session: tokio::sync::RwLock::new(session),
    });
    registry()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .insert(location.root(), connection);
    Ok(location.to_path())
}

/// Closes the connection `path` belongs to (if open).
pub fn disconnect(path: &Path) {
    if let Some(location) = RemoteLocation::parse(path) {
        registry()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .remove(&location.root());
    }
}

/// The home folder of the open connection `path` belongs to.
pub fn home(path: &Path) -> Option<PathBuf> {
    let location = RemoteLocation::parse(path)?;
    registry()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .get(&location.root())
        .map(|connection| connection.home.clone())
}

/// The root paths (`sftp://user@host:22`) of all open connections.
pub fn open_connections() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = registry()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .keys()
        .map(PathBuf::from)
        .collect();
    roots.sort();
    roots
}

/// Checks that the connection `path` belongs to still answers, by listing
/// its home folder; a connection the server dropped is reopened on the way.
pub async fn check(path: &Path) -> Result<(), String> {
    let home = home(path).ok_or_else(|| "not connected".to_string())?;
    tokio::time::timeout(std::time::Duration::from_secs(15), list(&home))
        .await
        .map_err(|_| "the server doesn't answer".to_string())?
        .map(|_| ())
}

/// Whether there's an open connection for `path`'s server.
pub fn is_connected(path: &Path) -> bool {
    RemoteLocation::parse(path).is_some_and(|location| {
        registry()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .contains_key(&location.root())
    })
}

fn connection_for(location: &RemoteLocation) -> Result<Arc<Connection>, String> {
    registry()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .get(&location.root())
        .cloned()
        .ok_or_else(|| {
            format!(
                "not connected to {} (File → Connect to server)",
                location.root()
            )
        })
}

/// Runs `op` on the location's session, reconnecting once if the server
/// dropped the connection (idle FTP and SSH sessions time out).
macro_rules! with_session {
    ($location:expr, |$session:ident| $op:expr) => {{
        let location: &RemoteLocation = $location;
        let connection = connection_for(location)?;
        let first = {
            let $session = connection.session.read().await;
            $op
        };
        match first {
            Err(err) if is_connection_lost(&err) => {
                let (fresh, _) = open_session(&connection.params)
                    .await
                    .map_err(|err| err.to_string())?;
                *connection.session.write().await = fresh;
                let $session = connection.session.read().await;
                $op
            }
            other => other,
        }
    }};
}

/// Errors that mean "the connection is gone" rather than "that file".
fn is_connection_lost(error: &str) -> bool {
    let error = error.to_lowercase();
    [
        "connection reset",
        "broken pipe",
        "not connected",
        "connection closed",
        "channel closed",
        "session closed",
        "disconnected",
        "unexpected eof",
        "timed out",
        "connection aborted",
    ]
    .iter()
    .any(|sign| error.contains(sign))
}

/// What a remote file or folder is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteStat {
    pub kind: EntryKind,
    pub size: u64,
}

/// Lists a remote folder.
pub async fn list(path: &Path) -> Result<Vec<DirEntry>, String> {
    let location = parse(path)?;
    let entries: Vec<(String, EntryKind, u64, Option<SystemTime>)> =
        with_session!(&location, |session| match &*session {
            Session::Sftp(sftp) => sftp.list(&location.path).await,
            Session::Ftp(ftp) => ftp.lock().await.list(&location.path).await,
        })?;
    Ok(entries
        .into_iter()
        .map(|(name, kind, size, modified)| DirEntry {
            path: location.join(&name).to_path(),
            name,
            kind,
            size,
            modified,
        })
        .collect())
}

/// What's at `path`; `None` if nothing is.
pub async fn stat(path: &Path) -> Result<Option<RemoteStat>, String> {
    let location = parse(path)?;
    if location.path == "/" {
        return Ok(Some(RemoteStat {
            kind: EntryKind::Dir,
            size: 0,
        }));
    }
    // Look the name up in its folder: works the same on SFTP and FTP (many
    // FTP servers have no way to ask about a single file).
    let parent = location.parent().map(|parent| parent.to_path());
    let Some(parent) = parent else {
        return Ok(None);
    };
    let name = location.name().to_string();
    Ok(list(&parent).await?.into_iter().find_map(|entry| {
        (entry.name == name).then_some(RemoteStat {
            kind: entry.kind,
            size: entry.size,
        })
    }))
}

pub async fn create_dir(path: &Path) -> Result<(), String> {
    let location = parse(path)?;
    with_session!(&location, |session| match &*session {
        Session::Sftp(sftp) => sftp.create_dir(&location.path).await,
        Session::Ftp(ftp) => ftp.lock().await.create_dir(&location.path).await,
    })
}

/// Renames or moves within one server.
pub async fn rename(from: &Path, to: &Path) -> Result<(), String> {
    let from = parse(from)?;
    let to = parse(to)?;
    if from.root() != to.root() {
        return Err("can't rename across servers".to_string());
    }
    with_session!(&from, |session| match &*session {
        Session::Sftp(sftp) => sftp.rename(&from.path, &to.path).await,
        Session::Ftp(ftp) => ftp.lock().await.rename(&from.path, &to.path).await,
    })
}

/// Deletes a file, or a folder with everything in it. Servers have no
/// trash, so this is always permanent.
pub async fn remove_all(path: &Path) -> Result<(), String> {
    let location = parse(path)?;
    let Some(stat) = stat(path).await? else {
        return Ok(());
    };
    if stat.kind == EntryKind::Dir {
        for entry in list(path).await? {
            Box::pin(remove_all(&entry.path)).await?;
        }
        with_session!(&location, |session| match &*session {
            Session::Sftp(sftp) => sftp.remove_dir(&location.path).await,
            Session::Ftp(ftp) => ftp.lock().await.remove_dir(&location.path).await,
        })
    } else {
        with_session!(&location, |session| match &*session {
            Session::Sftp(sftp) => sftp.remove_file(&location.path).await,
            Session::Ftp(ftp) => ftp.lock().await.remove_file(&location.path).await,
        })
    }
}

/// Downloads a remote file to the local file `local`, calling `progress`
/// with the bytes copied so far. Stops (with an error) when `cancelled`
/// returns true.
pub async fn download(
    remote: &Path,
    local: &Path,
    progress: &mut (dyn FnMut(u64) + Send),
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<(), String> {
    let location = parse(remote)?;
    with_session!(&location, |session| match &*session {
        Session::Sftp(sftp) => {
            sftp.download(&location.path, local, progress, cancelled)
                .await
        }
        Session::Ftp(ftp) => {
            ftp.lock()
                .await
                .download(&location.path, local, progress, cancelled)
                .await
        }
    })
}

/// Uploads the local file `local` to `remote`, replacing it.
pub async fn upload(
    local: &Path,
    remote: &Path,
    progress: &mut (dyn FnMut(u64) + Send),
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<(), String> {
    let location = parse(remote)?;
    with_session!(&location, |session| match &*session {
        Session::Sftp(sftp) =>
            sftp.upload(local, &location.path, progress, cancelled)
                .await,
        Session::Ftp(ftp) => {
            ftp.lock()
                .await
                .upload(local, &location.path, progress, cancelled)
                .await
        }
    })
}

/// A local copy of a remote file, for opening it in other apps (Enter, F3,
/// F4, Open With): downloaded to `~/.cache/2fip/remote/<server>/<path>`.
/// Local paths come back unchanged. Edits to the copy stay local.
pub async fn local_copy(path: &Path) -> Result<PathBuf, String> {
    let Some(location) = RemoteLocation::parse(path) else {
        return Ok(path.to_path_buf());
    };
    let server = format!(
        "{}-{}-{}",
        location.protocol.scheme(),
        location.host,
        location.port
    );
    let local = crate::sandbox::cache_home()
        .join("2fip/remote")
        .join(server)
        .join(location.path.trim_start_matches('/'));
    if let Some(folder) = local.parent() {
        tokio::fs::create_dir_all(folder)
            .await
            .map_err(|err| format!("failed to create {}: {err}", folder.display()))?;
    }
    download(path, &local, &mut |_| {}, &|| false).await?;
    Ok(local)
}

fn parse(path: &Path) -> Result<RemoteLocation, String> {
    RemoteLocation::parse(path).ok_or_else(|| format!("{} is not a remote path", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats_remote_paths() {
        let location =
            RemoteLocation::parse(Path::new("sftp://me@example.com:2222/home/me/")).unwrap();
        assert_eq!(location.protocol, Protocol::Sftp);
        assert_eq!(location.user, "me");
        assert_eq!(location.host, "example.com");
        assert_eq!(location.port, 2222);
        assert_eq!(location.path, "/home/me");
        assert_eq!(
            location.to_path(),
            PathBuf::from("sftp://me@example.com:2222/home/me")
        );

        // Default port, no user, the root.
        let ftp = RemoteLocation::parse(Path::new("ftp://files.example.org")).unwrap();
        assert_eq!(
            (ftp.port, ftp.user.as_str(), ftp.path.as_str()),
            (21, "", "/")
        );
        assert_eq!(ftp.to_path(), PathBuf::from("ftp://files.example.org:21"));

        // IPv6, and "." / ".." cleaned up.
        let v6 = RemoteLocation::parse(Path::new("ftps://u@[::1]:990/a/./b/../c")).unwrap();
        assert_eq!(
            (v6.host.as_str(), v6.port, v6.path.as_str()),
            ("::1", 990, "/a/c")
        );
        assert_eq!(v6.root(), "ftps://u@[::1]:990");

        assert!(RemoteLocation::parse(Path::new("/home/me")).is_none());
        assert!(RemoteLocation::parse(Path::new("http://example.com")).is_none());
    }

    #[test]
    fn parents_and_breadcrumbs_stop_at_the_server_root() {
        let path = Path::new("sftp://me@host:22/home/me");
        assert_eq!(parent(path), Some(PathBuf::from("sftp://me@host:22/home")));
        assert_eq!(
            parent(Path::new("sftp://me@host:22/home")),
            Some(PathBuf::from("sftp://me@host:22"))
        );
        assert_eq!(parent(Path::new("sftp://me@host:22")), None);
        // `Path::parent` agrees, so code using it lands on the same root.
        assert_eq!(
            Path::new("sftp://me@host:22/home").parent(),
            Some(Path::new("sftp://me@host:22"))
        );

        let crumbs: Vec<String> = breadcrumbs(path)
            .into_iter()
            .map(|(label, _)| label)
            .collect();
        assert_eq!(crumbs, ["me@host", "home", "me"]);
        let local: Vec<String> = breadcrumbs(Path::new("/home/me"))
            .into_iter()
            .map(|(label, _)| label)
            .collect();
        assert_eq!(local, ["/", "home", "me"]);
    }

    #[test]
    fn joins_names() {
        let root = RemoteLocation::parse(Path::new("ftp://host")).unwrap();
        assert_eq!(
            root.join("pub").to_path(),
            PathBuf::from("ftp://host:21/pub")
        );
        assert_eq!(
            Path::new("ftp://host:21").join("pub"),
            PathBuf::from("ftp://host:21/pub"),
            "Path::join gives the same result"
        );
    }
}
