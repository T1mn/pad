use super::super::model::{SidebarFolder, SidebarThread};
use crate::model::AgentType;

pub(super) fn folder_matches_search(folder: &SidebarFolder, query: &str) -> bool {
    contains_query_ignore_ascii_case(&folder.label, query)
        || contains_query_ignore_ascii_case(&folder.path, query)
}

pub(super) fn thread_matches_search(thread: &SidebarThread, query: &str) -> bool {
    contains_query_ignore_ascii_case(&thread.title, query)
        || thread
            .subtitle
            .as_deref()
            .is_some_and(|value| contains_query_ignore_ascii_case(value, query))
        || contains_query_ignore_ascii_case(&thread.working_dir, query)
        || thread
            .session_id
            .as_deref()
            .is_some_and(|value| contains_query_ignore_ascii_case(value, query))
        || thread
            .share_url
            .as_deref()
            .is_some_and(|value| contains_query_ignore_ascii_case(value, query))
        || thread
            .token_summary
            .as_deref()
            .is_some_and(|value| contains_query_ignore_ascii_case(value, query))
        || thread
            .cost
            .as_deref()
            .is_some_and(|value| contains_query_ignore_ascii_case(value, query))
        || agent_type_label(&thread.agent_type).contains(query)
        || thread
            .tags
            .iter()
            .any(|tag| contains_query_ignore_ascii_case(tag, query))
}

fn contains_query_ignore_ascii_case(value: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    if value.is_ascii() && query.is_ascii() {
        return value
            .as_bytes()
            .windows(query.len())
            .any(|window| window.eq_ignore_ascii_case(query.as_bytes()));
    }

    value.to_lowercase().contains(query)
}

fn agent_type_label(agent_type: &AgentType) -> &'static str {
    match agent_type {
        AgentType::Claude => "claude",
        AgentType::Codex => "codex",
        AgentType::Kimi => "kimi",
        AgentType::Gemini => "gemini",
        AgentType::OpenCode => "opencode",
        AgentType::Aider => "aider",
        AgentType::Cursor => "cursor",
        AgentType::Unknown => "unknown",
    }
}
