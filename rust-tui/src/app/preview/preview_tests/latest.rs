#[test]
fn open_latest_preview_turn_uses_selected_panel_cached_turns() {
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
        cached_preview_turns: vec![PreviewTurn {
            question: "latest".into(),
            answer: Some("- item".into()),
        }]
        .into(),
        session_cache_state: Some(SessionCacheState::Cached),
        git_info: None,
        pid: None,
        start_time: None,
        agent_session_id: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
    });

    app.preview.source = PreviewSource::Session;
    app.preview.pane_id = Some("%other".into());
    app.preview.turns = vec![PreviewTurn {
        question: "stale".into(),
        answer: Some("stale".into()),
    }]
    .into();

    assert!(app.open_latest_preview_turn());
    assert_eq!(app.preview.pane_id.as_deref(), Some("live:%1"));
    assert_eq!(app.preview.selected_turn, Some(0));
    assert_eq!(app.preview.expanded_turn, Some(0));
    assert_eq!(app.preview.turns[0].question, "latest");
}

#[test]
fn open_latest_preview_turn_prefers_newer_panel_cached_turns_over_current_preview() {
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
        state_source: AgentStateSource::Hook,
        transcript_path: None,
        cached_preview_turns: vec![PreviewTurn {
            question: "new prompt".into(),
            answer: None,
        }]
        .into(),
        session_cache_state: Some(SessionCacheState::Confirmed),
        git_info: None,
        pid: None,
        start_time: None,
        agent_session_id: Some("session-1".into()),
        last_user_prompt: Some("new prompt".into()),
        last_assistant_message: None,
        has_unread_stop: false,
    });
    app.table_state.select(Some(0));

    app.preview.source = PreviewSource::Session;
    app.preview.pane_id = Some("live:%1".into());
    app.preview.session_id = Some("session-1".into());
    app.preview.turns = vec![PreviewTurn {
        question: "old prompt".into(),
        answer: Some("old answer".into()),
    }]
    .into();

    assert!(app.open_latest_preview_turn());
    assert_eq!(
        app.preview.turns.first().map(|turn| turn.question.as_str()),
        Some("new prompt")
    );
    assert_eq!(
        app.preview
            .turns
            .first()
            .and_then(|turn| turn.answer.as_deref()),
        None
    );
}

