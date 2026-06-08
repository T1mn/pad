use crate::model::AgentPanel;

pub(super) fn find_unique_session_id<'a>(
    panel: &AgentPanel,
    session_ids: impl Iterator<Item = &'a str>,
) -> Option<&'a str> {
    let mut unique = None;

    for session_id in session_ids {
        if is_subagent_session(panel, session_id) {
            continue;
        }
        match unique {
            None => unique = Some(session_id),
            Some(existing) if existing == session_id => {}
            Some(_) => return None,
        }
    }

    unique
}

fn is_subagent_session(panel: &AgentPanel, session_id: &str) -> bool {
    matches!(panel.agent_type, crate::model::AgentType::Codex)
        && crate::codex_state::subagent_parent_thread_id(session_id)
            .ok()
            .flatten()
            .is_some()
}
