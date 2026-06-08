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
