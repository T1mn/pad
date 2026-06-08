use super::super::super::App;
use crate::log_debug;
use crate::sidebar::{SidebarFolder, SidebarItem};

impl App {
    pub(in crate::app::navigation) fn ensure_visible_sidebar_items_cache(&mut self) {
        if self.sidebar.visible_sidebar_items_dirty {
            let started_at = std::time::Instant::now();
            self.ensure_sidebar_folders_cache();
            self.sidebar.visible_sidebar_items_cache = crate::sidebar::build_visible_sidebar_items(
                &self.sidebar.sidebar_folders_cache,
                &self.sidebar.expanded_folders,
                &self.search_query,
            );
            self.sidebar.visible_sidebar_stats = crate::app::state::VisibleSidebarStats::from_items(
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

    #[allow(dead_code)]
    pub fn sidebar_folders(&mut self) -> Vec<SidebarFolder> {
        self.sidebar_folders_ref().to_vec()
    }

    #[allow(dead_code)]
    pub fn visible_sidebar_items(&mut self) -> Vec<SidebarItem> {
        self.visible_sidebar_items_ref().to_vec()
    }
}
