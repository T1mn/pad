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
