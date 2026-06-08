use super::super::{PadRestartPlan, PadRestartTarget};

pub(in crate::chat::providers::telegram::commands) fn execute_pad_restart_plan(
    plan: &PadRestartPlan,
) -> Result<(), String> {
    crate::log_debug!(
        "telegram: executing pad restart target={:?} start_dir={} command={}",
        plan.target,
        plan.start_dir,
        plan.shell_command
    );

    match &plan.target {
        PadRestartTarget::RespawnPane(pane_id) => {
            crate::tmux_dispatch::respawn_pane_shell(pane_id, &plan.start_dir, &plan.shell_command)
                .map_err(|err| err.to_string())
        }
        PadRestartTarget::NewDetachedSession(session_name) => {
            crate::tmux_dispatch::new_detached_session_shell(
                session_name,
                &plan.start_dir,
                &plan.shell_command,
            )
            .map_err(|err| err.to_string())
        }
    }
}
