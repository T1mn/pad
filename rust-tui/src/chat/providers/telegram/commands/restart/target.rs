use super::super::{PadRestartTarget, PAD_DEFAULT_SESSION_NAME};

pub(super) fn current_pad_restart_target(
    current_exe: &std::path::Path,
) -> Result<PadRestartTarget, String> {
    let current_tmux_pane = std::env::var("TMUX_PANE")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let pad_status_pid = crate::runtime_status::read_status(&crate::paths::pad_status_path())
        .filter(|status| crate::runtime_status::process_alive(status.pid))
        .map(|status| status.pid);

    let panes = if current_tmux_pane.is_some() {
        Vec::new()
    } else if crate::tmux_dispatch::session_exists(PAD_DEFAULT_SESSION_NAME)
        .map_err(|err| err.to_string())?
    {
        crate::tmux_dispatch::list_session_panes(PAD_DEFAULT_SESSION_NAME)
            .map_err(|err| err.to_string())?
    } else {
        Vec::new()
    };

    let expected_command = current_exe
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("pad");

    Ok(select_pad_restart_target(
        current_tmux_pane.as_deref(),
        PAD_DEFAULT_SESSION_NAME,
        &panes,
        pad_status_pid,
        expected_command,
    ))
}

pub(crate) fn select_pad_restart_target(
    current_tmux_pane: Option<&str>,
    session_name: &str,
    session_panes: &[crate::tmux_dispatch::SessionPaneInfo],
    pad_pid: Option<u32>,
    expected_command: &str,
) -> PadRestartTarget {
    if let Some(pane_id) = current_tmux_pane.filter(|value| !value.trim().is_empty()) {
        return PadRestartTarget::RespawnPane(pane_id.to_string());
    }

    if let Some(pid) = pad_pid {
        if let Some(pane) = session_panes.iter().find(|pane| pane.pid == Some(pid)) {
            return PadRestartTarget::RespawnPane(pane.pane_id.clone());
        }
    }

    if let Some(pane) = session_panes
        .iter()
        .find(|pane| pane.command.trim() == expected_command)
    {
        return PadRestartTarget::RespawnPane(pane.pane_id.clone());
    }

    if let Some(first) = session_panes.first() {
        return PadRestartTarget::RespawnPane(first.pane_id.clone());
    }

    PadRestartTarget::NewDetachedSession(session_name.to_string())
}
