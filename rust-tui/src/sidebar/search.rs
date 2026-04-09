use super::model::{SidebarFolder, SidebarItem, SidebarThread};
use serde_json::Value;

pub fn build_visible_sidebar_items(
    folders: &[SidebarFolder],
    expanded_folders: &std::collections::HashSet<String>,
    search_query: &str,
) -> Vec<SidebarItem> {
    let query = search_query.trim().to_lowercase();
    let searching = !query.is_empty();
    let mut items = Vec::new();

    for folder in folders {
        let folder_matches = searching && folder_matches_search(folder, &query);
        let matching_threads = if searching {
            folder
                .threads
                .iter()
                .filter(|thread| thread_matches_search(thread, &query))
                .cloned()
                .collect::<Vec<_>>()
        } else {
            folder.threads.clone()
        };

        if searching && !folder_matches && matching_threads.is_empty() {
            continue;
        }

        items.push(SidebarItem::Folder(folder.summary()));

        let is_expanded = searching || expanded_folders.contains(&folder.key);
        if is_expanded {
            for thread in matching_threads {
                items.push(SidebarItem::Thread(thread));
            }
        }
    }

    items
}

pub(crate) fn is_subagent_source(source: Option<&str>) -> bool {
    let Some(source) = source else {
        return false;
    };
    let source = source.trim();
    if source.is_empty() || !source.starts_with('{') {
        return false;
    }

    let Ok(value) = serde_json::from_str::<Value>(source) else {
        return false;
    };
    value.get("subagent").is_some_and(|value| !value.is_null())
}

fn folder_matches_search(folder: &SidebarFolder, query: &str) -> bool {
    folder.label.to_lowercase().contains(query) || folder.path.to_lowercase().contains(query)
}

fn thread_matches_search(thread: &SidebarThread, query: &str) -> bool {
    thread.title.to_lowercase().contains(query)
        || thread
            .subtitle
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(query)
        || thread.working_dir.to_lowercase().contains(query)
        || thread
            .session_id
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(query)
        || thread.agent_type.to_string().contains(query)
        || thread
            .tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(query))
}

#[cfg(test)]
mod tests {
    use super::super::model::ThreadRuntimeSource;
    use super::*;
    use crate::model::{AgentState, AgentType};

    fn sample_thread(key: &str, title: &str) -> SidebarThread {
        SidebarThread {
            key: key.into(),
            folder_key: "/tmp/demo".into(),
            working_dir: "/tmp/demo".into(),
            folder_label: "demo · tmp".into(),
            agent_type: AgentType::Codex,
            runtime_source: Some(ThreadRuntimeSource::Cli),
            session_id: Some(key.into()),
            transcript_path: None,
            title: title.into(),
            upstream_title: Some(title.into()),
            subtitle: Some("prompt".into()),
            title_override: None,
            note: None,
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
                sample_thread("a", "hello world"),
                sample_thread("b", "other"),
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
    fn source_json_detects_subagent_thread() {
        let thread_spawn_source = r#"{"subagent":{"thread_spawn":{"parent_thread_id":"019d28e6-0bc0-79c3-b529-a718f803d3c2","depth":1,"agent_path":"/root/audit_event_rs","agent_nickname":"Socrates","agent_role":"explorer"}}}"#;
        let review_source = r#"{"subagent":"review"}"#;
        assert!(is_subagent_source(Some(thread_spawn_source)));
        assert!(is_subagent_source(Some(review_source)));
        assert!(!is_subagent_source(Some("vscode")));
    }
}
