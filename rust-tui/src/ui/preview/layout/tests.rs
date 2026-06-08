use super::{
    info_card::preview_info_value_text_at, provider::preview_provider_value,
    selection::preview_visible_plain_text_rows,
};
use crate::app::App;
use crate::model::{AgentState, AgentType, PreviewSource, PreviewView};
use crate::sidebar::SidebarThread;
use crate::theme::ProviderConfig;
use ratatui::layout::Rect;

#[test]
fn preview_plain_visible_rows_respects_scroll_window_after_wrapping() {
    let mut app = App::new();
    app.preview.source = PreviewSource::Tmux;
    app.preview.view = PreviewView::Plain;
    app.preview.pane_id = Some("%1".into());
    app.preview.content = "abcd\nefgh".into();
    app.preview.follow_bottom = false;
    app.preview.scroll = 1;

    let rows = preview_visible_plain_text_rows(&mut app, Rect::new(0, 0, 2, 2));

    assert_eq!(rows, vec!["cd".to_string(), "ef".to_string()]);
    assert!(app.preview.plain_cache.is_some());
}

#[test]
fn preview_info_value_hit_test_returns_full_truncated_value() {
    let area = Rect::new(0, 0, 24, 11);
    let value = "https://opencode.ai/s/very-long-share-id";

    let copied = preview_info_value_text_at(area, 10, 8, 7, value);

    assert_eq!(copied.as_deref(), Some(value));
    assert_eq!(preview_info_value_text_at(area, 2, 8, 7, value), None);
    assert_eq!(preview_info_value_text_at(area, 10, 5, 7, value), None);
}

#[test]
fn preview_provider_value_prefers_session_bound_provider() {
    let mut app = App::new();
    if let Some(agent) = app
        .config
        .agents
        .iter_mut()
        .find(|agent| agent.name == "codex")
    {
        agent.providers = vec![
            ProviderConfig {
                label: "relay-a".into(),
                base_url: "http://127.0.0.1:8317".into(),
                api_key: String::new(),
                env_key: String::new(),
                wire_api: "responses".into(),
                provider_key: String::new(),
                npm_package: String::new(),
                models: Vec::new(),
                test_status: None,
                test_http_status: None,
                test_latency_ms: None,
                test_result: None,
            },
            ProviderConfig {
                label: "relay-b".into(),
                base_url: "http://127.0.0.1:8418".into(),
                api_key: String::new(),
                env_key: String::new(),
                wire_api: "responses".into(),
                provider_key: String::new(),
                npm_package: String::new(),
                models: Vec::new(),
                test_status: None,
                test_http_status: None,
                test_latency_ms: None,
                test_result: None,
            },
        ];
        agent.active_provider = Some(1);
    }

    let thread = SidebarThread {
        key: "codex:sid-1".into(),
        folder_key: "/repo".into(),
        working_dir: "/repo".into(),
        folder_label: "repo".into(),
        agent_type: AgentType::Codex,
        session_id: Some("sid-1".into()),
        transcript_path: None,
        session_provider_name: Some("relay_a".into()),
        title: "title".into(),
        upstream_title: None,
        generated_title: None,
        subtitle: None,
        title_override: None,
        note: None,
        share_url: None,
        cost: None,
        token_summary: None,
        tags: Vec::new(),
        pinned: false,
        updated_at: 0,
        sort_updated_at: 0,
        live_pane_id: None,
        live_location: None,
        pid: None,
        git_info: None,
        state: AgentState::Idle,
        is_active: false,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
        archived: false,
        deleted: false,
    };

    assert_eq!(
        preview_provider_value(&app, &thread),
        "relay-a · http://127.0.0.1:8317/v1"
    );
}

#[test]
fn preview_provider_value_falls_back_to_active_provider_without_session_binding() {
    let mut app = App::new();
    if let Some(agent) = app
        .config
        .agents
        .iter_mut()
        .find(|agent| agent.name == "codex")
    {
        agent.providers = vec![ProviderConfig {
            label: "relay-a".into(),
            base_url: "http://127.0.0.1:8317".into(),
            api_key: String::new(),
            env_key: String::new(),
            wire_api: "responses".into(),
            provider_key: String::new(),
            npm_package: String::new(),
            models: Vec::new(),
            test_status: None,
            test_http_status: None,
            test_latency_ms: None,
            test_result: None,
        }];
        agent.active_provider = Some(0);
    }

    let thread = SidebarThread {
        key: "codex:sid-1".into(),
        folder_key: "/repo".into(),
        working_dir: "/repo".into(),
        folder_label: "repo".into(),
        agent_type: AgentType::Codex,
        session_id: Some("sid-1".into()),
        transcript_path: None,
        session_provider_name: None,
        title: "title".into(),
        upstream_title: None,
        generated_title: None,
        subtitle: None,
        title_override: None,
        note: None,
        share_url: None,
        cost: None,
        token_summary: None,
        tags: Vec::new(),
        pinned: false,
        updated_at: 0,
        sort_updated_at: 0,
        live_pane_id: None,
        live_location: None,
        pid: None,
        git_info: None,
        state: AgentState::Idle,
        is_active: false,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
        archived: false,
        deleted: false,
    };

    assert_eq!(
        preview_provider_value(&app, &thread),
        "relay-a · http://127.0.0.1:8317/v1"
    );
}
