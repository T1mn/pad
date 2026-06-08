use super::run_tmux_with_output;
use std::error::Error;
use std::process::Command;

pub fn send_escape(pane_id: &str) -> Result<(), Box<dyn Error>> {
    run_tmux_with_output(["send-keys", "-t", pane_id, "Escape"])?;
    log_debug!("tmux_dispatch: escape sent pane={}", pane_id);
    Ok(())
}

pub fn send_approval_key(pane_id: &str, key: &str) -> Result<(), Box<dyn Error>> {
    run_tmux_with_output(["send-keys", "-t", pane_id, key])?;
    log_debug!(
        "tmux_dispatch: approval key sent pane={} key={}",
        pane_id,
        key
    );
    Ok(())
}

pub fn respawn_pane_shell(
    pane_id: &str,
    start_dir: &str,
    shell_command: &str,
) -> Result<(), Box<dyn Error>> {
    let mut command = Command::new("tmux");
    command.args(["respawn-pane", "-k", "-t", pane_id]);
    if !start_dir.trim().is_empty() {
        command.args(["-c", start_dir]);
    }
    command.arg(shell_command);

    let output = command.output()?;
    if output.status.success() {
        log_debug!(
            "tmux_dispatch: respawned pane={} start_dir={} command={}",
            pane_id,
            start_dir,
            shell_command
        );
        return Ok(());
    }

    Err(format!(
        "tmux respawn-pane failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
    .into())
}

pub fn new_detached_session_shell(
    session_name: &str,
    start_dir: &str,
    shell_command: &str,
) -> Result<(), Box<dyn Error>> {
    let mut command = Command::new("tmux");
    command.args(["new-session", "-d", "-s", session_name]);
    if !start_dir.trim().is_empty() {
        command.args(["-c", start_dir]);
    }
    command.arg(shell_command);

    let output = command.output()?;
    if output.status.success() {
        log_debug!(
            "tmux_dispatch: created detached session={} start_dir={} command={}",
            session_name,
            start_dir,
            shell_command
        );
        return Ok(());
    }

    Err(format!(
        "tmux new-session failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
    .into())
}
