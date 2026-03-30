use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[allow(dead_code)]
pub fn write_status(path: &Path, mode: &str) -> io::Result<()> {
    let status = ProcessStatus {
        pid: std::process::id(),
        started_at: now_ts(),
        mode: mode.to_string(),
    };
    write_status_body(path, &status)
}

pub fn read_status(path: &Path) -> Option<ProcessStatus> {
    let body = fs::read_to_string(path).ok()?;
    serde_json::from_str(&body).ok()
}

pub fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let alive = unsafe {
            let rc = libc::kill(pid as i32, 0);
            if rc == 0 {
                true
            } else {
                let err = *libc::__error();
                err == libc::EPERM
            }
        };
        alive && !process_is_zombie(pid)
    }

    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

#[cfg(unix)]
fn process_is_zombie(pid: u32) -> bool {
    let output = Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output();
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let stat = String::from_utf8_lossy(&output.stdout);
    stat_indicates_zombie(&stat)
}

fn stat_indicates_zombie(stat: &str) -> bool {
    stat.trim().chars().any(|ch| ch == 'Z')
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

fn write_status_body(path: &Path, status: &ProcessStatus) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(status)?;
    fs::write(path, body)
}

#[cfg(test)]
mod tests {
    use super::{
        read_status, stat_indicates_zombie, write_status_body, ProcessStatus, StatusGuard,
    };
    use std::fs;

    #[test]
    fn status_guard_drop_preserves_newer_status_file() {
        let path = std::env::temp_dir().join(format!(
            "pad-status-guard-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let guard = StatusGuard::new(path.clone(), "telegram-bot").unwrap();
        write_status_body(
            &path,
            &ProcessStatus {
                pid: guard.pid.saturating_add(1),
                started_at: guard.started_at.saturating_add(1),
                mode: "telegram-bot".to_string(),
            },
        )
        .unwrap();
        drop(guard);

        let status = read_status(&path).unwrap();
        assert_eq!(status.pid, std::process::id().saturating_add(1));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn stat_parser_treats_zombies_as_not_alive() {
        assert!(stat_indicates_zombie("Z+"));
        assert!(stat_indicates_zombie("SZ"));
        assert!(!stat_indicates_zombie("S+"));
        assert!(!stat_indicates_zombie("R"));
    }
}
