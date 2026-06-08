use std::sync::{Mutex, OnceLock};

pub fn home_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub fn temp_path(prefix: &str, name: &str) -> std::path::PathBuf {
    let unique = crate::time::unix_now_nanos();
    std::env::temp_dir().join(format!("{prefix}-{name}-{unique}"))
}
