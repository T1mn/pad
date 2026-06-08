use super::super::target::SessionTarget;
use crate::model::{AgentPanel, AgentState, AgentStateSource};
use crate::preview_source::PreviewRequest;

pub(crate) fn persistence_panel_from_request(
    request: &PreviewRequest,
    target: &SessionTarget,
) -> Option<AgentPanel> {
    let pane_id = request.live_pane_id.clone()?;
    Some(AgentPanel {
        session: String::new(),
        window: String::new(),
        window_index: String::new(),
        pane: String::new(),
        pane_id,
        agent_type: request.agent_type.clone(),
        working_dir: request.working_dir.clone(),
        is_active: matches!(request.state, AgentState::Busy | AgentState::Waiting),
        state: request.state.clone(),
        state_source: AgentStateSource::Scanner,
        transcript_path: Some(target.transcript_path.to_string_lossy().to_string()),
        cached_preview_turns: request.cached_preview_turns.clone(),
        session_cache_state: request.session_cache_state,
        git_info: None,
        pid: None,
        start_time: None,
        agent_session_id: target.session_id.clone(),
        last_user_prompt: request
            .cached_preview_turns
            .first()
            .map(|turn| turn.question.clone()),
        last_assistant_message: request
            .cached_preview_turns
            .first()
            .and_then(|turn| turn.answer.clone()),
        has_unread_stop: false,
    })
}
