use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) mod identity;
mod lock {
    use std::fs::{self, File, OpenOptions};
    use std::io;
    use std::path::Path;

    #[cfg(unix)]
    use std::os::fd::AsRawFd;
    #[cfg(windows)]
    use std::os::windows::fs::OpenOptionsExt;

    /// Serializes status-file ownership changes across processes.
    ///
    /// The lock file is intentionally kept after release. Removing an open lock
    /// file would let another process create a new inode and bypass the lock.
    pub(crate) struct StatusLock {
        _file: File,
    }

    impl Drop for StatusLock {
        fn drop(&mut self) {
            #[cfg(unix)]
            unsafe {
                let _ = libc::flock(self._file.as_raw_fd(), libc::LOCK_UN);
            }
        }
    }

    impl StatusLock {
        pub(crate) fn acquire(status_path: &Path) -> io::Result<Self> {
            if let Some(parent) = status_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let file = open_status_lock(&super::status_lock_path(status_path))?;
            lock_status_file(&file)?;
            Ok(Self { _file: file })
        }
    }

    fn open_status_lock(path: &Path) -> io::Result<File> {
        #[cfg(windows)]
        {
            return OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .share_mode(0)
                .open(path)
                .map_err(normalize_lock_error);
        }

        #[cfg(not(windows))]
        {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(path)
        }
    }

    #[cfg(unix)]
    fn lock_status_file(file: &File) -> io::Result<()> {
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result == 0 {
            return Ok(());
        }

        let error = io::Error::last_os_error();
        if error
            .raw_os_error()
            .is_some_and(|code| code == libc::EAGAIN || code == libc::EWOULDBLOCK)
        {
            Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "runtime status lock is already held",
            ))
        } else {
            Err(error)
        }
    }

    #[cfg(windows)]
    fn lock_status_file(_file: &File) -> io::Result<()> {
        Ok(())
    }

    #[cfg(not(any(unix, windows)))]
    fn lock_status_file(_file: &File) -> io::Result<()> {
        Ok(())
    }

    #[cfg(windows)]
    fn normalize_lock_error(error: io::Error) -> io::Error {
        match error.raw_os_error() {
            Some(32 | 33) => io::Error::new(
                io::ErrorKind::AlreadyExists,
                "runtime status lock is already held",
            ),
            _ => error,
        }
    }
}
mod process {
    use std::io;
    #[cfg(unix)]
    use std::process::Command;

    /// 通用存活探测:EPERM 也算活着,因为 pid 确实被某个进程占着。
    /// 想知道"那个进程是不是我们自己写下的 daemon",用 `status_process_alive`。
    pub fn process_alive(pid: u32) -> bool {
        #[cfg(unix)]
        {
            let alive = unsafe {
                let rc = libc::kill(pid as i32, 0);
                if rc == 0 {
                    true
                } else {
                    io::Error::last_os_error()
                        .raw_os_error()
                        .is_some_and(|err| err == libc::EPERM)
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

    /// 严格存活探测:只有我们能给它发信号才算活着。
    /// EPERM 说明 pid 已经属于别的用户的进程,对"daemon 是否在跑"这个语义就是死的。
    pub(in crate::runtime_status) fn process_signalable(pid: u32) -> bool {
        #[cfg(unix)]
        {
            let signalable = unsafe { libc::kill(pid as i32, 0) == 0 };
            signalable && !process_is_zombie(pid)
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

    pub(in crate::runtime_status) fn stat_indicates_zombie(stat: &str) -> bool {
        stat.trim().chars().any(|ch| ch == 'Z')
    }
}

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
pub(crate) mod tests;
