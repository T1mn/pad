use super::helpers::archive_deleted_thread;
use super::*;

#[test]
fn settings_search_matches_english_terms_under_chinese_locale() {
    let mut app = App::new();
    app.locale = Locale::ZhCN;

    app.settings_search = "theme".into();
    let theme_matches = app.filtered_settings_items();
    assert!(theme_matches.iter().any(|(id, _, _, _, _)| *id == "theme"));

    app.settings_search = "display settings".into();
    let display_matches = app.filtered_settings_items();
    assert!(display_matches
        .iter()
        .any(|(id, _, _, _, _)| *id == "display_mode"));

    app.settings_search = "relay".into();
    let relay_matches = app.filtered_settings_items();
    assert!(relay_matches.iter().any(|(id, _, _, _, _)| *id == "relay"));
}

#[test]
fn settings_list_hides_refresh_interval_item() {
    let app = App::new();

    assert!(!app
        .settings_items()
        .iter()
        .any(|(id, _, _, _, _)| *id == "refresh_interval"));
}

#[test]
fn settings_search_no_longer_matches_refresh_interval_aliases() {
    let mut app = App::new();
    app.settings_search = "refresh interval".into();
    assert!(!app
        .filtered_settings_items()
        .iter()
        .any(|(id, _, _, _, _)| *id == "refresh_interval"));

    app.settings_search = "interval".into();
    assert!(!app
        .filtered_settings_items()
        .iter()
        .any(|(id, _, _, _, _)| *id == "refresh_interval"));
}

#[test]
fn settings_detail_persists_when_filtered_value_changes() {
    let mut app = App::new();
    app.settings_open = true;
    app.mode = Mode::Settings;
    app.settings_search = "live".into();

    let items = app.filtered_settings_items();
    app.settings_selected = items
        .iter()
        .position(|(id, _, _, _, _)| *id == "display_mode")
        .expect("display_mode should match live search");

    app.enter_settings_detail();
    assert_eq!(
        app.current_settings_detail_kind(),
        Some(SettingsDetailKind::DisplayMode)
    );

    app.config.display.session_scope = "all".into();
    assert!(!app
        .filtered_settings_items()
        .iter()
        .any(|(id, _, _, _, _)| *id == "display_mode"));
    assert_eq!(
        app.current_settings_detail_kind(),
        Some(SettingsDetailKind::DisplayMode)
    );
}

#[test]
fn archive_deleted_thread_requires_supported_agent_and_session_id() {
    let base = SidebarThread {
        key: "k".into(),
        folder_key: "f".into(),
        working_dir: "/tmp".into(),
        folder_label: "tmp".into(),
        agent_type: AgentType::OpenCode,
        runtime_source: None,
        session_id: None,
        transcript_path: None,
        title: "t".into(),
        upstream_title: None,
        subtitle: None,
        title_override: None,
        note: None,
        tags: Vec::new(),
        pinned: false,
        updated_at: 0,
        sort_updated_at: 0,
        live_pane_id: Some("%1".into()),
        live_location: None,
        pid: None,
        git_info: None,
        state: crate::model::AgentState::Idle,
        is_active: false,
        cached_preview_turns: Vec::new(),
        session_cache_state: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
        archived: false,
    };

    assert!(!archive_deleted_thread(&base).unwrap());

    let supported_without_session = SidebarThread {
        agent_type: AgentType::Codex,
        ..base.clone()
    };
    assert!(!archive_deleted_thread(&supported_without_session).unwrap());

    let already_archived = SidebarThread {
        agent_type: AgentType::Codex,
        session_id: Some("sid".into()),
        archived: true,
        ..base
    };
    assert!(!archive_deleted_thread(&already_archived).unwrap());
}

#[test]
fn apply_deleted_panel_locally_removes_panel_immediately() {
    let mut app = App::new();
    app.panels = vec![
        crate::model::AgentPanel {
            session: "s".into(),
            window: "w".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: "%1".into(),
            agent_type: AgentType::Codex,
            working_dir: "/tmp/a".into(),
            is_active: true,
            state: crate::model::AgentState::Idle,
            state_source: crate::model::AgentStateSource::Scanner,
            transcript_path: None,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            git_info: None,
            pid: None,
            start_time: None,
            agent_session_id: Some("sid-1".into()),
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        },
        crate::model::AgentPanel {
            session: "s".into(),
            window: "w".into(),
            window_index: "1".into(),
            pane: "2".into(),
            pane_id: "%2".into(),
            agent_type: AgentType::Claude,
            working_dir: "/tmp/b".into(),
            is_active: false,
            state: crate::model::AgentState::Idle,
            state_source: crate::model::AgentStateSource::Scanner,
            transcript_path: None,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            git_info: None,
            pid: None,
            start_time: None,
            agent_session_id: Some("sid-2".into()),
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        },
    ];

    app.apply_deleted_panel_locally("%1");

    assert_eq!(app.panels.len(), 1);
    assert_eq!(app.panels[0].pane_id, "%2");
}
