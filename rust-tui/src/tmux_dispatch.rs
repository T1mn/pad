mod pane;
mod prompt;
mod query;

use std::error::Error;
use std::process::Command;

pub use pane::{new_detached_session_shell, respawn_pane_shell, send_approval_key, send_escape};
pub use prompt::dispatch_prompt;
pub use query::{capture_pane_tail, list_session_panes, session_exists};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionPaneInfo {
    pub pane_id: String,
    pub pid: Option<u32>,
    pub command: String,
}

pub(super) fn run_tmux_with_output<const N: usize>(args: [&str; N]) -> Result<(), Box<dyn Error>> {
    let output = Command::new("tmux").args(args).output()?;
    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "tmux {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    )
    .into())
}
