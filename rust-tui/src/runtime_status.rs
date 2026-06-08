use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

mod process;

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
        if let Some(existing) = read_status(&path) {
            if existing.pid != std::process::id() && process_alive(existing.pid) {
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
        if let Some(status) = read_status(&self.path) {
            if status.pid == self.pid && status.started_at == self.started_at {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
}

pub fn read_status(path: &Path) -> Option<ProcessStatus> {
    let body = fs::read_to_string(path).ok()?;
    serde_json::from_str(&body).ok()
}

pub use process::process_alive;

pub fn describe_status(path: &Path) -> String {
    match read_status(path) {
        Some(status) if process_alive(status.pid) => format!("running (pid {})", status.pid),
        Some(status) => format!("stopped (stale pid {})", status.pid),
        None => "stopped".to_string(),
    }
}

fn now_ts() -> i64 {
    crate::time::unix_now_ts()
}

fn write_status_body(path: &Path, status: &ProcessStatus) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(status)?;
    fs::write(path, body)
}

#[cfg(test)]
mod tests;
