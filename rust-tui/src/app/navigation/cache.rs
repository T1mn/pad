mod folders {
    use super::super::super::App;
    use crate::app::state::sidebar::ThreadListView;
    use crate::log_debug;
    use crate::sidebar::SidebarThread;
    use std::sync::Arc;

    impl App {
        pub(in crate::app::navigation) fn apply_cached_preview_to_thread(
            &self,
            thread: &mut SidebarThread,
        ) {
            let Some(cache) = self.preview.thread_preview_cache.get(&thread.key) else {
                return;
            };

            if cache.turns.len() > thread.cached_preview_turns.len() {
                thread.cached_preview_turns = cache.turns.clone();
            }
            if thread.session_cache_state.is_none() {
                thread.session_cache_state = cache.session_cache_state;
            }
            if thread.transcript_path.is_none() {
                thread.transcript_path = cache.transcript_path.clone();
            }
            if thread.session_id.is_none() {
                thread.session_id = cache.session_id.clone();
            }
            if let Some(updated_at) = cache.updated_at {
                thread.updated_at = thread.updated_at.max(updated_at);
            }
            if (thread.title.trim().is_empty() || thread.title == "untitled")
                && !cache.turns.is_empty()
            {
                thread.title = cache.turns[0]
                    .question
                    .lines()
                    .next()
                    .unwrap_or("untitled")
                    .trim()
                    .to_string();
            }
        }

        pub(in crate::app::navigation) fn ensure_sidebar_folders_cache(&mut self) {
            if self.sidebar.sidebar_folders_dirty {
                let started_at = std::time::Instant::now();
                let overrides = if self.thread_list_view() != ThreadListView::Normal {
                    Vec::new()
                } else {
                    self.prune_app_thread_activity(crate::app::unix_now_ts());
                    self.sidebar
                        .app_thread_activity
                        .values()
                        .cloned()
                        .collect::<Vec<_>>()
                };
                let mut folders = crate::sidebar::build_sidebar_folders(
                    &self.panels,
                    &overrides,
                    &self.sidebar.thread_sort_activity,
                    self.thread_list_view(),
                    self.thread_list_view() == ThreadListView::Normal
                        && self.showing_live_sessions(),
                );
                for folder in &mut folders {
                    for thread in &mut folder.threads {
                        self.apply_cached_preview_to_thread(Arc::make_mut(thread));
                    }
                    folder.threads.sort_by(crate::sidebar::thread_sort_key);
                    folder.updated_at = folder
                        .threads
                        .first()
                        .map(|thread| thread.sort_timestamp())
                        .unwrap_or_default();
                }
                folders.sort_by(crate::sidebar::folder_sort_key);
                self.sidebar.sidebar_folders_cache = folders;
                self.sidebar.sidebar_folders_dirty = false;
                self.sidebar.visible_sidebar_items_dirty = true;
                self.sidebar.preferred_panel_width_cache = None;
                let elapsed = started_at.elapsed();
                if elapsed >= std::time::Duration::from_millis(8) {
                    log_debug!(
                        "sidebar.cache: rebuild_folders folders={} elapsed_ms={}",
                        self.sidebar.sidebar_folders_cache.len(),
                        elapsed.as_millis()
                    );
                }
            }
        }
    }
}
mod visible {
    use super::super::super::App;
    use crate::log_debug;
    use crate::sidebar::{SidebarFolder, SidebarItem};

    impl App {
        pub(in crate::app::navigation) fn ensure_visible_sidebar_items_cache(&mut self) {
            if self.sidebar.visible_sidebar_items_dirty {
                let started_at = std::time::Instant::now();
                self.ensure_sidebar_folders_cache();
                self.sidebar.visible_sidebar_items_cache =
                    crate::sidebar::build_visible_sidebar_items(
                        &self.sidebar.sidebar_folders_cache,
                        &self.sidebar.expanded_folders,
                        &self.search_query,
                    );
                self.sidebar.visible_sidebar_stats =
                    crate::app::state::VisibleSidebarStats::from_items(
                        &self.sidebar.visible_sidebar_items_cache,
                    );
                self.sidebar.visible_sidebar_items_dirty = false;
                self.sidebar.preferred_panel_width_cache = None;
                let elapsed = started_at.elapsed();
                if elapsed >= std::time::Duration::from_millis(8) {
                    log_debug!(
                        "sidebar.cache: rebuild_visible items={} threads={} rows={} elapsed_ms={}",
                        self.sidebar.visible_sidebar_stats.item_count,
                        self.sidebar.visible_sidebar_stats.thread_count,
                        self.sidebar.visible_sidebar_stats.row_count,
                        elapsed.as_millis()
                    );
                }
            }
        }

        pub fn sidebar_folders_ref(&mut self) -> &[SidebarFolder] {
            self.ensure_sidebar_folders_cache();
            &self.sidebar.sidebar_folders_cache
        }

        pub fn visible_sidebar_items_ref(&mut self) -> &[SidebarItem] {
            self.ensure_visible_sidebar_items_cache();
            &self.sidebar.visible_sidebar_items_cache
        }
    }
}
