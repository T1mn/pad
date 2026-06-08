use crate::runtime_status;
use std::fs;
use std::io;
use std::time::Duration;

pub fn stop_daemon() -> io::Result<bool> {
    let status_path = crate::paths::telegram_bot_status_path();
    let socket_path = crate::paths::telegram_hook_socket_path();
    let mut stopped = false;

    if let Some(status) = runtime_status::read_status(&status_path) {
        if status.pid == std::process::id() {
            return Ok(false);
        }
        if runtime_status::process_alive(status.pid) {
            stopped = true;
            terminate_process(status.pid)?;
        }
    }

    remove_status_file()?;
    remove_inactive_socket_file(&socket_path)?;
    Ok(stopped)
}

pub(in crate::chat::providers::telegram::daemon::process) fn stop_external_daemon_if_running(
) -> io::Result<bool> {
    let status_path = crate::paths::telegram_bot_status_path();
    match runtime_status::read_status(&status_path) {
        Some(status) if status.pid != std::process::id() => stop_daemon(),
        _ => Ok(false),
    }
}

fn terminate_process(pid: u32) -> io::Result<()> {
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as i32, libc::SIGTERM);
    }
    wait_for_exit(pid, 20);

    #[cfg(unix)]
    if runtime_status::process_alive(pid) {
        unsafe {
            libc::kill(pid as i32, libc::SIGKILL);
        }
        wait_for_exit(pid, 10);
    }

    if runtime_status::process_alive(pid) {
        return Err(io::Error::other(format!(
            "telegram daemon pid {} did not exit",
            pid
        )));
    }
    Ok(())
}

fn wait_for_exit(pid: u32, attempts: usize) {
    for _ in 0..attempts {
        if !runtime_status::process_alive(pid) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn remove_status_file() -> io::Result<()> {
    match fs::remove_file(crate::paths::telegram_bot_status_path()) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn remove_inactive_socket_file(socket_path: &std::path::Path) -> io::Result<()> {
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
