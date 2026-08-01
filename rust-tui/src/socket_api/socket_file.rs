use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener as StdUnixListener, UnixStream as StdUnixStream};
use std::path::{Path, PathBuf};

const SOCKET_MODE: u32 = 0o600;
const SOCKET_DIR_MODE: u32 = 0o700;

pub(crate) fn socket_is_live(socket_path: &Path) -> bool {
    StdUnixStream::connect(socket_path).is_ok()
}

/// 同步绑定 socket 并交回 listener：所在目录收到 0700，socket 文件 0600。
///
/// 走 "bind 临时名 -> chmod 0600 -> link 到正式名"，没有选另外两种做法：
/// - "bind 正式名后再 chmod" 有一段 0755 的窗口；
/// - "bind 前翻 umask" 是进程级副作用，pad 有一堆 tokio 线程在并发建文件，会误伤它们；
/// - 用 link 而不是 rename，因为 rename 会原子覆盖掉另一个实例正在监听的 socket，
///   而 link 碰到已存在的正式名直接 EEXIST，这一步本身就是所有权判据，
///   不需要先 exists() 再动手，也就没有 TOCTOU。
pub(crate) fn bind_private_listener(socket_path: &Path) -> io::Result<StdUnixListener> {
    harden_socket_dir(socket_path.parent().unwrap_or_else(|| Path::new(".")))?;
    match bind_via_staging(socket_path) {
        Err(err) if link_unsupported(&err) => bind_in_place(socket_path),
        result => result,
    }
}

fn bind_via_staging(socket_path: &Path) -> io::Result<StdUnixListener> {
    let staging = staging_path(socket_path);
    let _ = std::fs::remove_file(&staging);
    let listener = StdUnixListener::bind(&staging)?;
    let published = publish_socket(&staging, socket_path);
    // 正式名是硬链接，临时名用完就删；listener 仍然指向同一个 inode。
    let _ = std::fs::remove_file(&staging);
    published?;
    listener.set_nonblocking(true)?;
    Ok(listener)
}

/// 少数文件系统不给 socket 建硬链接时的退路：直接 bind 正式名再 chmod。
/// 此时 `~/.pad` 已经是 0700，窗口期内别的用户也走不进这个目录。
fn bind_in_place(socket_path: &Path) -> io::Result<StdUnixListener> {
    let listener = match StdUnixListener::bind(socket_path) {
        Ok(listener) => listener,
        Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
            reclaim_stale_socket(socket_path)?;
            StdUnixListener::bind(socket_path)?
        }
        Err(err) => return Err(err),
    };
    set_mode(socket_path, SOCKET_MODE)?;
    listener.set_nonblocking(true)?;
    Ok(listener)
}

fn publish_socket(staging: &Path, socket_path: &Path) -> io::Result<()> {
    set_mode(staging, SOCKET_MODE)?;
    match std::fs::hard_link(staging, socket_path) {
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            reclaim_stale_socket(socket_path)?;
            std::fs::hard_link(staging, socket_path)
        }
        result => result,
    }
}

fn reclaim_stale_socket(socket_path: &Path) -> io::Result<()> {
    if socket_is_live(socket_path) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "pad Unix socket already active at {}",
                socket_path.display()
            ),
        ));
    }
    match std::fs::remove_file(socket_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

/// 临时名带 pid，避免两个 pad 实例（或同进程的两个 socket）互相踩掉对方的 staging。
fn staging_path(socket_path: &Path) -> PathBuf {
    let stem = socket_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("pad-socket");
    socket_path.with_file_name(format!("{stem}.{}.tmp", std::process::id()))
}

fn link_unsupported(err: &io::Error) -> bool {
    // The staging name is longer than the published socket name. macOS
    // reports EINVAL/InvalidInput when only the staging name exceeds
    // SUN_LEN, so fall back to binding the final name directly.
    err.kind() == io::ErrorKind::Unsupported
        || err.kind() == io::ErrorKind::InvalidInput
        || matches!(
            err.raw_os_error(),
            Some(libc::EPERM) | Some(libc::EOPNOTSUPP) | Some(libc::EMLINK)
        )
}

fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

/// Linux 上 $HOME 默认 0755，`~/.pad` 必须自己收到 0700，否则同机用户能走进来 connect。
fn harden_socket_dir(dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mode = std::fs::metadata(dir)?.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        set_mode(dir, SOCKET_DIR_MODE)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "socket_file_tests.rs"]
mod socket_file_tests;
