//! 原子写文件 helper：写同目录临时文件 -> 设权限 -> fsync -> rename。
//!
//! 直接 `fs::write` 会先截断再写，另一个进程（例如 telegram daemon 与 TUI 同时保存
//! config.toml）在中途读取就会看到半个文件。rename 在同一文件系统内是原子的，
//! 读者要么看到旧内容要么看到新内容。

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// 仅所有者可读写，用于存放 api_key / bot_token 的明文配置。
#[cfg(unix)]
pub const PRIVATE_MODE: u32 = 0o600;

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// 原子写入并把目标文件权限收紧到 0600（unix）。
pub fn write_private(path: &Path, content: impl AsRef<[u8]>) -> io::Result<()> {
    let content = content.as_ref();
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(parent) = parent {
        std::fs::create_dir_all(parent)?;
    }
    let dir = parent.unwrap_or_else(|| Path::new("."));
    let temp = temp_path(path, dir);

    if let Err(err) = write_temp(&temp, content) {
        let _ = std::fs::remove_file(&temp);
        return Err(err);
    }
    if let Err(err) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(err);
    }
    sync_dir(dir);
    Ok(())
}

/// Tighten an existing regular file without following a symlink.
pub fn ensure_private(path: &Path) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "private file path is not a regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(PRIVATE_MODE))?;
    }
    Ok(())
}

fn write_temp(temp: &Path, content: &[u8]) -> io::Result<()> {
    use std::io::Write;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(PRIVATE_MODE);
    }
    let mut file = options.open(temp)?;
    // umask 可能把 mode() 再削一刀，显式回设一次保证正好是 0600。
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(PRIVATE_MODE))?;
    }
    file.write_all(content)?;
    file.sync_all()
}

fn temp_path(path: &Path, dir: &Path) -> PathBuf {
    let stem = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!(".{stem}.tmp.{}.{seq}", std::process::id()))
}

/// rename 本身原子，但目录项要落盘才能扛住掉电；失败不影响本次写入结果。
fn sync_dir(dir: &Path) {
    if let Ok(handle) = std::fs::File::open(dir) {
        let _ = handle.sync_all();
    }
}

#[cfg(test)]
#[path = "atomic_file_tests.rs"]
mod tests;
