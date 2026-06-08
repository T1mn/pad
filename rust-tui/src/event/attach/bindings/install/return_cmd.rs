use crate::tmux_bindings::{restore_binding_cmd, PAD_RETURN_BINDING_MARKER};

use super::super::super::tmux::{shell_log_cmd, wait_for_zoom_flag_cmd, wrap_tmux_run_shell};
use super::context::InstallContext;
use super::saved::SavedBindings;

pub(super) fn build_return_run_shell_cmd(
    ctx: &InstallContext,
    target_pane_id: &str,
    target_session: &str,
    saved_bindings: &SavedBindings,
    restore_zoom_cmd: &str,
    restore_status_cmd: &str,
) -> String {
    let mut restore_parts = Vec::new();
    restore_parts.push(shell_log_cmd(&format!(
        "[handoff trace={}] stage=return.begin target_session={} target_pane={} pad_session={} pad_window={} pad_pane={}",
        ctx.trace_id,
        target_session,
        target_pane_id,
        ctx.pad_session,
        ctx.pad_win_target,
        ctx.pad_pane_id
    )));
    restore_parts.push(restore_binding_cmd(saved_bindings.f12.as_deref(), "F12"));
    restore_parts.push(restore_binding_cmd(saved_bindings.cq.as_deref(), "C-q"));
    for (key, saved_binding) in &saved_bindings.sider {
        restore_parts.push(restore_binding_cmd(saved_binding.as_deref(), key));
    }

    push_zoom_restore(&mut restore_parts, target_pane_id, restore_zoom_cmd);
    if !restore_status_cmd.is_empty() {
        restore_parts.push(restore_status_cmd.to_string());
    }
    push_pad_return(&mut restore_parts, ctx, target_session);

    let return_cmd = format!("{} {}", PAD_RETURN_BINDING_MARKER, restore_parts.join("; "));
    wrap_tmux_run_shell(&return_cmd)
}

fn push_zoom_restore(
    restore_parts: &mut Vec<String>,
    target_pane_id: &str,
    restore_zoom_cmd: &str,
) {
    if restore_zoom_cmd.is_empty() {
        return;
    }

    restore_parts.push(shell_log_cmd(&format!(
        "before_unzoom target_pane={}",
        target_pane_id
    )));
    restore_parts.push(restore_zoom_cmd.to_string());
    restore_parts.push(shell_log_cmd(&format!(
        "after_unzoom target_pane={}",
        target_pane_id
    )));
    restore_parts.push(wait_for_zoom_flag_cmd(
        target_pane_id,
        "0",
        "after_unzoom_wait",
    ));
}

fn push_pad_return(restore_parts: &mut Vec<String>, ctx: &InstallContext, target_session: &str) {
    if target_session != ctx.pad_session {
        restore_parts.push(shell_log_cmd(&format!(
            "before_return_switch target_session={} pad_session={}",
            target_session, ctx.pad_session
        )));
        restore_parts.push(format!("tmux switch-client -t '{}'", ctx.pad_session));
        restore_parts.push(shell_log_cmd(&format!(
            "after_return_switch target_session={} pad_session={}",
            target_session, ctx.pad_session
        )));
    }

    restore_parts.push(shell_log_cmd(&format!(
        "before_return_select pad_window={} pad_pane={}",
        ctx.pad_win_target, ctx.pad_pane_id
    )));
    restore_parts.push(format!("tmux select-window -t '{}'", ctx.pad_win_target));
    restore_parts.push(format!("tmux select-pane -t '{}'", ctx.pad_pane_id));
    restore_parts.push(shell_log_cmd(&format!(
        "after_return_select pad_window={} pad_pane={}",
        ctx.pad_win_target, ctx.pad_pane_id
    )));
}
