use crate::app::App;

use super::super::status::{apply_desired_status, tmux_status_value};

pub(super) fn apply_agent_status_style(
    app: &mut App,
    session_name: &str,
    pad_session: Option<&str>,
) -> Option<String> {
    let current_status = tmux_status_value(session_name);
    let desired_status = app.config.desired_agent_style.status.as_str();
    let keep_source_status = if desired_status == "keep" {
        pad_session.map(tmux_status_value)
    } else {
        None
    };
    let status_restore_value = apply_desired_status(
        desired_status,
        &current_status,
        keep_source_status.as_deref(),
        session_name,
    );
    app.saved_tmux_status = status_restore_value.clone();
    app.saved_tmux_status_target = status_restore_value
        .as_ref()
        .map(|_| session_name.to_string());
    status_restore_value
}
