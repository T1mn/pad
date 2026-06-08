use super::support::{create_codex_threads_db, insert_codex_thread, stop_event, with_temp_home};
use crate::app::App;
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};
use crate::notify::NotificationRequest;

#[test]
fn completion_notification_uses_prompt_when_lookup_is_unavailable() {
    let request = super::super::notification::build_completion_notification(
        &AgentType::Codex,
        Some("missing-session"),
        Some("Ship the relay settings redesign with a compact layout"),
        Some("/tmp/demo"),
    );

    assert_eq!(request.title, "PAD · Codex complete");
    assert_eq!(
        request.body,
        "Ship the relay settings redesign with a compact layout"
    );
}

#[test]
fn completion_notification_falls_back_to_workdir_name() {
    let request = super::super::notification::build_completion_notification(
        &AgentType::OpenCode,
        None,
        None,
        Some("/tmp/pad-demo"),
    );

    assert_eq!(
        request,
        NotificationRequest {
            title: "PAD · OpenCode complete".into(),
            body: "pad-demo".into(),
        }
    );
}

#[test]
fn completion_notification_truncates_long_text() {
    let body = super::super::notification_text::completion_notification_body(
        &AgentType::Unknown,
        None,
        Some(
            "this is a very long prompt that should be truncated before it reaches the desktop notification surface because otherwise it becomes noisy",
        ),
        None,
    );

    assert!(body.ends_with("..."));
    assert!(body.chars().count() <= 75);
}

#[test]
fn completion_notification_collapses_prompt_whitespace() {
    let body = super::super::notification_text::completion_notification_body(
        &AgentType::Unknown,
        None,
        Some("ship\tthis\nprompt   now"),
        None,
    );

    assert_eq!(body, "ship this prompt now");
}

#[test]
fn completion_notification_prefers_latest_prompt_over_persisted_codex_title() {
    with_temp_home("notify-latest-prompt", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let db_path = codex_dir.join("state_5.sqlite");
        create_codex_threads_db(&db_path);
        insert_codex_thread(&db_path, "session-1", "Very old title");

        let request = super::super::notification::build_completion_notification(
            &AgentType::Codex,
            Some("session-1"),
            Some("Latest prompt should win over the old title"),
            Some("/tmp/demo"),
        );

        assert_eq!(request.body, "Latest prompt should win over the old title");
    });
}

#[test]
fn stop_hook_emits_completion_sound_event() {
    with_temp_home("completion-sound", |_home| {
        crate::sound::with_test_sound_capture(|| {
            let _ = crate::sound::take_test_playbacks();
            let mut app = App::new();
            app.panels.push(AgentPanel {
                session: "0".into(),
                window: "main".into(),
                window_index: "1".into(),
                pane: "1".into(),
                pane_id: "%1".into(),
                agent_type: AgentType::Codex,
                working_dir: "/tmp/demo".into(),
                is_active: true,
                state: AgentState::Busy,
                state_source: AgentStateSource::Scanner,
                transcript_path: None,
                cached_preview_turns: Default::default(),
                session_cache_state: None,
                git_info: None,
                pid: None,
                start_time: None,
                agent_session_id: Some("session-1".into()),
                last_user_prompt: Some("ship it".into()),
                last_assistant_message: None,
                has_unread_stop: false,
            });

            app.apply_hook_event(stop_event("%1"));

            assert_eq!(
                crate::sound::take_test_playbacks(),
                vec![crate::sound::TestPlayback {
                    event: Some(crate::sound::SoundEvent::Completion),
                    preset: "glass".into(),
                }]
            );
        });
    });
}

#[test]
fn stop_hook_adds_completion_to_notification_inbox() {
    let mut app = App::new();
    app.notification_inbox.entries.clear();
    app.panels.push(AgentPanel {
        session: "0".into(),
        window: "main".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: "%1".into(),
        agent_type: AgentType::Codex,
        working_dir: "/tmp/demo".into(),
        is_active: true,
        state: AgentState::Busy,
        state_source: AgentStateSource::Scanner,
        transcript_path: Some("/tmp/demo/transcript.jsonl".into()),
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        git_info: None,
        pid: None,
        start_time: None,
        agent_session_id: Some("session-1".into()),
        last_user_prompt: Some("ship it".into()),
        last_assistant_message: None,
        has_unread_stop: false,
    });

    app.apply_hook_event(stop_event("%1"));

    assert_eq!(app.notification_inbox.entries.len(), 1);
    let entry = &app.notification_inbox.entries[0];
    assert_eq!(entry.agent_type, "codex");
    assert_eq!(entry.event, "stop");
    assert_eq!(entry.body, "ship it");
    assert_eq!(entry.pane_id.as_deref(), Some("%1"));
    assert!(!entry.read);
}
