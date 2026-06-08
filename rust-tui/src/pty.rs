mod attach;
mod capture;
mod keys;

pub use attach::attach_to_pane_pty;
pub use capture::capture_pane;
pub use keys::{find_detach_key, find_f12_key};
