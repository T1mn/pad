use crate::app::App;
use std::process::Command;

use super::super::bindings::{pad_sider_toggle_command, PAD_SIDER_TOGGLE_KEYS};
use super::super::shell::wrap_tmux_run_shell;
use super::return_cmd::build_return_run_shell_cmd;

#[allow(clippy::too_many_arguments)]
pub(in crate::session) fn install_return_bindings(
    app: &mut App,
    trace_id: &str,
    session_name: &str,
    target_window: &str,
    pad_pane: &str,
    pad_window: &str,
    pad_session: &str,
    status_restore_value: Option<&str>,
) {
    let run_shell_cmd = build_return_run_shell_cmd(
        app,
        trace_id,
        session_name,
        target_window,
        pad_pane,
        pad_window,
        pad_session,
        status_restore_value,
    );
    let bind_result = Command::new("tmux")
        .args(["bind-key", "-T", "root", "F12", "run-shell", &run_shell_cmd])
        .output();
    log_debug!(
        "handoff trace={} stage=create.bind_installed cmd={} result={:?}",
        trace_id,
        run_shell_cmd,
        bind_result.map(|o| o.status)
    );

    let _ = Command::new("tmux")
        .args(["bind-key", "-T", "root", "C-q", "run-shell", &run_shell_cmd])
        .output();
    let sider_cmd = wrap_tmux_run_shell(&pad_sider_toggle_command());
    for key in PAD_SIDER_TOGGLE_KEYS {
        let _ = Command::new("tmux")
            .args(["bind-key", "-T", "root", key, "run-shell", &sider_cmd])
            .output();
    }

    app.same_session_attached = true;
    log_debug!(
        "handoff trace={} stage=create.same_session_attached",
        trace_id
    );
}
