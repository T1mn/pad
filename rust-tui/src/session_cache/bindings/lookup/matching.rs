use crate::model::AgentPanel;
use crate::session_cache::model::CachedPaneBinding;

const RECENT_BINDING_WINDOW_SECS: i64 = 2 * 60 * 60;

pub(super) fn exact_binding_matches(
    binding: &CachedPaneBinding,
    panel: &AgentPanel,
    now: i64,
) -> bool {
    if binding.pane_id != panel.pane_id {
        return false;
    }

    if pane_pid_matches(binding, panel) {
        return true;
    }

    binding_is_recent(binding, now) && binding_matches_slot(binding, panel)
}

pub(super) fn fallback_binding_matches(
    binding: &CachedPaneBinding,
    panel: &AgentPanel,
    now: i64,
) -> bool {
    binding_is_recent(binding, now) && binding_matches_slot(binding, panel)
}

fn binding_matches_slot(binding: &CachedPaneBinding, panel: &AgentPanel) -> bool {
    binding.path == panel.working_dir
        && binding.session_name == panel.session
        && binding.window_index == panel.window_index
        && binding.pane_index == panel.pane
}

fn pane_pid_matches(binding: &CachedPaneBinding, panel: &AgentPanel) -> bool {
    match (binding.pane_pid.as_deref(), panel.pid.as_deref()) {
        (Some(binding_pid), Some(panel_pid)) => !binding_pid.is_empty() && binding_pid == panel_pid,
        _ => false,
    }
}

fn binding_is_recent(binding: &CachedPaneBinding, now: i64) -> bool {
    binding.updated_at >= now.saturating_sub(RECENT_BINDING_WINDOW_SECS)
}
