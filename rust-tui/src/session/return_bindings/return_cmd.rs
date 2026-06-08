use crate::app::App;

use super::super::bindings::{PAD_RETURN_BINDING_MARKER, PAD_SIDER_TOGGLE_KEYS};
use super::super::shell::{shell_trace_log_cmd, wrap_tmux_run_shell};
use super::saved::saved_binding_restore_cmd;

#[allow(clippy::too_many_arguments)]
pub(super) fn build_return_run_shell_cmd(
    app: &App,
    trace_id: &str,
    session_name: &str,
    target_window: &str,
    pad_pane: &str,
    pad_window: &str,
    pad_session: &str,
    status_restore_value: Option<&str>,
) -> String {
    let restore_parts = return_binding_parts(
        app,
        trace_id,
        session_name,
        target_window,
        pad_pane,
        pad_window,
        pad_session,
        status_restore_value,
    );
    let return_cmd = format!("{} {}", PAD_RETURN_BINDING_MARKER, restore_parts.join("; "));
    wrap_tmux_run_shell(&return_cmd)
}

#[allow(clippy::too_many_arguments)]
fn return_binding_parts(
    app: &App,
    trace_id: &str,
    session_name: &str,
    target_window: &str,
    pad_pane: &str,
    pad_window: &str,
    pad_session: &str,
    status_restore_value: Option<&str>,
) -> Vec<String> {
    let mut restore_parts = Vec::new();
    restore_parts.push(shell_trace_log_cmd(
        trace_id,
        "return.begin",
        &format!(
            "target_session={} target_window={} target_pane={} pad_session={} pad_window={} pad_pane={}",
            session_name, target_window, pad_pane, pad_session, pad_window, pad_pane
        ),
    ));
    restore_parts.push(saved_binding_restore_cmd(app, "F12"));
    restore_parts.push(saved_binding_restore_cmd(app, "C-q"));
    for key in PAD_SIDER_TOGGLE_KEYS {
        restore_parts.push(saved_binding_restore_cmd(app, key));
    }
    if let Some(status) = status_restore_value {
        restore_parts.push(format!(
            "tmux set -t '{}' status '{}'",
            session_name, status
        ));
    }
    restore_parts.push(shell_trace_log_cmd(
        trace_id,
        "return.before_switch",
        &format!(
            "pad_session={} pad_window={} pad_pane={}",
            pad_session, pad_window, pad_pane
        ),
    ));
    restore_parts.push(format!("tmux switch-client -t '{}'", pad_session));
    restore_parts.push(format!("tmux select-window -t '{}'", pad_window));
    restore_parts.push(format!("tmux select-pane -t '{}'", pad_pane));
    restore_parts.push(shell_trace_log_cmd(
        trace_id,
        "return.after_select",
        &format!(
            "pad_session={} pad_window={} pad_pane={}",
            pad_session, pad_window, pad_pane
        ),
    ));
    restore_parts
}
