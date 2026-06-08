use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("lock settings tests");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let home = std::env::temp_dir().join(format!("pad-settings-{name}-{stamp}"));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create temp home");

    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    let result = f();

    if let Some(prev) = prev_home {
        std::env::set_var("HOME", prev);
    } else {
        std::env::remove_var("HOME");
    }
    let _ = std::fs::remove_dir_all(&home);
    result
}
