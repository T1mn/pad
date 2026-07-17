use super::support::with_temp_home;
use crate::app::App;
use crate::hook::{HookEvent, HookTmuxInfo};
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};

fn panel_with_old_session() -> AgentPanel {
    AgentPanel {
        session: "0".into(),
        window: "main".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: "%1".into(),
        agent_type: AgentType::Codex,
        working_dir: "/tmp/demo".into(),
        is_active: false,
        state: AgentState::Idle,
        state_source: AgentStateSource::Scanner,
        transcript_path: Some("/tmp/old.jsonl".into()),
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        git_info: None,
        pid: None,
        start_time: None,
        agent_session_id: Some("old".into()),
        last_user_prompt: Some("old prompt".into()),
        last_assistant_message: Some("old answer".into()),
        has_unread_stop: false,
    }
}

fn new_session_start() -> HookEvent {
    HookEvent {
        event: "session_start".into(),
        turn_id: None,
        session_id: Some("new".into()),
        transcript_path: None,
        cwd: None,
        prompt: None,
        last_assistant_message: None,
        timestamp: None,
        tmux: HookTmuxInfo {
            pane_id: Some("%1".into()),
            session_name: Some("0".into()),
            window_index: Some("1".into()),
            pane_index: Some("1".into()),
            pane_current_path: Some("/tmp/demo".into()),
        },
    }
}

#[test]
fn new_session_start_does_not_inherit_prior_panel_snapshot() {
    with_temp_home("new-session-isolation", |_| {
        let mut app = App::new();
        app.panels.push(panel_with_old_session());

        app.apply_hook_event(new_session_start());

        let panel = &app.panels[0];
        assert_eq!(panel.agent_session_id.as_deref(), Some("new"));
        assert_eq!(panel.transcript_path, None);
        assert!(panel.cached_preview_turns.is_empty());
        assert_eq!(panel.last_user_prompt, None);
        assert_eq!(panel.last_assistant_message, None);

        let cached = crate::session_cache::find_cached_session("new").expect("new session cache");
        assert_eq!(cached.transcript_path, None);
        assert_eq!(cached.last_user_prompt, None);
        assert_eq!(cached.last_assistant_message, None);
    });
}
