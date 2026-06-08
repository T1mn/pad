mod matchers;
mod source;

use super::model::{SidebarFolder, SidebarItem};
use matchers::{folder_matches_search, thread_matches_search};

pub(crate) use source::is_subagent_source;

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
                .filter(|thread| thread_matches_search(thread.as_ref(), &query))
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

#[cfg(test)]
mod tests;
