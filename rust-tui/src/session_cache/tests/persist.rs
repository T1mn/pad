use super::support::panel;
use crate::hook::{HookEvent, HookTmuxInfo};

fn session_start(session_id: &str) -> HookEvent {
    HookEvent {
        event: "session_start".to_string(),
        turn_id: None,
        session_id: Some(session_id.to_string()),
        transcript_path: None,
        cwd: None,
        prompt: None,
        last_assistant_message: None,
        timestamp: None,
        tmux: HookTmuxInfo {
            pane_id: Some("%1".to_string()),
            session_name: None,
            window_index: None,
            pane_index: None,
            pane_current_path: None,
        },
    }
}

fn panel_with_session(session_id: &str) -> crate::model::AgentPanel {
    let mut panel = panel("%1", "dev", "1", "0", "/repo");
    panel.agent_session_id = Some(session_id.to_string());
    panel.transcript_path = Some("/tmp/old.jsonl".to_string());
    panel.last_user_prompt = Some("old prompt".to_string());
    panel.last_assistant_message = Some("old answer".to_string());
    panel
}

#[test]
fn session_start_for_new_id_does_not_inherit_panel_session_state() {
    crate::test_support::with_temp_home("pad-session-cache-persist", "new-session", |_| {
        let panel = panel_with_session("old");

        let snapshot = super::super::persist_hook_event(&panel, &session_start("new"))
            .unwrap()
            .unwrap();

        assert_eq!(snapshot.agent_session_id, "new");
        assert_eq!(snapshot.transcript_path, None);
        assert!(snapshot.recent_turns.is_empty());
        assert_eq!(snapshot.last_user_prompt, None);
        assert_eq!(snapshot.last_assistant_message, None);
    });
}

#[test]
fn session_start_for_same_id_keeps_panel_fallbacks() {
    crate::test_support::with_temp_home("pad-session-cache-persist", "same-session", |_| {
        let panel = panel_with_session("same");

        let snapshot = super::super::persist_hook_event(&panel, &session_start("same"))
            .unwrap()
            .unwrap();

        assert_eq!(snapshot.transcript_path.as_deref(), Some("/tmp/old.jsonl"));
        assert_eq!(snapshot.recent_turns.len(), 1);
        assert_eq!(snapshot.recent_turns[0].question, "old prompt");
        assert_eq!(
            snapshot.recent_turns[0].answer.as_deref(),
            Some("old answer")
        );
        assert_eq!(snapshot.last_user_prompt.as_deref(), Some("old prompt"));
        assert_eq!(
            snapshot.last_assistant_message.as_deref(),
            Some("old answer")
        );
    });
}
