use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

fn log_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
    });
    path.push("pad");
    path.push("pad.log");
    path
}

pub fn init() -> Result<(), Box<dyn Error>> {
    let path = log_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Truncate log file on start
    std::fs::write(&path, "")?;
    LOG_PATH.set(path).ok();
    Ok(())
}

/// Check if logger has been initialized (debug mode is on)
pub fn is_enabled() -> bool {
    LOG_PATH.get().is_some()
}

pub fn log(msg: &str) {
    if let Some(path) = LOG_PATH.get() {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let now = chrono_lite();
            let _ = writeln!(file, "[{}] {}", now, msg);
        }
    }
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
            format!("{:02}:{:02}:{:02}", hours, mins, s)
        }
        Err(_) => "??:??:??".to_string(),
    }
}
