use super::App;
use crate::sidebar::SidebarItem;

mod cache;
mod folders;
mod movement;
mod selection;
mod space_action;

#[cfg(test)]
mod tests;

impl App {
    pub(super) fn nth_visible_thread_sidebar_index(
        items: &[SidebarItem],
        target: usize,
    ) -> Option<usize> {
        let mut visible_threads = 0usize;
        for (index, item) in items.iter().enumerate() {
            if item.as_thread().is_none() {
                continue;
            }
            if visible_threads == target {
                return Some(index);
            }
            visible_threads += 1;
        }
        None
    }

    pub(super) fn sidebar_item_is_navigable(
        items: &[SidebarItem],
        index: usize,
        item: &SidebarItem,
    ) -> bool {
        match item {
            SidebarItem::Thread(_) => true,
            SidebarItem::Folder(folder) => items
                .get(index + 1)
                .and_then(SidebarItem::as_thread)
                .is_none_or(|thread| thread.folder_key != folder.key),
        }
    }

    pub(super) fn next_navigable_sidebar_index(
        items: &[SidebarItem],
        current: Option<usize>,
        forward: bool,
    ) -> Option<usize> {
        let mut first = None;
        let mut last = None;
        let mut next = None;
        let mut previous = None;

        for (index, item) in items.iter().enumerate() {
            if !Self::sidebar_item_is_navigable(items, index, item) {
                continue;
            }

            first.get_or_insert(index);
            last = Some(index);

            if let Some(current) = current {
                if index > current && next.is_none() {
                    next = Some(index);
                }
                if index < current {
                    previous = Some(index);
                }
            }
        }

        match current {
            Some(_) if forward => next.or(first),
            Some(_) => previous.or(last),
            None if forward => first,
            None => last,
        }
    }
}
