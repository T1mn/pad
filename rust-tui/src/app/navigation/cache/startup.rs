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
