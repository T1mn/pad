use std::io;
use std::os::fd::{AsRawFd, RawFd};

/// socket API 能执行 ssh / 向原生终端注入 prompt / 拉起进程，所以只接受和本进程同一个
/// uid 的本地连接；root 也不特殊放行（root 本来就能绕过一切，无需在这里开口子）。
pub(super) fn peer_uid_is_allowed(peer_uid: u32, owner_uid: u32) -> bool {
    peer_uid == owner_uid
}

pub(super) fn current_uid() -> u32 {
    // SAFETY: geteuid 无参数、无副作用，总是成功。
    unsafe { libc::geteuid() as u32 }
}

/// 校验通过返回对端 uid，否则返回错误；调用方必须直接关闭连接。
pub(crate) fn authorize_peer(stream: &impl AsRawFd) -> io::Result<u32> {
    let owner_uid = current_uid();
    let peer_uid = peer_uid(stream.as_raw_fd())?;
    if !peer_uid_is_allowed(peer_uid, owner_uid) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("peer uid {peer_uid} != owner uid {owner_uid}"),
        ));
    }
    Ok(peer_uid)
}

#[cfg(target_os = "linux")]
fn peer_uid(fd: RawFd) -> io::Result<u32> {
    let mut cred = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: cred/len 都是本地变量，长度与 SO_PEERCRED 期望的结构体一致。
    let rc = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            std::ptr::addr_of_mut!(cred).cast(),
            &mut len,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(cred.uid)
}

#[cfg(not(target_os = "linux"))]
fn peer_uid(fd: RawFd) -> io::Result<u32> {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    // SAFETY: 两个出参都是本地变量，getpeereid 只写入它们。
    let rc = unsafe { libc::getpeereid(fd, &mut uid, &mut gid) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(uid as u32)
}
