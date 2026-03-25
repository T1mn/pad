use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessStatus {
    pub pid: u32,
    pub started_at: i64,
    pub mode: String,
}

pub struct StatusGuard {
    path: PathBuf,
}

impl StatusGuard {
    pub fn new(path: PathBuf, mode: &str) -> io::Result<Self> {
        write_status(&path, mode)?;
        Ok(Self { path })
    }
}

impl Drop for StatusGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn write_status(path: &Path, mode: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let status = ProcessStatus {
        pid: std::process::id(),
        started_at: now_ts(),
        mode: mode.to_string(),
    };
    let body = serde_json::to_string_pretty(&status)?;
    fs::write(path, body)
}

pub fn read_status(path: &Path) -> Option<ProcessStatus> {
    let body = fs::read_to_string(path).ok()?;
    serde_json::from_str(&body).ok()
}

pub fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    unsafe {
        let rc = libc::kill(pid as i32, 0);
        if rc == 0 {
            true
        } else {
            let err = *libc::__error();
            err == libc::EPERM
        }
    }

    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

pub fn describe_status(path: &Path) -> String {
    match read_status(path) {
        Some(status) if process_alive(status.pid) => format!("running (pid {})", status.pid),
        Some(status) => format!("stopped (stale pid {})", status.pid),
        None => "stopped".to_string(),
    }
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
