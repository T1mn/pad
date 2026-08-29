//! Process-wide ownership lock for one PAD Desktop data root.
//!
//! SQLite can serialize individual transactions, but the Desktop control
//! plane also owns Pi/auth/terminal processes and in-memory task state. One
//! server must therefore own the complete root for its whole lifetime.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

#[derive(Debug)]
pub(crate) struct DesktopDataRootLock {
    file: File,
}

impl DesktopDataRootLock {
    pub(crate) fn acquire(root: &Path) -> io::Result<Self> {
        let path = lock_path(root);
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Desktop lock path has no parent",
            )
        })?;
        crate::paths::base::ensure_private_dir(parent)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        let mut file = options.open(&path)?;
        lock_file(&file)?;
        #[cfg(unix)]
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        writeln!(file, "{}", std::process::id())?;
        file.flush()?;
        Ok(Self { file })
    }
}

impl Drop for DesktopDataRootLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        unsafe {
            let _ = libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

pub(crate) fn lock_path(root: &Path) -> PathBuf {
    root.join("v1").join("store").join("desktop-server.lock")
}

#[cfg(unix)]
fn lock_file(file: &File) -> io::Result<()> {
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
            "another PAD desktop-server already owns this data root",
        ))
    } else {
        Err(error)
    }
}

#[cfg(not(unix))]
fn lock_file(_file: &File) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
pub(crate) fn same_root_has_exactly_one_owner() {
    let root = std::env::temp_dir().join(format!(
        "pad-desktop-root-lock-{}-{}",
        std::process::id(),
        crate::time::unix_now_nanos()
    ));
    let first = DesktopDataRootLock::acquire(&root).unwrap();
    let second = DesktopDataRootLock::acquire(&root).unwrap_err();
    assert_eq!(second.kind(), io::ErrorKind::AlreadyExists);
    drop(first);
    let reacquired = DesktopDataRootLock::acquire(&root).unwrap();
    assert!(lock_path(&root).is_file());
    drop(reacquired);
}
