use super::thread_row::format_jump_badge;
use super::thread_subtitle::thread_subtitle;
use crate::model::{AgentState, AgentType};
use crate::sidebar::SidebarThread;

#[test]
fn jump_badge_is_fixed_width_and_limited_to_nine() {
    assert_eq!(format_jump_badge(Some(1), 4), "#1  ");
    assert_eq!(format_jump_badge(Some(9), 4), "#9  ");
    assert_eq!(format_jump_badge(Some(10), 4), "    ");
    assert_eq!(format_jump_badge(None, 4), "    ");
}

#[test]
fn latest_prompt_wins_over_stale_subtitle() {
    let thread = SidebarThread {
        key: "thread:1".into(),
        folder_key: "folder:/tmp".into(),
        working_dir: "/tmp".into(),
        folder_label: "tmp".into(),
        agent_type: AgentType::Codex,
        session_id: Some("session-1".into()),
        transcript_path: None,
        session_provider_name: None,
        title: "Test".into(),
        upstream_title: None,
        generated_title: None,
        subtitle: Some("very old prompt".into()),
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
        last_user_prompt: Some("latest prompt".into()),
        last_assistant_message: Some("latest answer".into()),
        has_unread_stop: false,
        archived: false,
        deleted: false,
    };

    assert_eq!(thread_subtitle(&thread), "latest prompt");
}
