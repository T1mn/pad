use super::preferred_panel_width;
use crate::app::state::PreferredPanelWidthCache;
use crate::app::App;
use crate::model::{AgentPanel, AgentState, AgentStateSource, AgentType};

#[test]
fn preferred_panel_width_keeps_short_name_visible() {
    let mut app = App::new();
    app.panels.push(AgentPanel {
        session: "0".into(),
        window: "kanban".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: "%1".into(),
        agent_type: AgentType::Codex,
        working_dir: "/tmp/rust-tui".into(),
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

    assert!(preferred_panel_width(&mut app) >= 13);
}

#[test]
fn preferred_panel_width_cache_clears_on_sidebar_invalidation() {
    let mut app = App::new();
    app.sidebar.visible_sidebar_items_dirty = false;
    app.sidebar.preferred_panel_width_cache = Some(PreferredPanelWidthCache {
        width: 33,
        locale: app.locale,
        thread_list_view: app.thread_list_view(),
        live_only: app.showing_live_sessions(),
        manual_width: app.config.display.agent_panel_width,
    });

    assert_eq!(preferred_panel_width(&mut app), 33);

    app.invalidate_sidebar_visible_cache();

    assert!(app.sidebar.preferred_panel_width_cache.is_none());
}

#[test]
fn thread_width_grows_with_long_titles() {
    let short = super::thread_item_width("短标题");
    let long = super::thread_item_width(
        "这是一个比较长的会话标题，用来确认左侧 pane 会根据标题长度自动变宽",
    );

    assert!(long > short);
    assert!(long > 46);
}

#[test]
fn manual_width_is_used_as_minimum() {
    let mut app = App::new();
    app.config.display.agent_panel_width = Some(70);

    assert!(preferred_panel_width(&mut app) >= 70);
}
