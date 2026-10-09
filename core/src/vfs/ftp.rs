//! FTP and FTPS (explicit TLS) with `suppaftp`; TLS is rustls with the
//! Mozilla root certificates, so the server's certificate is checked without
//! OpenSSL. One control connection per server: requests take turns.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use suppaftp::list::{File as ListedFile, ListParser};
use suppaftp::tokio::{
    AsyncFtpStream, AsyncRustlsConnector, AsyncRustlsFtpStream, ImplAsyncFtpStream, TokioTlsStream,
};
use suppaftp::types::FileType as TransferType;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::{ConnectError, ConnectParams, Protocol};
use crate::entry::EntryKind;

const CHUNK: usize = 64 * 1024;

/// Turns one line of a directory listing into a file entry.
type LineParser = fn(&str) -> Option<ListedFile>;
const TIMEOUT: Duration = Duration::from_secs(20);

enum Stream {
    Plain(AsyncFtpStream),
    Tls(AsyncRustlsFtpStream),
}

pub(super) struct FtpConnection {
    stream: Stream,
    /// The server lists folders with MLSD (exact, machine-readable); else
    /// LIST, whose Unix/DOS output is parsed. Asked once at login (FEAT):
    /// trying MLSD on a server without it can leave the connection out of
    /// step.
    mlsd: bool,
}

/// Runs the same code on a plain or a TLS stream.
macro_rules! on_stream {
    ($connection:expr, |$stream:ident| $body:expr) => {
        match $connection {
            Stream::Plain($stream) => $body,
            Stream::Tls($stream) => $body,
        }
    };
}

impl FtpConnection {
    /// Connects and logs in (anonymously without a user name); returns the
    /// connection and the starting folder.
    pub(super) async fn connect(
        params: &ConnectParams,
    ) -> Result<(FtpConnection, String), ConnectError> {
        let host = params.host.trim().to_string();
        let address = format!("{}:{}", bracket_ipv6(&host), params.port);
        let other = |err: &dyn std::fmt::Display| ConnectError::Other(format!("{address}: {err}"));
        let timed_out = |_| other(&"connection timed out");
        let mut stream = match params.protocol {
            Protocol::Ftps => {
                // Explicit TLS: connect in the clear, then `AUTH TLS`.
                let stream =
                    tokio::time::timeout(TIMEOUT, AsyncRustlsFtpStream::connect(address.as_str()))
                        .await
                        .map_err(timed_out)?
                        .map_err(|err| other(&err))?;
                let secure = stream
                    .into_secure(tls_connector(), &host)
                    .await
                    .map_err(|err| other(&err))?;
                Stream::Tls(secure)
            }
            _ => {
                let stream =
                    tokio::time::timeout(TIMEOUT, AsyncFtpStream::connect(address.as_str()))
                        .await
                        .map_err(timed_out)?
                        .map_err(|err| other(&err))?;
                Stream::Plain(stream)
            }
        };

        let (user, password) = if params.user.trim().is_empty() {
            ("anonymous".to_string(), "anonymous@".to_string())
        } else {
            (
                params.user.clone(),
                params.password.clone().unwrap_or_default(),
            )
        };
        let (home, mlsd) = on_stream!(&mut stream, |stream| {
            login(stream, &user, &password).await
        })?;
        Ok((FtpConnection { stream, mlsd }, home))
    }

    pub(super) async fn list(
        &mut self,
        path: &str,
    ) -> Result<Vec<(String, EntryKind, u64, Option<SystemTime>)>, String> {
        let mlsd = self.mlsd;
        on_stream!(&mut self.stream, |stream| {
            let (lines, parse): (Vec<String>, LineParser) = if mlsd {
                (
                    stream
                        .mlsd(Some(path))
                        .await
                        .map_err(|err| format!("{path}: {err}"))?,
                    |line| ListParser::parse_mlsd(line).ok(),
                )
            } else {
                (
                    stream
                        .list(Some(path))
                        .await
                        .map_err(|err| format!("{path}: {err}"))?,
                    |line| {
                        ListParser::parse_posix(line)
                            .or_else(|_| ListParser::parse_dos(line))
                            .ok()
                    },
                )
            };
            Ok(lines
                .iter()
                .filter_map(|line| parse(line))
                .filter(|file| file.name() != "." && file.name() != "..")
                .map(|file| {
                    let kind = if file.is_directory() {
                        EntryKind::Dir
                    } else if file.is_symlink() {
                        EntryKind::Symlink
                    } else {
                        EntryKind::File
                    };
                    (
                        file.name().to_string(),
                        kind,
                        file.size() as u64,
                        Some(file.modified()),
                    )
                })
                .collect())
        })
    }

    pub(super) async fn create_dir(&mut self, path: &str) -> Result<(), String> {
        on_stream!(&mut self.stream, |stream| stream
            .mkdir(path)
            .await
            .map_err(|err| format!("{path}: {err}")))
    }

    pub(super) async fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        on_stream!(&mut self.stream, |stream| stream
            .rename(from, to)
            .await
            .map_err(|err| format!("{from} → {to}: {err}")))
    }

    pub(super) async fn remove_file(&mut self, path: &str) -> Result<(), String> {
        on_stream!(&mut self.stream, |stream| stream
            .rm(path)
            .await
            .map_err(|err| format!("{path}: {err}")))
    }

    pub(super) async fn remove_dir(&mut self, path: &str) -> Result<(), String> {
        on_stream!(&mut self.stream, |stream| stream
            .rmdir(path)
            .await
            .map_err(|err| format!("{path}: {err}")))
    }

    pub(super) async fn download(
        &mut self,
        path: &str,
        local: &Path,
        progress: &mut (dyn FnMut(u64) + Send),
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<(), String> {
        on_stream!(&mut self.stream, |stream| {
            let mut file = tokio::fs::File::create(local)
                .await
                .map_err(|err| format!("{}: {err}", local.display()))?;
            let mut remote = stream
                .retr_as_stream(path)
                .await
                .map_err(|err| format!("{path}: {err}"))?;
            let mut buffer = vec![0u8; CHUNK];
            let mut done = 0u64;
            loop {
                if cancelled() {
                    let _ = stream.abort(remote).await;
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
            remote
                .finish()
                .await
                .map_err(|err| format!("{path}: {err}"))?;
            file.flush()
                .await
                .map_err(|err| format!("{}: {err}", local.display()))
        })
    }

    pub(super) async fn upload(
        &mut self,
        local: &Path,
        path: &str,
        progress: &mut (dyn FnMut(u64) + Send),
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<(), String> {
        on_stream!(&mut self.stream, |stream| {
            let mut file = tokio::fs::File::open(local)
                .await
                .map_err(|err| format!("{}: {err}", local.display()))?;
            let mut remote = stream
                .put_with_stream(path)
                .await
                .map_err(|err| format!("{path}: {err}"))?;
            let mut buffer = vec![0u8; CHUNK];
            let mut done = 0u64;
            loop {
                if cancelled() {
                    let _ = stream.abort(remote).await;
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
                .finish()
                .await
                .map_err(|err| format!("{path}: {err}"))
        })
    }
}

async fn login<T: TokioTlsStream + Send>(
    stream: &mut ImplAsyncFtpStream<T>,
    user: &str,
    password: &str,
) -> Result<(String, bool), ConnectError> {
    stream.login(user, password).await.map_err(|err| {
        let message = err.to_string();
        if message.contains("530") {
            ConnectError::AuthenticationFailed
        } else {
            ConnectError::Other(message)
        }
    })?;
    stream
        .transfer_type(TransferType::Binary)
        .await
        .map_err(|err| ConnectError::Other(err.to_string()))?;
    // MLST in FEAT means MLSD works too (RFC 3659).
    let mlsd = stream.feat().await.is_ok_and(|features| {
        features.keys().any(|feature| {
            feature.eq_ignore_ascii_case("MLST") || feature.eq_ignore_ascii_case("MLSD")
        })
    });
    let home = stream.pwd().await.unwrap_or_else(|_| "/".to_string());
    Ok((home, mlsd))
}

/// rustls with Mozilla's root certificates and the `ring` provider.
fn tls_connector() -> AsyncRustlsConnector {
    use tokio_rustls::rustls;
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("ring supports the default TLS versions")
    .with_root_certificates(roots)
    .with_no_client_auth();
    AsyncRustlsConnector::from(tokio_rustls::TlsConnector::from(Arc::new(config)))
}

fn bracket_ipv6(host: &str) -> String {
    if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}
