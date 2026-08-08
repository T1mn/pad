mod embedded {
    use super::stop::stop_external_daemon_if_running;
    use crate::log_debug;
    use std::io;
    use std::sync::{LazyLock, Mutex};
    use tokio::task::JoinHandle;

    static EMBEDDED_DAEMON: LazyLock<Mutex<Option<JoinHandle<()>>>> =
        LazyLock::new(|| Mutex::new(None));

    pub fn ensure_embedded_daemon_running() -> io::Result<bool> {
        stop_external_daemon_if_running()?;

        let mut handle_slot = EMBEDDED_DAEMON
            .lock()
            .map_err(|_| io::Error::other("telegram embedded daemon lock poisoned"))?;
        if let Some(handle) = handle_slot.as_ref() {
            if !handle.is_finished() {
                return Ok(false);
            }
        }

        let handle = tokio::spawn(async move {
            if let Err(err) = super::super::run_loop::run_daemon_loop(true).await {
                log_debug!("telegram: embedded daemon exited with error: {}", err);
            }
        });
        *handle_slot = Some(handle);
        Ok(true)
    }
}
mod external {
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
            .map(|status| runtime_status::status_process_alive(&status))
            .unwrap_or(false)
            || super::super::super::daemon_socket_is_active()
    }
}
mod stop;

pub use embedded::ensure_embedded_daemon_running;
pub use external::{restart_daemon, sync_daemon};
