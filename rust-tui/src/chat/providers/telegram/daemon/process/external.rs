use super::stop::stop_daemon;
use crate::log_debug;
use crate::runtime_status;
use crate::theme::Config;
use std::io;
use std::process::Stdio;

pub fn ensure_daemon_running(config: &Config) -> io::Result<bool> {
    if !config.telegram.enabled || config.telegram.bot_token.trim().is_empty() {
        return Ok(false);
    }
    if daemon_is_running() {
        return Ok(false);
    }

    let exe = std::env::current_exe()?;
    let child = std::process::Command::new(exe)
        .arg("telegram-bot")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    log_debug!("telegram: auto-started daemon pid={}", child.id());
    Ok(true)
}

pub fn sync_daemon(config: &Config) -> io::Result<bool> {
    if crate::chat::backend::pad_is_online() {
        let _ = super::embedded::ensure_embedded_daemon_running()?;
        return Ok(false);
    }
    if !config.telegram.enabled || config.telegram.bot_token.trim().is_empty() {
        return stop_daemon();
    }
    ensure_daemon_running(config)
}

pub fn restart_daemon(config: &Config) -> io::Result<bool> {
    if crate::chat::backend::pad_is_online() {
        let _ = super::embedded::ensure_embedded_daemon_running()?;
        return Ok(false);
    }
    let _ = stop_daemon()?;
    ensure_daemon_running(config)
}

pub fn daemon_is_running() -> bool {
    runtime_status::read_status(&crate::paths::telegram_bot_status_path())
        .map(|status| runtime_status::process_alive(status.pid))
        .unwrap_or(false)
        || super::super::super::daemon_socket_is_active()
}
