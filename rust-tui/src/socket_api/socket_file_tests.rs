use super::bind_private_listener;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener as StdUnixListener;
use std::path::{Path, PathBuf};

fn scratch_dir(name: &str) -> PathBuf {
    let dir = crate::test_support::temp_path("pad", name);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn mode_of(path: &Path) -> u32 {
    std::fs::metadata(path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777
}

#[test]
fn bind_private_listener_sets_private_socket_and_directory_modes() {
    let dir = scratch_dir("b4");
    let socket = dir.join("pad.sock");

    let listener = bind_private_listener(&socket).expect("bind socket");

    assert_eq!(mode_of(&dir), 0o700);
    assert_eq!(mode_of(&socket), 0o600);
    drop(listener);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bind_private_listener_reclaims_stale_socket() {
    let dir = scratch_dir("b5");
    let socket = dir.join("pad.sock");
    drop(StdUnixListener::bind(&socket).expect("seed stale socket"));

    let listener = bind_private_listener(&socket).expect("rebind over stale socket");

    assert_eq!(mode_of(&socket), 0o600);
    drop(listener);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bind_private_listener_rejects_active_socket() {
    let dir = scratch_dir("b6");
    let socket = dir.join("pad.sock");
    let listener = bind_private_listener(&socket).expect("first bind");

    let err = bind_private_listener(&socket).expect_err("second bind must fail");

    assert_eq!(err.kind(), ErrorKind::AlreadyExists);
    drop(listener);
    let _ = std::fs::remove_dir_all(&dir);
}
