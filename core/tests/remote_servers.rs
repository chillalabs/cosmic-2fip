//! End-to-end tests against real servers (ignored by default). Start test
//! servers, e.g. with Podman:
//!
//! ```sh
//! podman run -d --name 2fip-test-sftp -p 127.0.0.1:2222:22 docker.io/atmoz/sftp tester:testpass:::upload
//! podman run -d --name 2fip-test-ftp -p 127.0.0.1:2121:21 -p 127.0.0.1:21000-21010:21000-21010 \
//!     -e USERS="tester|testpass" -e ADDRESS=127.0.0.1 docker.io/delfer/alpine-ftp-server
//! cargo test -p fs-ops --test remote_servers -- --ignored --test-threads=1
//! ```

use std::path::{Path, PathBuf};

use fs_ops::ops::{CancelHandle, ConflictHandle, OpEvent};
use fs_ops::vfs::{self, ConnectError, ConnectParams, Protocol};
use tokio_stream::StreamExt;

async fn finish(stream: impl tokio_stream::Stream<Item = OpEvent>) -> Vec<OpEvent> {
    let events: Vec<OpEvent> = stream.collect().await;
    assert!(
        matches!(events.last(), Some(OpEvent::Done)),
        "operation didn't finish: {:?}",
        events.last()
    );
    events
}

async fn names(path: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs_ops::list_dir(path)
        .await
        .unwrap()
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    names.sort();
    names
}

/// The same round trip for every protocol, in a folder the test user may
/// write to: `writable` inside the home folder, or the home folder itself.
async fn round_trip(params: ConnectParams, writable: Option<&str>) {
    let home = vfs::connect(params).await.expect("connect");
    let base: PathBuf = match writable {
        Some(name) => vfs::RemoteLocation::parse(&home)
            .unwrap()
            .join(name)
            .to_path(),
        None => home.clone(),
    };
    let work = fs_ops::ops::create_dir(base.clone(), format!("2fip-test-{}", std::process::id()))
        .await
        .expect("mkdir");

    // Upload a local folder with a file and a subfolder.
    let local = tempfile::tempdir().unwrap();
    let src = local.path().join("bundle");
    std::fs::create_dir_all(src.join("inner")).unwrap();
    std::fs::write(src.join("a.txt"), b"hello server").unwrap();
    std::fs::write(src.join("inner/b.bin"), vec![7u8; 300_000]).unwrap();
    let events = finish(fs_ops::ops::copy(
        vec![src.clone()],
        work.clone(),
        CancelHandle::new(),
        ConflictHandle::new(),
    ))
    .await;
    let bytes = events
        .iter()
        .filter_map(|event| match event {
            OpEvent::Progress(progress) => Some(progress.bytes_total),
            _ => None,
        })
        .max();
    assert_eq!(bytes, Some(300_012), "byte totals reported");
    assert_eq!(names(&work).await, ["bundle"]);
    let bundle = vfs::RemoteLocation::parse(&work)
        .unwrap()
        .join("bundle")
        .to_path();
    assert_eq!(names(&bundle).await, ["a.txt", "inner"]);

    // Download it back and compare.
    let back = tempfile::tempdir().unwrap();
    finish(fs_ops::ops::copy(
        vec![bundle.clone()],
        back.path().to_path_buf(),
        CancelHandle::new(),
        ConflictHandle::new(),
    ))
    .await;
    assert_eq!(
        std::fs::read(back.path().join("bundle/a.txt")).unwrap(),
        b"hello server"
    );
    assert_eq!(
        std::fs::read(back.path().join("bundle/inner/b.bin"))
            .unwrap()
            .len(),
        300_000
    );

    // Rename, then move within the server, then a local copy for opening.
    let renamed = fs_ops::ops::rename(bundle.clone(), "renamed".to_string())
        .await
        .expect("rename");
    let sub = fs_ops::ops::create_dir(work.clone(), "sub".to_string())
        .await
        .unwrap();
    finish(fs_ops::ops::move_paths(
        vec![renamed.clone()],
        sub.clone(),
        CancelHandle::new(),
        ConflictHandle::new(),
    ))
    .await;
    assert_eq!(names(&work).await, ["sub"]);
    let moved_file = vfs::RemoteLocation::parse(&sub)
        .unwrap()
        .join("renamed")
        .join("a.txt")
        .to_path();
    let copy = vfs::local_copy(&moved_file).await.expect("local copy");
    assert_eq!(std::fs::read(copy).unwrap(), b"hello server");

    // Delete everything (permanently: servers have no trash).
    finish(fs_ops::ops::delete_to_trash(
        vec![work.clone()],
        CancelHandle::new(),
    ))
    .await;
    let work_name = vfs::RemoteLocation::parse(&work)
        .unwrap()
        .name()
        .to_string();
    assert!(
        !names(&base).await.contains(&work_name),
        "the work folder is gone"
    );
    vfs::disconnect(&home);
}

/// Isolates `~/.ssh/known_hosts` and the SSH agent from the real ones.
fn isolate_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", home.path());
    std::env::remove_var("SSH_AUTH_SOCK");
    home
}

#[tokio::test]
#[ignore = "needs the SFTP test server (see the top of this file)"]
async fn sftp_round_trip() {
    let _home = isolate_home();
    let params = ConnectParams {
        protocol: Protocol::Sftp,
        host: "127.0.0.1".into(),
        port: 2222,
        user: "tester".into(),
        password: Some("testpass".into()),
        trust_new_host_key: false,
    };
    // An unknown server must be confirmed first, with its fingerprint.
    match vfs::connect(params.clone()).await {
        Err(ConnectError::UnknownHostKey { fingerprint }) => {
            assert!(fingerprint.starts_with("SHA256:"))
        }
        other => panic!("expected an unknown host key, got {other:?}"),
    }
    // A wrong password fails cleanly.
    let wrong = ConnectParams {
        password: Some("nope".into()),
        trust_new_host_key: true,
        ..params.clone()
    };
    assert_eq!(
        vfs::connect(wrong).await,
        Err(ConnectError::AuthenticationFailed)
    );
    // Now trusted (saved to the isolated known_hosts by the last attempt).
    round_trip(params, Some("upload")).await;
}

#[tokio::test]
#[ignore = "needs the FTP test server (see the top of this file)"]
async fn ftp_round_trip() {
    let params = ConnectParams {
        protocol: Protocol::Ftp,
        host: "127.0.0.1".into(),
        port: 2121,
        user: "tester".into(),
        password: Some("testpass".into()),
        trust_new_host_key: false,
    };
    let wrong = ConnectParams {
        password: Some("nope".into()),
        ..params.clone()
    };
    assert_eq!(
        vfs::connect(wrong).await,
        Err(ConnectError::AuthenticationFailed)
    );
    round_trip(params, None).await;
}
