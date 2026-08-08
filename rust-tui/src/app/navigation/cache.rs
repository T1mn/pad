mod folders;
mod startup {
    use super::super::super::App;
    use crate::app::state::sidebar::ThreadListView;
    use crate::log_debug;
    use std::collections::HashMap;

    impl App {
        pub fn seed_startup_thread_sort_activity_once(&mut self) -> bool {
            if self.sidebar.startup_thread_sort_seeded {
                return false;
            }

            let folders = crate::sidebar::build_sidebar_folders(
                &self.panels,
                &[],
                &HashMap::new(),
                &HashMap::new(),
                ThreadListView::Normal,
                false,
            );

            let mut seeded_threads = 0usize;
            let mut seeded_keys = 0usize;
            for folder in folders {
                for thread in folder.threads {
                    if thread.archived || thread.updated_at <= 0 {
                        continue;
                    }
                    seeded_threads += 1;
                    for key in thread.sort_activity_keys() {
                        let entry = self
                            .sidebar
                            .startup_thread_sort_activity
                            .entry(key)
                            .or_insert(thread.updated_at);
                        if thread.updated_at > *entry {
                            *entry = thread.updated_at;
                        }
                        seeded_keys += 1;
                    }
                }
            }

            self.sidebar.startup_thread_sort_seeded = true;
            log_debug!(
                "sidebar.startup_sort: seeded threads={} candidate_keys={} unique_keys={} panels={}",
                seeded_threads,
                seeded_keys,
                self.sidebar.startup_thread_sort_activity.len(),
                self.panels.len()
            );
            true
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
