use std::error::Error;
use std::process::{Command, Output};

use super::tmux::current_tmux_client_snapshot;

pub(super) struct TargetInfo {
    pub(super) window: String,
    pub(super) pane: Option<String>,
}

pub(super) fn create_tmux_target(
    path: &str,
    agent_cmd: &str,
    session_name: &str,
    launch_after_attach: bool,
) -> Result<Output, Box<dyn Error>> {
    let target_format = "#{session_name}:#{window_index} #{pane_id}";
    let check = Command::new("tmux")
        .args(["has-session", "-t", session_name])
        .output()?;

    let mut cmd = Command::new("tmux");
    if check.status.success() {
        log_debug!(
            "session: session '{}' already exists, opening new window",
            session_name
        );
        cmd.args([
            "new-window",
            "-P",
            "-F",
            target_format,
            "-t",
            session_name,
            "-c",
            path,
        ]);
    } else {
        log_debug!("session: creating new session '{}'", session_name);
        cmd.args([
            "new-session",
            "-d",
            "-P",
            "-F",
            target_format,
            "-s",
            session_name,
            "-c",
            path,
        ]);
    }

    if !launch_after_attach {
        cmd.arg(agent_cmd);
    }

    let out = cmd.output()?;
    log_debug!(
        "session: create target status={} stderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr).trim()
    );
    Ok(out)
}

pub(super) fn parse_target_info(output: &Output, session_name: &str) -> TargetInfo {
    let target_info = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let mut target_parts = target_info.split_whitespace();
    let window = target_parts
        .next()
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}:0", session_name));
    let pane = target_parts.next().map(str::to_string);
    TargetInfo { window, pane }
}

pub(super) fn select_target_window(target: &TargetInfo) {
    let _ = Command::new("tmux")
        .args(["select-window", "-t", &target.window])
        .output();
    if let Some(target_pane) = target.pane.as_deref() {
        let _ = Command::new("tmux")
            .args(["select-pane", "-t", target_pane])
            .output();
    }
}

pub(super) fn switch_client_to_target(trace_id: &str, session_name: &str) {
    log_debug!(
        "handoff trace={} stage=create.before_switch snapshot={}",
        trace_id,
        current_tmux_client_snapshot().as_deref().unwrap_or("-")
    );

    let sw = Command::new("tmux")
        .args(["switch-client", "-t", session_name])
        .output();
    log_debug!(
        "handoff trace={} stage=create.switch_client target_session={} result={:?}",
        trace_id,
        session_name,
        sw.map(|o| o.status)
    );

    log_debug!(
        "handoff trace={} stage=create.after_switch snapshot={}",
        trace_id,
        current_tmux_client_snapshot().as_deref().unwrap_or("-")
    );
}
