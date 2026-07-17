use crate::app::App;
use crate::log_debug;

use super::super::super::tmux::writable_client_for_pane;

pub(super) struct InstallContext {
    pub(super) trace_id: String,
    pub(super) pad_pane_id: String,
    pub(super) pad_win_target: String,
    pub(super) pad_session: String,
    pub(super) pad_client: Option<String>,
}

impl InstallContext {
    pub(super) fn load(app: &mut App, target_pane_id: &str, target_session: &str) -> Option<Self> {
        let trace_id = app
            .same_session_trace_id
            .clone()
            .unwrap_or_else(|| crate::app::new_handoff_trace("attach"));
        app.same_session_trace_id = Some(trace_id.clone());

        let pad_pane_id = match std::env::var("TMUX_PANE") {
            Ok(id) => id,
            Err(_) => {
                log_debug!(
                    "handoff trace={} stage=attach.skip reason=tmux_pane_missing",
                    trace_id
                );
                return None;
            }
        };

        log_debug!(
            "handoff trace={} stage=attach.begin target_pane={} target_session={} pad_pane={}",
            trace_id,
            target_pane_id,
            target_session,
            pad_pane_id
        );

        let pad_win_target = tmux_display(&pad_pane_id, "#{session_name}:#{window_index}")?;
        if pad_win_target.is_empty() {
            log_debug!(
                "install_return_bindings: pad_win_target empty, pad_pane_id={}",
                pad_pane_id
            );
            return None;
        }

        let pad_session = tmux_display(&pad_pane_id, "#{session_name}")?;
        if pad_session.is_empty() {
            log_debug!(
                "install_return_bindings: pad_session empty, pad_pane_id={}",
                pad_pane_id
            );
            return None;
        }
        let pad_client = writable_client_for_pane(&pad_pane_id);

        Some(Self {
            trace_id,
            pad_pane_id,
            pad_win_target,
            pad_session,
            pad_client,
        })
    }
}

fn tmux_display(target: &str, format: &str) -> Option<String> {
    std::process::Command::new("tmux")
        .args(["display-message", "-t", target, "-p", format])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
}
