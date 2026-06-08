#[test]
fn preview_update_marks_dirty_when_only_panel_cache_state_changes() {
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
    });
    app.preview.content = "latest\nlatest answer".into();
    app.preview.source = PreviewSource::Session;
    app.preview.pane_id = Some("live:%1".into());
    app.preview.session_origin = Some(crate::model::PreviewSessionOrigin::Pane);
    app.preview.session_id = Some("session-1".into());
    app.preview.turns = vec![PreviewTurn {
        question: "latest".into(),
        answer: Some("latest answer".into()),
    }]
    .into();
    app.preview.view = PreviewView::SessionList;
    app.preview.follow_bottom = false;
    app.preview.follow_selection = true;
    app.dirty = false;

    send_preview_update(
        &mut app,
        PreviewUpdate {
            target_key: "live:%1".into(),
            live_pane_id: Some("%1".into()),
            content: "latest\nlatest answer".into(),
            source: PreviewSource::Session,
            session_origin: Some(crate::model::PreviewSessionOrigin::Pane),
            session_id: Some("session-1".into()),
            turns: vec![PreviewTurn {
                question: "latest".into(),
                answer: Some("latest answer".into()),
            }]
            .into(),
            transcript_path: None,
            session_cache_state: Some(SessionCacheState::Cached),
            updated_at: Some(42),
        },
    );

    assert!(app.dirty);
    assert_eq!(
        app.panels[0].session_cache_state,
        Some(SessionCacheState::Cached)
    );
}
