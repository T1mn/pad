use super::state::ThreadListView;
use super::App;

impl App {
    pub fn invalidate_sidebar_cache(&mut self) {
        self.sidebar.sidebar_folders_dirty = true;
        self.sidebar.visible_sidebar_items_dirty = true;
        self.sidebar.preferred_panel_width_cache = None;
    }

    pub fn invalidate_sidebar_visible_cache(&mut self) {
        self.sidebar.visible_sidebar_items_dirty = true;
        self.sidebar.preferred_panel_width_cache = None;
    }

    pub fn showing_live_sessions(&self) -> bool {
        self.sidebar.display_session_scope == "live"
    }

    pub fn thread_list_view(&self) -> ThreadListView {
        self.sidebar.thread_list_view
    }

    pub fn apply_display_session_scope(&mut self, scope: &str, persist_default: bool) -> bool {
        let normalized = if scope == "all" { "all" } else { "live" };
        let runtime_changed = self.sidebar.display_session_scope != normalized;
        let config_changed = self.config.display.session_scope != normalized;

        if persist_default && config_changed {
            self.config.display.session_scope = normalized.to_string();
            self.config.save();
        }

        if runtime_changed {
            self.sidebar.display_session_scope = normalized.to_string();
            self.sidebar.pending_thread_action = None;
            self.invalidate_sidebar_cache();
            self.sync_sidebar_selection();
            self.invalidate_preview();
            self.focus_panel();
            self.dirty = true;
        } else if persist_default && config_changed {
            self.dirty = true;
        }

        runtime_changed || (persist_default && config_changed)
    }

    pub fn toggle_display_session_scope_view(&mut self) -> bool {
        if self.thread_list_view() != ThreadListView::Normal {
            return false;
        }
        let next_scope = if self.showing_live_sessions() {
            "all"
        } else {
            "live"
        };
        self.apply_display_session_scope(next_scope, false)
    }
}
