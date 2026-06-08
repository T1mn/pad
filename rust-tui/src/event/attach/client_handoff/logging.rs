use crate::log_debug;
use crate::model::AgentPanel;

use super::super::tmux::{current_tmux_pane_id, current_tmux_window_target, tmux_target_snapshot};

pub(super) fn log_start(
    prefix: &str,
    target_window: &str,
    panel: &AgentPanel,
    current_session: &str,
) {
    log_debug!(
        "{}: start target_window={} target_pane={} current_session={} current_window={} current_pane={} target_snapshot={}",
        prefix,
        target_window,
        panel.pane_id,
        current_session,
        current_tmux_window_target().as_deref().unwrap_or("-"),
        current_tmux_pane_id().as_deref().unwrap_or("-"),
        tmux_target_snapshot(&panel.pane_id).as_deref().unwrap_or("-")
    );
}

pub(super) fn log_after_step(prefix: &str, stage: &str, target_pane: &str) {
    log_debug!(
        "{}: {} current_window={} current_pane={} target_snapshot={}",
        prefix,
        stage,
        current_tmux_window_target().as_deref().unwrap_or("-"),
        current_tmux_pane_id().as_deref().unwrap_or("-"),
        tmux_target_snapshot(target_pane).as_deref().unwrap_or("-")
    );
}

pub(super) fn log_complete(
    prefix: &str,
    target_window: &str,
    panel: &AgentPanel,
    should_zoom: bool,
) {
    log_debug!(
        "{}: handoff complete target_window={} target_pane={} should_zoom={} target_snapshot={}",
        prefix,
        target_window,
        panel.pane_id,
        should_zoom,
        tmux_target_snapshot(&panel.pane_id)
            .as_deref()
            .unwrap_or("-")
    );
}
