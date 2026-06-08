use super::*;
use crate::sidebar::{SidebarFolder, SidebarItem, SidebarThread};
use std::sync::Arc;

#[test]
fn shimmer_preserves_text_content() {
    let text = "rust-tui";
    let rendered: String = animation::shimmer_spans(
        text,
        ratatui::style::Color::White,
        ratatui::style::Color::Cyan,
        ratatui::style::Color::Black,
    )
    .into_iter()
    .map(|span| span.content.to_string())
    .collect();
    assert_eq!(rendered, text);
}

#[test]
fn waiting_threads_do_not_breathe() {
    assert!(animation::thread_badge_breathes(
        &crate::model::AgentState::Busy
    ));
    assert!(!animation::thread_badge_breathes(
        &crate::model::AgentState::Waiting
    ));
    assert!(!animation::thread_badge_breathes(
        &crate::model::AgentState::Idle
    ));
}

#[test]
fn visible_thread_count_ignores_folder_rows() {
    let folder = SidebarFolder {
        key: "folder:/tmp".into(),
        path: "/tmp".into(),
        label: "tmp".into(),
        updated_at: 0,
        threads: Vec::new(),
    };
    let thread = SidebarThread {
        key: "thread:1".into(),
        folder_key: folder.key.clone(),
        working_dir: "/tmp".into(),
        folder_label: "tmp".into(),
        agent_type: crate::model::AgentType::Codex,
        session_id: Some("session-1".into()),
        transcript_path: None,
        session_provider_name: None,
        title: "Test".into(),
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
    let items = vec![
        SidebarItem::Folder(folder.summary()),
        SidebarItem::Thread(Arc::new(thread)),
    ];

    assert_eq!(
        crate::app::state::VisibleSidebarStats::from_items(&items).thread_count,
        1
    );
}

#[test]
fn visible_thread_jump_badges_ignore_folders_and_cap_at_nine() {
    let folder = SidebarFolder {
        key: "folder:/tmp".into(),
        path: "/tmp".into(),
        label: "tmp".into(),
        updated_at: 0,
        threads: Vec::new(),
    };
    let thread = |index: usize| SidebarThread {
        key: format!("thread:{index}"),
        folder_key: folder.key.clone(),
        working_dir: "/tmp".into(),
        folder_label: "tmp".into(),
        agent_type: crate::model::AgentType::Codex,
        session_id: Some(format!("session-{index}")),
        transcript_path: None,
        session_provider_name: None,
        title: format!("Test {index}"),
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
    let mut items = vec![SidebarItem::Folder(folder.summary())];
    for index in 1..=10 {
        items.push(SidebarItem::Thread(Arc::new(thread(index))));
    }

    let badges = viewport::visible_thread_jump_badges(&items);
    assert_eq!(badges[0], None);
    assert_eq!(badges[1], Some(1));
    assert_eq!(badges[9], Some(9));
    assert_eq!(badges[10], None);
}
