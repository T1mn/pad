mod auth;
mod maintenance;
mod process;
mod run_loop;
mod state_io;
mod updates;

pub use process::{
    daemon_is_running, ensure_daemon_running, ensure_embedded_daemon_running, restart_daemon,
    stop_daemon, sync_daemon,
};

pub async fn run_daemon() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run_loop::run_daemon_loop(false).await
}
