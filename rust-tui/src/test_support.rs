use std::sync::{Mutex, OnceLock};

pub fn home_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub fn temp_path(prefix: &str, name: &str) -> std::path::PathBuf {
    let unique = crate::time::unix_now_nanos();
    std::env::temp_dir().join(format!("{prefix}-{name}-{unique}"))
}

pub fn with_temp_home<T>(prefix: &str, name: &str, f: impl FnOnce(&std::path::Path) -> T) -> T {
    let _guard = home_env_lock().lock().expect("lock HOME env for test");
    let home = temp_path(prefix, name);
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create temp home");

    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    let result = f(&home);

    if let Some(prev) = prev_home {
        std::env::set_var("HOME", prev);
    } else {
        std::env::remove_var("HOME");
    }
    let _ = std::fs::remove_dir_all(&home);

    result
}
