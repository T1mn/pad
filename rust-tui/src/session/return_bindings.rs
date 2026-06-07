use crate::app::App;
use std::process::Command;

use super::bindings::{
    current_root_binding, pad_sider_toggle_command, restore_binding_cmd, PAD_RETURN_BINDING_MARKER,
    PAD_SIDER_TOGGLE_KEYS,
};
use super::shell::{shell_trace_log_cmd, wrap_tmux_run_shell};

pub(super) fn save_current_return_bindings(app: &mut App) {
    app.saved_tmux_bindings.clear();
    if let Some(line) = current_root_binding("F12") {
        app.saved_tmux_bindings.push(line);
    }
    if let Some(line) = current_root_binding("C-q") {
        app.saved_tmux_bindings.push(line);
    }
    for key in PAD_SIDER_TOGGLE_KEYS {
        if let Some(line) = current_root_binding(key) {
            app.saved_tmux_bindings.push(line);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn install_return_bindings(
    app: &mut App,
    trace_id: &str,
    session_name: &str,
    target_window: &str,
    pad_pane: &str,
    pad_window: &str,
    pad_session: &str,
    status_restore_value: Option<&str>,
) {
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
    let run_shell_cmd = wrap_tmux_run_shell(&return_cmd);
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

fn saved_binding_restore_cmd(app: &App, key: &str) -> String {
    restore_binding_cmd(
        app.saved_tmux_bindings
            .iter()
            .find(|line| line.contains(&format!(" {} ", key)))
            .map(String::as_str),
        key,
    )
}
