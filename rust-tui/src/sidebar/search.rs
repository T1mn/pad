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
mod tests;
