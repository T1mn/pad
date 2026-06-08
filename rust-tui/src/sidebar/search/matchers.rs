use super::super::model::{SidebarFolder, SidebarThread};

pub(super) fn folder_matches_search(folder: &SidebarFolder, query: &str) -> bool {
    folder.label.to_lowercase().contains(query) || folder.path.to_lowercase().contains(query)
}

pub(super) fn thread_matches_search(thread: &SidebarThread, query: &str) -> bool {
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
        || thread
            .share_url
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(query)
        || thread
            .token_summary
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(query)
        || thread
            .cost
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
