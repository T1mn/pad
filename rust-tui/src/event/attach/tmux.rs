mod command;
mod query;
mod shell;
mod status {
    pub(in crate::event::attach) fn tmux_status_value(target_session: Option<&str>) -> String {
        let mut cmd = std::process::Command::new("tmux");
        cmd.arg("show").arg("-v");
        if let Some(target) = target_session {
            cmd.args(["-t", target]);
        }
        cmd.arg("status");
        cmd.output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    }

    pub(in crate::event::attach) fn apply_desired_status(
        desired_status: &str,
        current_status: &str,
        target_session: &str,
    ) -> Option<String> {
        if current_status.is_empty() {
            return None;
        }

        match desired_status {
            "show" if current_status != "on" => {
                let _ = std::process::Command::new("tmux")
                    .args(["set", "-t", target_session, "status", "on"])
                    .output();
                Some(current_status.to_string())
            }
            "hide" if current_status != "off" => {
                let _ = std::process::Command::new("tmux")
                    .args(["set", "-t", target_session, "status", "off"])
                    .output();
                Some(current_status.to_string())
            }
            _ => None,
        }
    }
}

pub(super) use command::{run_tmux_logged, run_tmux_success, summarize_log_text};
pub(crate) use query::current_tmux_pane_id;
pub(super) use query::writable_client_for_pane;
pub(super) use query::{current_tmux_session, current_tmux_window_target, tmux_target_snapshot};
pub(super) use shell::{shell_log_cmd, wait_for_zoom_flag_cmd, wrap_tmux_run_shell};
pub(super) use status::{apply_desired_status, tmux_status_value};
