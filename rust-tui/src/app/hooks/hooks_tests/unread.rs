use super::support::stop_event;
use crate::app::state::FocusTarget;
use crate::app::App;
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};

fn panel_for_unread_test(state: AgentState) -> AgentPanel {
    AgentPanel {
        session: "0".into(),
        window: "main".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: "%1".into(),
        agent_type: AgentType::Codex,
        working_dir: "/tmp/demo".into(),
        is_active: matches!(state, AgentState::Busy),
        state,
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

#[test]
fn stop_hook_marks_panel_unread_when_panel_item_is_not_focused() {
    let mut app = App::new();
    app.panels.push(panel_for_unread_test(AgentState::Busy));
    app.table_state.select(Some(0));
    app.preview.focus = FocusTarget::Preview;

    app.apply_hook_event(stop_event("%1"));

    assert!(app.panels[0].has_unread_stop);
}

#[test]
fn focusing_panel_clears_unread_stop_marker() {
    let mut app = App::new();
    let mut panel = panel_for_unread_test(AgentState::Waiting);
    panel.state_source = AgentStateSource::Hook;
    panel.last_assistant_message = Some("done".into());
    panel.has_unread_stop = true;
    app.panels.push(panel);
    app.table_state.select(Some(0));
    app.preview.focus = FocusTarget::Preview;

    app.focus_panel();

    assert!(!app.panels[0].has_unread_stop);
}
