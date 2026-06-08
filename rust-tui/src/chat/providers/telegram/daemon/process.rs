mod embedded;
mod external;
mod stop;

pub use embedded::ensure_embedded_daemon_running;
pub use external::{daemon_is_running, ensure_daemon_running, restart_daemon, sync_daemon};
pub use stop::stop_daemon;
