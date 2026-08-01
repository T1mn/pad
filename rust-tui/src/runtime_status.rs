use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

mod identity;
mod lock;
mod process;

pub(crate) use lock::StatusLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessStatus {
    pub pid: u32,
    pub started_at: i64,
    pub mode: String,
}

pub struct StatusGuard {
    path: PathBuf,
    pid: u32,
    started_at: i64,
}

impl StatusGuard {
    pub fn new(path: PathBuf, mode: &str) -> io::Result<Self> {
        let _lock = StatusLock::acquire(&path)?;
        if let Some(existing) = read_status(&path) {
            if status_process_alive(&existing) {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("{} already running with pid {}", mode, existing.pid),
                ));
            }
        }

        let started_at = now_ts();
        let status = ProcessStatus {
            pid: std::process::id(),
            started_at,
            mode: mode.to_string(),
        };
        write_status_body(&path, &status)?;
        Ok(Self {
            path,
            pid: status.pid,
            started_at,
        })
    }
}

impl Drop for StatusGuard {
    fn drop(&mut self) {
        // A stop operation may already hold the lock while terminating us. In
        // that case it owns the cleanup; leaving the file behind is safer than
        // racing a new daemon's status write.
        let Ok(_lock) = StatusLock::acquire(&self.path) else {
            return;
        };
        if status_matches(&self.path, self.pid, self.started_at) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn status_matches(path: &Path, pid: u32, started_at: i64) -> bool {
    read_status(path).is_some_and(|status| status.pid == pid && status.started_at == started_at)
}

pub(crate) fn status_lock_path(status_path: &Path) -> PathBuf {
    let mut lock_path = status_path.to_path_buf();
    lock_path.set_extension("lock");
    lock_path
}

pub fn read_status(path: &Path) -> Option<ProcessStatus> {
    let body = fs::read_to_string(path).ok()?;
    serde_json::from_str(&body).ok()
}

pub use identity::status_process_alive;
pub use process::process_alive;

pub fn describe_status(path: &Path) -> String {
    match read_status(path) {
        Some(status) if status_process_alive(&status) => format!("running (pid {})", status.pid),
        Some(status) => format!("stopped (stale pid {})", status.pid),
        None => "stopped".to_string(),
    }
}

fn now_ts() -> i64 {
    crate::time::unix_now_ts()
}

fn write_status_body(path: &Path, status: &ProcessStatus) -> io::Result<()> {
    let body = serde_json::to_string_pretty(status)?;
    crate::atomic_file::write_private(path, body)
}

#[cfg(test)]
mod tests;
