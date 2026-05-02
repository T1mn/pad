use crate::app::App;
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};

pub(super) fn visible_item_keys(app: &mut App) -> Vec<String> {
    app.visible_sidebar_items_ref()
        .iter()
        .map(|item| item.key().to_string())
        .collect()
}

pub(super) fn sample_panel(pane_id: &str, working_dir: &str) -> AgentPanel {
    AgentPanel {
        session: "0".into(),
        window: "main".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: pane_id.into(),
        agent_type: AgentType::Codex,
        working_dir: working_dir.into(),
        is_active: true,
        state: AgentState::Idle,
        state_source: AgentStateSource::Scanner,
        transcript_path: None,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        git_info: None,
        pid: None,
        start_time: None,
        agent_session_id: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
    }
}

mod movement;
mod selection;
