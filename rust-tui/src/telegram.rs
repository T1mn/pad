use crate::theme::Config;
use std::io;

pub async fn run_daemon() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    crate::chat::providers::telegram::run_daemon().await
}

pub fn ensure_embedded_daemon_running() -> io::Result<bool> {
    crate::chat::providers::telegram::ensure_embedded_daemon_running()
}

pub fn sync_daemon(config: &Config) -> io::Result<bool> {
    crate::chat::providers::telegram::sync_daemon(config)
}

pub fn restart_daemon(config: &Config) -> io::Result<bool> {
    crate::chat::providers::telegram::restart_daemon(config)
}
