use crate::app::{App, Mode, ThreadActionKind};
use crate::model::AgentType;

#[test]
fn opencode_thread_can_open_archive_confirm() {
    let mut app = App::new();
    let thread = crate::sidebar::SidebarThread {
        key: "opencode:ses_1".into(),
        folder_key: "/repo".into(),
        working_dir: "/repo".into(),
        folder_label: "repo".into(),
        agent_type: AgentType::OpenCode,
        session_id: Some("ses_1".into()),
        transcript_path: None,
        session_provider_name: None,
        title: "OpenCode task".into(),
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
        updated_at: 1,
        sort_updated_at: 1,
        live_pane_id: None,
        live_location: None,
        pid: None,
        git_info: None,
        state: crate::model::AgentState::Idle,
        is_active: false,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
        archived: false,
        deleted: false,
    };
    let thread = std::sync::Arc::new(thread);
    let folder = crate::sidebar::SidebarFolder {
        key: "/repo".into(),
        path: "/repo".into(),
        label: "repo".into(),
        updated_at: 1,
        threads: vec![thread.clone()],
    };
    app.sidebar.sidebar_folders_cache = vec![folder];
    app.sidebar.visible_sidebar_items_cache = vec![crate::sidebar::SidebarItem::Thread(thread)];
    app.sidebar.selected_sidebar_key = Some("opencode:ses_1".into());
    app.sidebar.sidebar_folders_dirty = false;
    app.sidebar.visible_sidebar_items_dirty = false;
    app.table_state.select(Some(0));

    assert!(app.request_archive_selected_thread());
    assert!(matches!(app.mode, Mode::ThreadActionConfirm));
    assert_eq!(
        app.sidebar
            .pending_thread_action
            .as_ref()
            .map(|action| action.kind),
        Some(ThreadActionKind::Archive)
    );
}
