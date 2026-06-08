use crate::app::App;
use std::process::Command;

use super::super::bindings::{pad_sider_toggle_command, PAD_SIDER_TOGGLE_KEYS};
use super::super::shell::wrap_tmux_run_shell;
use super::context::ReturnBindingContext;
use super::return_cmd::build_return_run_shell_cmd;

pub(in crate::session) fn install_return_bindings(app: &mut App, ctx: &ReturnBindingContext<'_>) {
    let run_shell_cmd = build_return_run_shell_cmd(app, ctx);
    let bind_result = Command::new("tmux")
        .args(["bind-key", "-T", "root", "F12", "run-shell", &run_shell_cmd])
        .output();
    log_debug!(
        "handoff trace={} stage=create.bind_installed cmd={} result={:?}",
        ctx.trace_id,
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
        ctx.trace_id
    );
}
