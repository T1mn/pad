use super::super::model::{CachedPaneBinding, HookBindingContext, SessionCacheIndex};
use crate::model::AgentPanel;

pub(in crate::session_cache) fn upsert_binding(
    index: &mut SessionCacheIndex,
    panel: &AgentPanel,
    agent_session_id: &str,
    ctx: HookBindingContext,
    now: i64,
) {
    let binding = CachedPaneBinding {
        agent_session_id: agent_session_id.to_string(),
        pane_id: panel.pane_id.clone(),
        pane_pid: panel.pid.clone(),
        session_name: ctx.session_name.unwrap_or_else(|| panel.session.clone()),
        window_index: ctx
            .window_index
            .unwrap_or_else(|| panel.window_index.clone()),
        pane_index: ctx.pane_index.unwrap_or_else(|| panel.pane.clone()),
        path: ctx.path.unwrap_or_else(|| panel.working_dir.clone()),
        agent_type: panel.agent_type.to_string(),
        updated_at: now,
    };

    if let Some(existing) = index
        .pane_bindings
        .iter_mut()
        .find(|item| item.pane_id == binding.pane_id)
    {
        *existing = binding;
        return;
    }

    if let Some(existing) = index.pane_bindings.iter_mut().find(|item| {
        item.agent_session_id == binding.agent_session_id
            && item.agent_type == binding.agent_type
            && item.session_name == binding.session_name
            && item.window_index == binding.window_index
            && item.pane_index == binding.pane_index
            && item.path == binding.path
    }) {
        *existing = binding;
        return;
    }

    index.pane_bindings.push(binding);
}
