mod matchers {
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
            AgentType::Pi => "pi",
            AgentType::Grok => "grok",
            AgentType::Kimi => "kimi",
            AgentType::Gemini => "gemini",
            AgentType::OpenCode => "opencode",
            AgentType::Aider => "aider",
            AgentType::Cursor => "cursor",
            AgentType::Unknown => "unknown",
        }
    }
}
mod source {
    use serde_json::Value;

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
}

use super::model::{SidebarFolder, SidebarItem};
use matchers::{folder_matches_search, thread_matches_search};

pub(crate) use source::is_subagent_source;

pub fn build_visible_sidebar_items(
    folders: &[SidebarFolder],
    expanded_folders: &std::collections::HashSet<String>,
    search_query: &str,
) -> Vec<SidebarItem> {
    let query = search_query.trim();
    let searching = !query.is_empty();
    let mut items =
        Vec::with_capacity(visible_items_capacity(folders, expanded_folders, searching));

    for folder in folders {
        if searching {
            push_search_results(&mut items, folder, query);
        } else {
            push_folder_items(&mut items, folder, expanded_folders.contains(&folder.key));
        }
    }

    items
}

fn visible_items_capacity(
    folders: &[SidebarFolder],
    expanded_folders: &std::collections::HashSet<String>,
    searching: bool,
) -> usize {
    folders
        .iter()
        .map(|folder| {
            1 + if searching || expanded_folders.contains(&folder.key) {
                folder.threads.len()
            } else {
                0
            }
        })
        .sum()
}

fn push_search_results(items: &mut Vec<SidebarItem>, folder: &SidebarFolder, query: &str) {
    let folder_matches = folder_matches_search(folder, query);
    let folder_index = items.len();
    items.push(SidebarItem::folder(folder.summary()));

    let thread_start = items.len();
    items.extend(
        folder
            .threads
            .iter()
            .filter(|thread| thread_matches_search(thread.as_ref(), query))
            .cloned()
            .map(SidebarItem::Thread),
    );

    if !folder_matches && items.len() == thread_start {
        items.remove(folder_index);
    }
}

fn push_folder_items(items: &mut Vec<SidebarItem>, folder: &SidebarFolder, is_expanded: bool) {
    items.push(SidebarItem::folder(folder.summary()));
    if is_expanded {
        items.extend(folder.threads.iter().cloned().map(SidebarItem::Thread));
    }
}

#[cfg(test)]
pub(crate) mod tests;
