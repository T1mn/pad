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
