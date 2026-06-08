mod context;
mod return_cmd;
mod saved;
mod zoom;

use crate::app::App;
use crate::log_debug;
use crate::tmux_bindings::{pad_sider_toggle_command, PAD_SIDER_TOGGLE_KEYS};

use super::super::tmux::{
    apply_desired_status, run_tmux_logged, tmux_status_value, wrap_tmux_run_shell,
};
use context::InstallContext;
use return_cmd::build_return_run_shell_cmd;
use saved::SavedBindings;
use zoom::ZoomDecision;

/// Install F12/C-q/F10/C-Tab bindings for same-session attach.
/// Snapshots zoom and status bar state, modifies them for the attach,
/// and encodes restoration into the return command.
pub(in crate::event::attach) fn install_return_bindings(
    app: &mut App,
    target_pane_id: &str,
    target_session: &str,
) -> bool {
    let Some(ctx) = InstallContext::load(app, target_pane_id, target_session) else {
        return false;
    };

    let zoom = ZoomDecision::for_target(app, target_pane_id);
    let saved_bindings = SavedBindings::capture_into_app(app);
    let (status_val, desired_status, status_restore_value, restore_status_cmd) =
        apply_attach_status(app, target_session);

    log_debug!(
        "install_return_bindings: target={} target_session={} panes={} zoomed={} should_zoom={} status={} desired_status={} status_restore={} pad_session={} pad_win={}",
        target_pane_id,
        target_session,
        zoom.pane_count,
        zoom.already_zoomed,
        zoom.should_zoom,
        status_val,
        desired_status,
        status_restore_value.as_deref().unwrap_or("-"),
        ctx.pad_session,
        ctx.pad_win_target
    );

    let run_shell_cmd = build_return_run_shell_cmd(
        &ctx,
        target_pane_id,
        target_session,
        &saved_bindings,
        &zoom.restore_zoom_cmd,
        &restore_status_cmd,
    );
    install_return_keys(&run_shell_cmd);
    install_sider_toggle_keys();

    log_debug!(
        "handoff trace={} stage=attach.return_cmd cmd={}",
        ctx.trace_id,
        run_shell_cmd
    );
    zoom.should_zoom
}

fn apply_attach_status(
    app: &mut App,
    target_session: &str,
) -> (String, String, Option<String>, String) {
    let status_val = tmux_status_value(Some(target_session));
    let desired_status = app.config.desired_agent_style.status.clone();
    let status_restore_value = apply_desired_status(&desired_status, &status_val, target_session);

    app.saved_tmux_status = status_restore_value.clone();
    app.saved_tmux_status_target = status_restore_value
        .as_ref()
        .map(|_| target_session.to_string());

    let restore_status_cmd = status_restore_value
        .as_ref()
        .map(|status| format!("tmux set -t '{}' status '{}'", target_session, status))
        .unwrap_or_default();

    (
        status_val,
        desired_status,
        status_restore_value,
        restore_status_cmd,
    )
}

fn install_return_keys(run_shell_cmd: &str) {
    bind_root_key(
        "install_return_bindings.bind_f12",
        "F12",
        run_shell_cmd.to_string(),
    );
    bind_root_key(
        "install_return_bindings.bind_cq",
        "C-q",
        run_shell_cmd.to_string(),
    );
}

fn install_sider_toggle_keys() {
    let sider_cmd = wrap_tmux_run_shell(&pad_sider_toggle_command());
    for key in PAD_SIDER_TOGGLE_KEYS {
        bind_root_key(
            &format!("install_return_bindings.bind_sider_{}", key),
            key,
            sider_cmd.clone(),
        );
    }
}

fn bind_root_key(context: &str, key: &str, run_shell_cmd: String) {
    let _ = run_tmux_logged(
        context,
        vec![
            "bind-key".to_string(),
            "-T".to_string(),
            "root".to_string(),
            key.to_string(),
            "run-shell".to_string(),
            run_shell_cmd,
        ],
    );
}
