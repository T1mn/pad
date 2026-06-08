use std::io;
#[cfg(unix)]
use std::process::Command;

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
