use super::super::socket_file::bind_private_listener;
use super::start_api_listener;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener as StdUnixListener;
use std::path::{Path, PathBuf};

fn mode_of(path: &Path) -> u32 {
    std::fs::metadata(path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777
}

/// 故意建成 0755，验证 bind 会把 socket 所在目录收回 0700。
fn scratch_dir(name: &str) -> PathBuf {
    let dir = crate::test_support::temp_path("pad", name);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("widen dir");
    dir
}

#[test]
fn bind_private_listener_creates_owner_only_socket() {
    let dir = scratch_dir("b1");
    let socket = dir.join("s.sock");

    let listener = bind_private_listener(&socket).expect("bind socket");

    assert_eq!(mode_of(&socket), 0o600);
    assert_eq!(mode_of(&dir) & 0o077, 0);
    drop(listener);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bind_private_listener_reclaims_dead_socket() {
    let dir = scratch_dir("b2");
    let socket = dir.join("s.sock");
    drop(StdUnixListener::bind(&socket).expect("seed stale socket"));

    let listener = bind_private_listener(&socket).expect("rebind over stale socket");

    assert_eq!(mode_of(&socket), 0o600);
    drop(listener);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bind_private_listener_refuses_live_socket() {
    let dir = scratch_dir("b3");
    let socket = dir.join("s.sock");
    let live = bind_private_listener(&socket).expect("first bind");

    let err = bind_private_listener(&socket).expect_err("second bind must fail");

    assert_eq!(err.kind(), ErrorKind::AlreadyExists);
    drop(live);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn start_api_listener_binds_owner_only_socket() {
    crate::test_support::with_temp_home("pad", "api", |_home| {
        start_api_listener().expect("start api listener");

        let socket = crate::paths::api_socket_path();
        assert_eq!(mode_of(&socket), 0o600);
        assert_eq!(mode_of(socket.parent().expect("pad home")) & 0o077, 0);
    });
}

#[tokio::test]
async fn start_api_listener_reports_bind_failure() {
    crate::test_support::with_temp_home("pad", "busy", |_home| {
        let socket = crate::paths::api_socket_path();
        std::fs::create_dir_all(socket.parent().expect("pad home")).expect("create pad home");
        let _live = StdUnixListener::bind(&socket).expect("seed live socket");

        let err = start_api_listener().expect_err("bind failure must reach the caller");

        assert_eq!(err.kind(), ErrorKind::AlreadyExists);
    });
}
