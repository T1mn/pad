use super::{daemon_socket_is_active, start_direct_hook_listener};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn mode_of(path: &std::path::Path) -> u32 {
    std::fs::metadata(path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777
}

fn with_short_home<T>(f: impl FnOnce() -> T) -> T {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("lock HOME env for test");
    let home = Path::new("/tmp/pad-tg-hook-test");
    let _ = std::fs::remove_dir_all(home);
    std::fs::create_dir_all(home).expect("create test home");
    let previous = std::env::var_os("HOME");
    std::env::set_var("HOME", home);

    let result = f();

    if let Some(previous) = previous {
        std::env::set_var("HOME", previous);
    } else {
        std::env::remove_var("HOME");
    }
    let _ = std::fs::remove_dir_all(home);
    result
}

#[tokio::test]
async fn direct_hook_listener_binds_private_socket_and_reports_conflicts() {
    with_short_home(|| {
        start_direct_hook_listener().expect("start direct hook listener");
        let socket = crate::paths::telegram_hook_socket_path();

        assert_eq!(mode_of(&socket), 0o600);
        assert_eq!(mode_of(socket.parent().expect("pad home")) & 0o077, 0);
        assert!(daemon_socket_is_active());

        let err = start_direct_hook_listener().expect_err("duplicate hook listener must fail");
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    });
}
