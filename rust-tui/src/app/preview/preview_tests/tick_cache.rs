#[test]
fn preview_update_identical_plain_view_preserves_plain_cache() {
    let mut app = App::new();
    app.preview.source = PreviewSource::Tmux;
    app.preview.view = PreviewView::Plain;
    app.preview.pane_id = Some("%1".into());
    app.preview.content = "plain".into();
    app.preview.plain_cache = Some(crate::app::PreviewPlainCache {
        target_key: "%1".into(),
        width: 80,
        theme_name: app.theme.name.to_string(),
        content: "plain".into(),
        lines: vec![Line::from("plain")],
        wrapped_rows: 1,
    });

    send_preview_update(
        &mut app,
        PreviewUpdate {
            target_key: "%1".into(),
            live_pane_id: Some("%1".into()),
            content: "plain".into(),
            source: PreviewSource::Tmux,
            session_origin: None,
            session_id: None,
            turns: Default::default(),
            transcript_path: None,
            session_cache_state: None,
            updated_at: None,
        },
    );

    assert!(app.preview.plain_cache.is_some());
}

#[test]
fn detail_view_does_not_pause_busy_animations() {
    let mut app = App::new();
    app.preview.focus = FocusTarget::Preview;
    app.preview.expanded_turn = Some(0);
    app.preview.view = PreviewView::SessionDetail;
    assert!(!app.should_pause_busy_animations());

    app.preview.expanded_turn = None;
    app.preview.view = PreviewView::SessionList;
    assert!(!app.should_pause_busy_animations());

    app.preview.expanded_turn = Some(0);
    app.preview.view = PreviewView::SessionDetail;
    app.preview.focus = FocusTarget::Panel;
    assert!(!app.should_pause_busy_animations());
}

#[test]
fn detail_view_applies_preview_updates_immediately() {
    let mut app = App::new();
    app.preview.source = PreviewSource::Session;
    app.preview.pane_id = Some("live:%1".into());
    app.preview.session_id = Some("session-1".into());
    app.preview.turns = vec![PreviewTurn {
        question: "latest".into(),
        answer: None,
    }]
    .into();
    app.preview.selected_turn = Some(0);
    app.preview.expanded_turn = Some(0);
    app.preview.view = PreviewView::SessionDetail;

    let (tx, rx) = mpsc::channel(1);
    tx.blocking_send(PreviewUpdate {
        target_key: "live:%1".into(),
        live_pane_id: Some("%1".into()),
        content: "latest\nlatest answer".into(),
        source: PreviewSource::Session,
        session_origin: None,
        session_id: Some("session-1".into()),
        turns: vec![PreviewTurn {
            question: "latest".into(),
            answer: Some("latest answer".into()),
        }]
        .into(),
        transcript_path: None,
        session_cache_state: Some(SessionCacheState::Confirmed),
        updated_at: Some(42),
    })
    .unwrap();
    app.preview.rx = Some(rx);

    app.check_preview_result();

    assert!(app.preview.deferred_preview_update.is_none());
    assert_eq!(
        app.preview
            .turns
            .first()
            .and_then(|turn| turn.answer.as_deref()),
        Some("latest answer")
    );
    assert_eq!(app.preview.expanded_turn, Some(0));
}

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
fn detail_view_busy_threads_use_high_frequency_tick_rate() {
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

    assert_eq!(app.desired_tick_rate(), Duration::from_millis(16));
}

#[test]
fn thread_preview_cache_prunes_to_max_entries() {
    let mut app = App::new();
    let base_ts = 1_000_000i64;
    let total = THREAD_PREVIEW_CACHE_MAX_ENTRIES + 8;
    for i in 0..total {
        let ts = base_ts + i as i64;
        app.preview.thread_preview_cache.insert(
            format!("thread:{}", i),
            ThreadPreviewCacheEntry {
                turns: Default::default(),
                session_cache_state: None,
                transcript_path: None,
                session_id: None,
                updated_at: Some(ts),
                cached_at: ts,
            },
        );
    }

    assert!(app.prune_thread_preview_cache());
    assert_eq!(
        app.preview.thread_preview_cache.len(),
        THREAD_PREVIEW_CACHE_MAX_ENTRIES
    );
    assert!(app
        .preview
        .thread_preview_cache
        .contains_key(&format!("thread:{}", total - 1)));
    assert!(!app.preview.thread_preview_cache.contains_key("thread:0"));
}
