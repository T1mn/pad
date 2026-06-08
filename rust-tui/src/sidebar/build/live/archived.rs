use crate::model::{AgentPanel, AgentType};

pub(in crate::sidebar::build) fn should_hide_live_panel(panel: &AgentPanel) -> bool {
    let Some(session_id) = panel.agent_session_id.as_deref() else {
        return false;
    };

    match panel.agent_type {
        AgentType::Codex => crate::codex_state::thread_for_id(session_id)
            .ok()
            .flatten()
            .is_some_and(|thread| thread.archived),
        AgentType::Claude => crate::claude_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .is_some_and(|thread| thread.archived),
        AgentType::Gemini => crate::gemini_history::thread_for_id(session_id)
            .ok()
            .flatten()
            .is_some_and(|thread| thread.archived),
        _ => false,
    }
}
