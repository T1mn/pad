use super::helpers::delete_failed_title;
use super::*;

impl App {
    pub fn delete_panel(&mut self, panel: &crate::model::AgentPanel) {
        self.sidebar.pending_sidebar_selection_index = self.table_state.selected();
        log_debug!(
            "delete_panel: pane_id={} runtime=native agent_type={:?}",
            panel.pane_id,
            panel.agent_type,
        );

        if App::is_native_agent_terminal_id(&panel.pane_id) {
            match self.close_native_agent_terminal(&panel.pane_id) {
                Ok(true) => {
                    self.invalidate_preview();
                    self.focus_panel();
                }
                Ok(false) => self.show_action_toast(
                    delete_failed_title(self.locale),
                    "native terminal pane no longer exists",
                ),
                Err(error) => {
                    self.show_action_toast(delete_failed_title(self.locale), &error.to_string())
                }
            }
            return;
        }

        self.apply_deleted_panel_locally(&panel.pane_id);
        self.show_action_toast(
            delete_failed_title(self.locale),
            "removed stale non-native panel entry",
        );
    }

    pub(crate) fn apply_deleted_panel_locally(&mut self, pane_id: &str) {
        let original_len = self.panels.len();
        self.panels.retain(|panel| panel.pane_id != pane_id);
        if self.panels.len() == original_len {
            return;
        }

        self.invalidate_sidebar_cache();
        self.sync_sidebar_selection();
        if self.selected_panel().is_none() {
            self.focus_panel();
        }
        self.invalidate_preview();
        self.dirty = true;
    }

    pub fn refresh_panels(&mut self) {
        self.last_refresh = std::time::Instant::now();
        self.invalidate_sidebar_cache();
        self.invalidate_preview();
        self.dirty = true;
    }
}
