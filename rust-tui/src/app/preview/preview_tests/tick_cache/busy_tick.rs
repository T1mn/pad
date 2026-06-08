#[test]
fn slow_frame_only_slows_busy_animation_instead_of_stopping_it() {
    let mut app = App::new();
    app.preview.view = PreviewView::SessionList;
    app.frame_budget_exceeded = true;
    app.last_busy_animation_tick = Instant::now() - Duration::from_secs(1);
    app.panels.push(AgentPanel {
        session: "0".into(),
        window: "1".into(),
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
        agent_session_id: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
    });

    assert_eq!(app.busy_animation_interval(), Duration::from_millis(240));
    app.last_busy_animation_tick = Instant::now() - Duration::from_millis(150);
    assert!(!app.should_tick_busy_animation());
    app.last_busy_animation_tick = Instant::now() - Duration::from_secs(1);
    assert!(app.should_tick_busy_animation());
}

#[test]
fn app_only_busy_thread_keeps_busy_animation_ticking() {
    let mut app = App::new();
    app.preview.view = PreviewView::SessionList;
    app.last_busy_animation_tick = Instant::now() - Duration::from_secs(1);
    app.sidebar.app_thread_activity.insert(
        "codex:app-thread".into(),
        ThreadActivityOverride {
            agent_type: AgentType::Codex,
            session_id: Some("app-thread".into()),
            transcript_path: None,
            working_dir: "/tmp/demo".into(),
            state: AgentState::Busy,
            is_active: true,
            last_user_prompt: None,
            last_assistant_message: None,
            updated_at: 1,
        },
    );

    assert!(app.should_tick_busy_animation());
}

#[test]
fn hidden_busy_threads_do_not_force_animation_redraws() {
    let mut app = App::new();
    app.preview.view = PreviewView::SessionList;
    app.last_busy_animation_tick = Instant::now() - Duration::from_secs(1);
    app.panels.push(AgentPanel {
        session: "0".into(),
        window: "1".into(),
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
        agent_session_id: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
    });
    app.sidebar.visible_sidebar_items_dirty = false;
    app.sidebar.visible_sidebar_items_cache.clear();

    assert!(!app.should_tick_busy_animation());
    assert_eq!(app.desired_tick_rate(), Duration::from_millis(120));
}

#[test]
fn busy_threads_use_moderate_tick_rate() {
    let mut app = App::new();
    app.preview.view = PreviewView::SessionDetail;
    app.panels.push(AgentPanel {
        session: "0".into(),
        window: "1".into(),
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
        agent_session_id: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
    });

    assert_eq!(app.desired_tick_rate(), Duration::from_millis(60));
}
