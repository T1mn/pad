use super::*;
use crate::model::{AgentState, AgentType};
use std::sync::Arc;

fn sample_thread(key: &str, title: &str) -> super::super::model::SidebarThread {
    super::super::model::SidebarThread {
        key: key.into(),
        folder_key: "/tmp/demo".into(),
        working_dir: "/tmp/demo".into(),
        folder_label: "demo · tmp".into(),
        agent_type: AgentType::Codex,
        session_id: Some(key.into()),
        transcript_path: None,
        session_provider_name: None,
        title: title.into(),
        upstream_title: Some(title.into()),
        generated_title: None,
        subtitle: Some("prompt".into()),
        title_override: None,
        note: None,
        share_url: None,
        cost: None,
        token_summary: None,
        tags: Vec::new(),
        pinned: false,
        updated_at: 1,
        sort_updated_at: 1,
        live_pane_id: Some("%1".into()),
        live_location: Some("0:1.1".into()),
        pid: None,
        git_info: None,
        state: AgentState::Idle,
        is_active: true,
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
fn search_expands_matching_folder_threads() {
    let folder = SidebarFolder {
        key: "/tmp/demo".into(),
        path: "/tmp/demo".into(),
        label: "demo · tmp".into(),
        updated_at: 1,
        threads: vec![
            Arc::new(sample_thread("a", "hello world")),
            Arc::new(sample_thread("b", "other")),
        ],
    };

    let items = build_visible_sidebar_items(&[folder], &Default::default(), "hello");
    assert_eq!(items.len(), 2);
    assert!(matches!(items[0], SidebarItem::Folder(_)));
    assert!(matches!(items[1], SidebarItem::Thread(_)));
    match &items[0] {
        SidebarItem::Folder(folder) => {
            assert_eq!(folder.thread_count, 2);
            assert!(!folder.has_unread_stop);
        }
        _ => panic!("expected folder item"),
    }
}

#[test]
fn visible_items_reuse_thread_allocation_when_folder_is_expanded() {
    let shared_thread = Arc::new(sample_thread("a", "hello world"));
    let folder = SidebarFolder {
        key: "/tmp/demo".into(),
        path: "/tmp/demo".into(),
        label: "demo · tmp".into(),
        updated_at: 1,
        threads: vec![shared_thread.clone()],
    };
    let expanded = std::collections::HashSet::from([String::from("/tmp/demo")]);

    let items = build_visible_sidebar_items(&[folder], &expanded, "");

    let SidebarItem::Thread(visible_thread) = &items[1] else {
        panic!("expected thread item");
    };
    assert!(Arc::ptr_eq(&shared_thread, visible_thread));
}

#[test]
fn source_json_detects_subagent_thread() {
    let thread_spawn_source = r#"{"subagent":{"thread_spawn":{"parent_thread_id":"019d28e6-0bc0-79c3-b529-a718f803d3c2","depth":1,"agent_path":"/root/audit_event_rs","agent_nickname":"Socrates","agent_role":"explorer"}}}"#;
    let review_source = r#"{"subagent":"review"}"#;
    assert!(is_subagent_source(Some(thread_spawn_source)));
    assert!(is_subagent_source(Some(review_source)));
    assert!(!is_subagent_source(Some("vscode")));
}

#[test]
fn search_matches_ascii_case_insensitively_without_lowercase_copy() {
    let mut thread = sample_thread("a", "Release Notes");
    thread.tags = vec!["HotFix".into()];
    let folder = SidebarFolder {
        key: "/tmp/demo".into(),
        path: "/tmp/demo".into(),
        label: "Demo Folder".into(),
        updated_at: 1,
        threads: vec![Arc::new(thread)],
    };

    let items = build_visible_sidebar_items(&[folder], &Default::default(), "HOTFIX");
    assert_eq!(items.len(), 2);
    assert!(matches!(items[1], SidebarItem::Thread(_)));
}

#[test]
fn search_keeps_unicode_case_fold_behavior() {
    let folder = SidebarFolder {
        key: "/tmp/demo".into(),
        path: "/tmp/demo".into(),
        label: "Demo Folder".into(),
        updated_at: 1,
        threads: vec![Arc::new(sample_thread("a", "Éclair Notes"))],
    };

    let items = build_visible_sidebar_items(&[folder], &Default::default(), "éclair");
    assert_eq!(items.len(), 2);
    assert!(matches!(items[1], SidebarItem::Thread(_)));
}

#[test]
fn search_matches_agent_type_without_string_allocation() {
    let folder = SidebarFolder {
        key: "/tmp/demo".into(),
        path: "/tmp/demo".into(),
        label: "Demo Folder".into(),
        updated_at: 1,
        threads: vec![Arc::new(sample_thread("a", "other"))],
    };

    let items = build_visible_sidebar_items(&[folder], &Default::default(), "CODEX");
    assert_eq!(items.len(), 2);
    assert!(matches!(items[1], SidebarItem::Thread(_)));
}
