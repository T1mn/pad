use super::super::App;
use crate::app::state::sidebar::ThreadListView;
use crate::log_debug;
use crate::sidebar::{SidebarFolder, SidebarItem, SidebarThread};
use std::collections::HashMap;
use std::sync::Arc;

impl App {
    pub(super) fn apply_cached_preview_to_thread(&self, thread: &mut SidebarThread) {
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
        if (thread.title.trim().is_empty() || thread.title == "untitled") && !cache.turns.is_empty()
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

    fn ensure_sidebar_folders_cache(&mut self) {
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
            let empty_startup_thread_sort_activity = HashMap::new();
            let startup_thread_sort_activity = if self.thread_list_view() != ThreadListView::Normal
            {
                &empty_startup_thread_sort_activity
            } else {
                &self.sidebar.startup_thread_sort_activity
            };
            let mut folders = crate::sidebar::build_sidebar_folders(
                &self.panels,
                &overrides,
                &self.sidebar.thread_sort_activity,
                startup_thread_sort_activity,
                self.thread_list_view(),
                self.thread_list_view() == ThreadListView::Normal && self.showing_live_sessions(),
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

    pub(super) fn ensure_visible_sidebar_items_cache(&mut self) {
        if self.sidebar.visible_sidebar_items_dirty {
            let started_at = std::time::Instant::now();
            self.ensure_sidebar_folders_cache();
            self.sidebar.visible_sidebar_items_cache = crate::sidebar::build_visible_sidebar_items(
                &self.sidebar.sidebar_folders_cache,
                &self.sidebar.expanded_folders,
                &self.search_query,
            );
            self.sidebar.visible_sidebar_items_dirty = false;
            let elapsed = started_at.elapsed();
            if elapsed >= std::time::Duration::from_millis(8) {
                log_debug!(
                    "sidebar.cache: rebuild_visible items={} elapsed_ms={}",
                    self.sidebar.visible_sidebar_items_cache.len(),
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

    #[allow(dead_code)]
    pub fn sidebar_folders(&mut self) -> Vec<SidebarFolder> {
        self.sidebar_folders_ref().to_vec()
    }

    #[allow(dead_code)]
    pub fn visible_sidebar_items(&mut self) -> Vec<SidebarItem> {
        self.visible_sidebar_items_ref().to_vec()
    }
}
