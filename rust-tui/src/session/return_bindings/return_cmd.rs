use crate::app::App;

use super::super::bindings::{return_binding_command, PAD_SIDER_TOGGLE_KEYS};
use super::super::shell::{shell_trace_log_cmd, wrap_tmux_run_shell};
use super::context::ReturnBindingContext;
use super::saved::saved_binding_restore_cmd;

pub(super) fn build_return_run_shell_cmd(app: &App, ctx: &ReturnBindingContext<'_>) -> String {
    let restore_parts = return_binding_parts(app, ctx);
    let return_cmd = return_binding_command(&restore_parts);
    wrap_tmux_run_shell(&return_cmd)
}

fn return_binding_parts(app: &App, ctx: &ReturnBindingContext<'_>) -> Vec<String> {
    let mut restore_parts = Vec::new();
    restore_parts.push(shell_trace_log_cmd(
        ctx.trace_id,
        "return.begin",
        &format!(
            "target_session={} target_window={} target_pane={} pad_session={} pad_window={} pad_pane={}",
            ctx.target_session,
            ctx.target_window,
            ctx.pad_pane,
            ctx.pad_session,
            ctx.pad_window,
            ctx.pad_pane
        ),
    ));
    restore_parts.push(saved_binding_restore_cmd(app, "F12"));
    restore_parts.push(saved_binding_restore_cmd(app, "C-q"));
    for key in PAD_SIDER_TOGGLE_KEYS {
        restore_parts.push(saved_binding_restore_cmd(app, key));
    }
    if let Some(status) = ctx.status_restore_value {
        restore_parts.push(format!(
            "tmux set -t '{}' status '{}'",
            ctx.target_session, status
        ));
    }
    restore_parts.push(shell_trace_log_cmd(
        ctx.trace_id,
        "return.before_switch",
        &format!(
            "pad_session={} pad_window={} pad_pane={}",
            ctx.pad_session, ctx.pad_window, ctx.pad_pane
        ),
    ));
    restore_parts.push(format!("tmux switch-client -t '{}'", ctx.pad_session));
    restore_parts.push(format!("tmux select-window -t '{}'", ctx.pad_window));
    restore_parts.push(format!("tmux select-pane -t '{}'", ctx.pad_pane));
    restore_parts.push(shell_trace_log_cmd(
        ctx.trace_id,
        "return.after_select",
        &format!(
            "pad_session={} pad_window={} pad_pane={}",
            ctx.pad_session, ctx.pad_window, ctx.pad_pane
        ),
    ));
    restore_parts
}
