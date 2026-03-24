use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
const MAX_LOG_SIZE_BYTES: u64 = 5 * 1024 * 1024;

fn log_path() -> PathBuf {
    crate::paths::log_path()
}

pub fn init() -> Result<(), Box<dyn Error>> {
    let path = log_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        std::fs::write(&path, "")?;
    }
    LOG_PATH.set(path).ok();
    Ok(())
}

/// Check if logger has been initialized (debug mode is on)
pub fn is_enabled() -> bool {
    LOG_PATH.get().is_some()
}

pub fn log(msg: &str) {
    if let Some(path) = LOG_PATH.get() {
        rotate_log_if_needed(path);
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let now = chrono_lite();
            let _ = writeln!(file, "[{}] {}", now, msg);
        }
    }
}

fn rotate_log_if_needed(path: &PathBuf) {
    let should_rotate = fs::metadata(path)
        .map(|meta| meta.len() >= MAX_LOG_SIZE_BYTES)
        .unwrap_or(false);

    if !should_rotate {
        return;
    }

    let rotated_path = path.with_extension("log.1");
    let _ = fs::remove_file(&rotated_path);
    let _ = fs::rename(path, &rotated_path);
}

/// Convenience macro for debug logging — only writes when logger is initialized
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        if $crate::logger::is_enabled() {
            $crate::logger::log(&format!($($arg)*));
        }
    };
}

/// Simple timestamp without chrono dependency
fn chrono_lite() -> String {
    use std::time::SystemTime;
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => {
            let secs = d.as_secs();
            let hours = (secs / 3600) % 24;
            let mins = (secs / 60) % 60;
            let s = secs % 60;
            let millis = d.subsec_millis();
            format!("{:02}:{:02}:{:02}.{:03}", hours, mins, s, millis)
        }
        Err(_) => "??:??:??.???".to_string(),
    }
}
