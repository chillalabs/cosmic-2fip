//! SFTP over SSH (`russh`, pure Rust). The server's key is checked against
//! `~/.ssh/known_hosts`, like `ssh` does; login tries the SSH agent, then the
//! usual key files in `~/.ssh` (without a passphrase), then the password.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use russh::client::{self, Handle};
use russh::keys::{self, HashAlg, PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::{FileType, OpenFlags};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::{ConnectError, ConnectParams};
use crate::entry::EntryKind;

const CHUNK: usize = 64 * 1024;

/// What the host key check found, for [`ConnectError`].
#[derive(Debug, Clone)]
enum HostKeyIssue {
    Unknown(String),
    Changed(String),
    Other(String),
}

struct Client {
    host: String,
    port: u16,
    trust_new: bool,
    issue: Arc<Mutex<Option<HostKeyIssue>>>,
}

impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        // Host certificates need a CA setup we don't read: their key is
        // checked like a plain one.
        let key = server_key.public_key();
        let key = &key;
        let fingerprint = key.fingerprint(HashAlg::Sha256).to_string();
        let record = |issue| {
            *self
                .issue
                .lock()
                .unwrap_or_else(|poison| poison.into_inner()) = Some(issue);
        };
        match keys::check_known_hosts(&self.host, self.port, key) {
            Ok(true) => Ok(true),
            Ok(false) if self.trust_new => {
                match keys::known_hosts::learn_known_hosts(&self.host, self.port, key) {
                    Ok(()) => Ok(true),
                    Err(err) => {
                        record(HostKeyIssue::Other(format!(
                            "couldn't save the server key: {err}"
                        )));
                        Ok(false)
                    }
                }
            }
            Ok(false) => {
                record(HostKeyIssue::Unknown(fingerprint));
                Ok(false)
            }
            Err(keys::Error::KeyChanged { .. }) => {
                record(HostKeyIssue::Changed(fingerprint));
                Ok(false)
            }
            Err(err) => {
                record(HostKeyIssue::Other(format!(
                    "couldn't check ~/.ssh/known_hosts: {err}"
                )));
                Ok(false)
            }
        }
    }
}

pub(super) struct SftpConnection {
    // Keeps the SSH connection open as long as the SFTP session lives.
    _handle: Handle<Client>,
    sftp: SftpSession,
}

impl SftpConnection {
    /// Connects and logs in; returns the session and the user's home folder.
    pub(super) async fn connect(
        params: &ConnectParams,
    ) -> Result<(SftpConnection, String), ConnectError> {
        let host = params.host.trim().to_string();
        let issue = Arc::new(Mutex::new(None));
        let client = Client {
            host: host.clone(),
            port: params.port,
            trust_new: params.trust_new_host_key,
            issue: issue.clone(),
        };
        let config = Arc::new(client::Config {
            inactivity_timeout: None,
            keepalive_interval: Some(Duration::from_secs(30)),
            ..Default::default()
        });
        let connected = tokio::time::timeout(
            Duration::from_secs(20),
            client::connect(config, (host.as_str(), params.port), client),
        )
        .await
        .map_err(|_| {
            ConnectError::Other(format!("{host}:{}: connection timed out", params.port))
        })?;
        let mut handle = match connected {
            Ok(handle) => handle,
            Err(err) => {
                let issue = issue
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .take();
                return Err(match issue {
                    Some(HostKeyIssue::Unknown(fingerprint)) => {
                        ConnectError::UnknownHostKey { fingerprint }
                    }
                    Some(HostKeyIssue::Changed(fingerprint)) => {
                        ConnectError::HostKeyChanged { fingerprint }
                    }
                    Some(HostKeyIssue::Other(message)) => ConnectError::Other(message),
                    None => ConnectError::Other(format!("{host}:{}: {err}", params.port)),
                });
            }
        };

        let user = params.effective_user();
        if !authenticate(&mut handle, &user, params.password.as_deref()).await {
            return Err(ConnectError::AuthenticationFailed);
        }

        let other = |err: &dyn std::fmt::Display| ConnectError::Other(err.to_string());
        let channel = handle
            .channel_open_session()
            .await
            .map_err(|err| other(&err))?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|err| other(&err))?;
        let sftp = SftpSession::new(channel.into_stream())
            .await
            .map_err(|err| other(&err))?;
        let home = sftp
            .canonicalize(".")
            .await
            .unwrap_or_else(|_| "/".to_string());
        Ok((
            SftpConnection {
                _handle: handle,
                sftp,
            },
            home,
        ))
    }

    pub(super) async fn list(
        &self,
        path: &str,
    ) -> Result<Vec<(String, EntryKind, u64, Option<SystemTime>)>, String> {
        let entries = self
            .sftp
            .read_dir(path)
            .await
            .map_err(|err| format!("{path}: {err}"))?;
        Ok(entries
            .filter(|entry| entry.file_name() != "." && entry.file_name() != "..")
            .map(|entry| {
                let metadata = entry.metadata();
                let kind = match entry.file_type() {
                    FileType::Dir => EntryKind::Dir,
                    FileType::Symlink => EntryKind::Symlink,
                    FileType::File | FileType::Other => EntryKind::File,
                };
                let modified = metadata
                    .mtime
                    .map(|secs| UNIX_EPOCH + Duration::from_secs(u64::from(secs)));
                (
                    entry.file_name(),
                    kind,
                    metadata.size.unwrap_or(0),
                    modified,
                )
            })
            .collect())
    }

    pub(super) async fn create_dir(&self, path: &str) -> Result<(), String> {
        self.sftp
            .create_dir(path)
            .await
            .map_err(|err| format!("{path}: {err}"))
    }

    pub(super) async fn rename(&self, from: &str, to: &str) -> Result<(), String> {
        self.sftp
            .rename(from, to)
            .await
            .map_err(|err| format!("{from} → {to}: {err}"))
    }

    pub(super) async fn remove_file(&self, path: &str) -> Result<(), String> {
        self.sftp
            .remove_file(path)
            .await
            .map_err(|err| format!("{path}: {err}"))
    }

    pub(super) async fn remove_dir(&self, path: &str) -> Result<(), String> {
        self.sftp
            .remove_dir(path)
            .await
            .map_err(|err| format!("{path}: {err}"))
    }

    pub(super) async fn download(
        &self,
        path: &str,
        local: &Path,
        progress: &mut (dyn FnMut(u64) + Send),
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<(), String> {
        let mut remote = self
            .sftp
            .open(path)
            .await
            .map_err(|err| format!("{path}: {err}"))?;
        let mut file = tokio::fs::File::create(local)
            .await
            .map_err(|err| format!("{}: {err}", local.display()))?;
        let mut buffer = vec![0u8; CHUNK];
        let mut done = 0u64;
        loop {
            if cancelled() {
                return Err("cancelled".to_string());
            }
            let read = remote
                .read(&mut buffer)
                .await
                .map_err(|err| format!("{path}: {err}"))?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])
                .await
                .map_err(|err| format!("{}: {err}", local.display()))?;
            done += read as u64;
            progress(done);
        }
        file.flush()
            .await
            .map_err(|err| format!("{}: {err}", local.display()))
    }

    pub(super) async fn upload(
        &self,
        local: &Path,
        path: &str,
        progress: &mut (dyn FnMut(u64) + Send),
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<(), String> {
        let mut file = tokio::fs::File::open(local)
            .await
            .map_err(|err| format!("{}: {err}", local.display()))?;
        let mut remote = self
            .sftp
            .open_with_flags(
                path,
                OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
            )
            .await
            .map_err(|err| format!("{path}: {err}"))?;
        let mut buffer = vec![0u8; CHUNK];
        let mut done = 0u64;
        loop {
            if cancelled() {
                return Err("cancelled".to_string());
            }
            let read = file
                .read(&mut buffer)
                .await
                .map_err(|err| format!("{}: {err}", local.display()))?;
            if read == 0 {
                break;
            }
            remote
                .write_all(&buffer[..read])
                .await
                .map_err(|err| format!("{path}: {err}"))?;
            done += read as u64;
            progress(done);
        }
        remote
            .shutdown()
            .await
            .map_err(|err| format!("{path}: {err}"))
    }
}

/// Tries the SSH agent's keys, then unprotected key files in `~/.ssh`, then
/// the password. Returns whether the server accepted one.
async fn authenticate(handle: &mut Handle<Client>, user: &str, password: Option<&str>) -> bool {
    let rsa_hash = handle
        .best_supported_rsa_hash()
        .await
        .ok()
        .flatten()
        .flatten();

    // The agent is a Unix socket ($SSH_AUTH_SOCK); on Windows (a named pipe)
    // it isn't supported yet.
    #[cfg(unix)]
    if let Ok(mut agent) = keys::agent::client::AgentClient::connect_env().await {
        if let Ok(identities) = agent.request_identities().await {
            for identity in identities {
                let key = identity.public_key().into_owned();
                let accepted = handle
                    .authenticate_publickey_with(user, key, rsa_hash, &mut agent)
                    .await
                    .is_ok_and(|result| result.success());
                if accepted {
                    return true;
                }
            }
        }
    }

    if let Some(home) = std::env::var_os("HOME") {
        let ssh = Path::new(&home).join(".ssh");
        for name in ["id_ed25519", "id_ecdsa", "id_rsa"] {
            // Keys with a passphrase are skipped: the agent covers those.
            let Ok(key) = keys::load_secret_key(ssh.join(name), None) else {
                continue;
            };
            let key = PrivateKeyWithHashAlg::new(Arc::new(key), rsa_hash);
            let accepted = handle
                .authenticate_publickey(user, key)
                .await
                .is_ok_and(|result| result.success());
            if accepted {
                return true;
            }
        }
    }

    if let Some(password) = password.filter(|password| !password.is_empty()) {
        return handle
            .authenticate_password(user, password)
            .await
            .is_ok_and(|result| result.success());
    }
    false
}
