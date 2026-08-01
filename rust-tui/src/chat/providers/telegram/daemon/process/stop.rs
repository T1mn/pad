use crate::runtime_status::{self, ProcessStatus};
use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;

/// 读到一份状态文件之后能做的三种处置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StopPlan {
    /// 状态文件属于当前进程,留给它自己的 StatusGuard 收尾。
    KeepSelf,
    /// pid 已死,或身份对不上(pid 被系统回收给了别人):只清陈旧状态文件。
    CleanupStale,
    /// 身份对账通过,可以安全终止。
    Terminate,
}

pub fn stop_daemon() -> io::Result<bool> {
    stop_daemon_at(
        &crate::paths::telegram_bot_status_path(),
        &crate::paths::telegram_hook_socket_path(),
        terminate_process,
    )
}

fn stop_daemon_at(
    status_path: &Path,
    socket_path: &Path,
    terminate: impl Fn(&ProcessStatus) -> io::Result<()>,
) -> io::Result<bool> {
    // Serialize the read/terminate/cleanup sequence with a daemon startup.
    // The daemon only holds this lock while claiming the status file, so an
    // active daemon can still be stopped here.
    let _status_lock = runtime_status::StatusLock::acquire(status_path)?;
    let status = runtime_status::read_status(status_path);
    let plan = plan_stop(
        status.as_ref(),
        std::process::id(),
        runtime_status::status_process_alive,
    );

    let stopped = match (plan, status.as_ref()) {
        (StopPlan::KeepSelf, _) => return Ok(false),
        (StopPlan::Terminate, Some(status)) => {
            terminate(status)?;
            true
        }
        _ => false,
    };

    remove_status_file(status_path, status.as_ref())?;
    remove_inactive_socket_file(socket_path)?;
    Ok(stopped)
}

fn plan_stop(
    status: Option<&ProcessStatus>,
    self_pid: u32,
    owner_alive: impl Fn(&ProcessStatus) -> bool,
) -> StopPlan {
    let Some(status) = status else {
        return StopPlan::CleanupStale;
    };
    if status.pid == self_pid {
        return StopPlan::KeepSelf;
    }
    if owner_alive(status) {
        StopPlan::Terminate
    } else {
        StopPlan::CleanupStale
    }
}

pub(in crate::chat::providers::telegram::daemon::process) fn stop_external_daemon_if_running(
) -> io::Result<bool> {
    let status_path = crate::paths::telegram_bot_status_path();
    match runtime_status::read_status(&status_path) {
        Some(status) if status.pid != std::process::id() => stop_daemon(),
        _ => Ok(false),
    }
}

/// 每次发信号前都重新对账:SIGTERM 之后 pid 同样可能被回收给别的进程。
#[cfg(unix)]
fn terminate_process(status: &ProcessStatus) -> io::Result<()> {
    let _ = send_signal_if_owner(status, libc::SIGTERM)?;
    wait_for_exit(status, 20);

    if runtime_status::status_process_alive(status) {
        let _ = send_signal_if_owner(status, libc::SIGKILL)?;
        wait_for_exit(status, 10);
    }

    if runtime_status::status_process_alive(status) {
        return Err(io::Error::other(format!(
            "telegram daemon pid {} did not exit",
            status.pid
        )));
    }
    Ok(())
}

#[cfg(not(unix))]
fn terminate_process(_status: &ProcessStatus) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "stopping a Telegram daemon is unsupported on this platform",
    ))
}

#[cfg(unix)]
fn send_signal_if_owner(status: &ProcessStatus, signal: libc::c_int) -> io::Result<bool> {
    // This check must stay immediately before kill: after SIGTERM the pid may
    // disappear and be recycled before the SIGKILL escalation.
    if !runtime_status::status_process_alive(status) {
        return Ok(false);
    }

    let result = unsafe { libc::kill(status.pid as libc::pid_t, signal) };
    if result == 0 {
        return Ok(true);
    }

    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        // The owner exited between validation and kill. Treat it as stopped;
        // importantly, do not retry the signal against a recycled pid.
        Ok(false)
    } else {
        Err(error)
    }
}

fn wait_for_exit(status: &ProcessStatus, attempts: usize) {
    for _ in 0..attempts {
        if !runtime_status::status_process_alive(status) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn remove_status_file(status_path: &Path, expected: Option<&ProcessStatus>) -> io::Result<()> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let Some(current) = runtime_status::read_status(status_path) else {
        return Ok(());
    };
    if current.pid != expected.pid || current.started_at != expected.started_at {
        return Ok(());
    }

    match fs::remove_file(status_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn remove_inactive_socket_file(socket_path: &Path) -> io::Result<()> {
    if !socket_path.exists() {
        return Ok(());
    }
    if super::super::super::daemon_socket_is_active() {
        return Err(io::Error::other(
            "telegram direct hook socket is still active",
        ));
    }
    match fs::remove_file(socket_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
#[path = "stop_tests.rs"]
mod tests;
