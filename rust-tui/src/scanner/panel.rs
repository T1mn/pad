use super::tmux_panes::PaneLine;
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};

pub(super) fn panel_from_pane_line(pane_line: PaneLine<'_>, agent_type: AgentType) -> AgentPanel {
    AgentPanel {
        session: pane_line.session.to_string(),
        window: pane_line.window.to_string(),
        window_index: pane_line.window_index.to_string(),
        pane: pane_line.pane.to_string(),
        pane_id: pane_line.pane_id.to_string(),
        agent_type,
        working_dir: pane_line.working_dir.to_string(),
        is_active: false,
        state: AgentState::Idle,
        state_source: AgentStateSource::Scanner,
        transcript_path: None,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        git_info: None,
        pid: Some(pane_line.pane_pid.to_string()),
        start_time: Some(std::time::Instant::now()),
        agent_session_id: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
    }
}
