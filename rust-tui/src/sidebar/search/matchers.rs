use super::super::model::{SidebarFolder, SidebarThread};
use crate::model::AgentType;
use crate::text_match::contains_ignore_case;

pub(super) fn folder_matches_search(folder: &SidebarFolder, query: &str) -> bool {
    contains_ignore_case(&folder.label, query) || contains_ignore_case(&folder.path, query)
}

pub(super) fn thread_matches_search(thread: &SidebarThread, query: &str) -> bool {
    contains_ignore_case(&thread.title, query)
        || thread
            .subtitle
            .as_deref()
            .is_some_and(|value| contains_ignore_case(value, query))
        || contains_ignore_case(&thread.working_dir, query)
        || thread
            .session_id
            .as_deref()
            .is_some_and(|value| contains_ignore_case(value, query))
        || thread
            .share_url
            .as_deref()
            .is_some_and(|value| contains_ignore_case(value, query))
        || thread
            .token_summary
            .as_deref()
            .is_some_and(|value| contains_ignore_case(value, query))
        || thread
            .cost
            .as_deref()
            .is_some_and(|value| contains_ignore_case(value, query))
        || contains_ignore_case(agent_type_label(&thread.agent_type), query)
        || thread
            .tags
            .iter()
            .any(|tag| contains_ignore_case(tag, query))
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
