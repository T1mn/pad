use super::*;

fn panel_with_state(state: AgentState, source: AgentStateSource) -> AgentPanel {
    AgentPanel {
        session: "0".into(),
        window: "main".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: "%1".into(),
        agent_type: crate::model::AgentType::Aider,
        working_dir: "/tmp/demo".into(),
        is_active: matches!(state, AgentState::Busy),
        state,
        state_source: source,
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

fn warm_app_with_panel() -> App {
    let mut app = App::new();
    app.sidebar.display_session_scope = "live".into();
    app.sidebar.thread_list_view = crate::app::state::ThreadListView::Normal;
    app.sidebar.app_thread_activity.clear();
    app.invalidate_sidebar_cache();
    app.apply_scan_panels(vec![panel_with_state(
        AgentState::Idle,
        AgentStateSource::Scanner,
    )]);
    app.dirty = false;
    app.preview.priority_refresh = false;
    app.preview.plain_cache = Some(crate::app::PreviewPlainCache {
        target_key: "live:%1".into(),
        width: 80,
        theme_name: app.theme.name.to_string(),
        content_revision: app.preview.content_revision,
        lines: Vec::new(),
        wrapped_rows: 1,
    });
    app
}

#[test]
fn only_busy_hook_state_is_preserved_across_scan() {
    assert!(should_preserve_hook_state(&panel_with_state(
        AgentState::Busy,
        AgentStateSource::Hook,
    )));
    assert!(!should_preserve_hook_state(&panel_with_state(
        AgentState::Waiting,
        AgentStateSource::Hook,
    )));
    assert!(!should_preserve_hook_state(&panel_with_state(
        AgentState::Idle,
        AgentStateSource::Hook,
    )));
    assert!(!should_preserve_hook_state(&panel_with_state(
        AgentState::Busy,
        AgentStateSource::Scanner,
    )));
}

#[test]
fn identical_scan_result_keeps_preview_cache_and_dirty_clear() {
    let mut app = warm_app_with_panel();
    let same_panels = app.panels.clone();

    app.apply_scan_panels(same_panels);

    assert!(!app.dirty);
    assert!(!app.preview.priority_refresh);
    assert!(app.preview.plain_cache.is_some());
}

#[test]
fn changed_scan_result_invalidates_preview_once() {
    let mut app = warm_app_with_panel();
    let mut changed_panels = app.panels.clone();
    changed_panels[0].state = AgentState::Busy;
    changed_panels[0].is_active = true;

    app.apply_scan_panels(changed_panels);

    assert!(app.dirty);
    assert!(app.preview.priority_refresh);
    assert!(app.preview.plain_cache.is_none());
}
