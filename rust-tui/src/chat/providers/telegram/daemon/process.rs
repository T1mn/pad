mod embedded;
mod external;
mod stop;

pub use embedded::ensure_embedded_daemon_running;
pub use external::{restart_daemon, sync_daemon};
