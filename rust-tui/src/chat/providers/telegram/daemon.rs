mod auth;
mod maintenance;
mod process;
mod run_loop;
mod state_io;
mod updates;

pub use process::{ensure_embedded_daemon_running, restart_daemon, sync_daemon};

pub async fn run_daemon() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run_loop::run_daemon_loop(false).await
}
