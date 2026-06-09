use super::shortened_thread_path;
use crate::model::{AgentState, AgentType};
use crate::sidebar::SidebarThread;

fn thread_with_dir(working_dir: &str) -> SidebarThread {
    SidebarThread {
        key: "codex:sid-1".into(),
        folder_key: working_dir.into(),
        working_dir: working_dir.into(),
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
    }
}

#[test]
fn shortened_thread_path_uses_last_two_segments_without_vec() {
    let thread = thread_with_dir("/very/long/workspace/project/repo");
    assert_eq!(shortened_thread_path(&thread, 24), "~/.../project/repo");

    let trailing = thread_with_dir("/very/long/workspace/project/repo/");
    assert_eq!(shortened_thread_path(&trailing, 24), "~/.../repo/");
}
