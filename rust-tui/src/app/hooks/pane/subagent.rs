use crate::hook::HookEvent;
use crate::log_debug;
use crate::model::{AgentPanel, AgentType};

pub(super) fn handle_codex_subagent_event(
    panel: &mut AgentPanel,
    event: &HookEvent,
    pane_id: &str,
) -> bool {
    if panel.agent_type != AgentType::Codex {
        return false;
    }

    let Some(subagent_session_id) = event.session_id.as_deref() else {
        return false;
    };
    let Ok(Some(parent_thread_id)) =
        crate::codex_state::subagent_parent_thread_id(subagent_session_id)
    else {
        return false;
    };

    if panel.agent_session_id.is_none()
        || panel.agent_session_id.as_deref() == Some(subagent_session_id)
    {
        panel.agent_session_id = Some(parent_thread_id.clone());
    }
    log_debug!(
        "hook: ignoring codex subagent event pane={} subagent_session={} parent_session={}",
        pane_id,
        subagent_session_id,
        parent_thread_id
    );
    true
}
