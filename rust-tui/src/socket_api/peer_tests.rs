use super::{authorize_peer, current_uid, peer_uid_is_allowed};
use std::os::unix::net::{UnixListener, UnixStream};

#[test]
fn same_uid_peer_is_allowed() {
    assert!(peer_uid_is_allowed(501, 501));
    assert!(peer_uid_is_allowed(0, 0));
}

#[test]
fn foreign_uid_peer_is_rejected() {
    assert!(!peer_uid_is_allowed(1000, 501));
    // root 也不特殊放行。
    assert!(!peer_uid_is_allowed(0, 501));
    assert!(!peer_uid_is_allowed(501, 0));
}

#[test]
fn authorize_peer_accepts_same_uid_connection() {
    let path = crate::test_support::temp_path("pad", "peer");
    let listener = UnixListener::bind(&path).expect("bind peer socket");
    let _client = UnixStream::connect(&path).expect("connect peer socket");
    let (server, _) = listener.accept().expect("accept peer socket");

    assert_eq!(
        authorize_peer(&server).expect("authorize peer"),
        current_uid()
    );

    let _ = std::fs::remove_file(&path);
}
