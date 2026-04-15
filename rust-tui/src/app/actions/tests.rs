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

    app.settings_search = "completion sound".into();
    let sound_matches = app.filtered_settings_items();
    assert!(sound_matches.iter().any(|(id, _, _, _, _)| *id == "sound"));
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
fn settings_search_matches_trash_aliases() {
    let mut app = App::new();
    app.settings_search = "recycle bin".into();
    assert!(app
        .filtered_settings_items()
        .iter()
        .any(|(id, _, _, _, _)| *id == "trash"));
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
            cached_preview_turns: Default::default(),
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
            cached_preview_turns: Default::default(),
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
