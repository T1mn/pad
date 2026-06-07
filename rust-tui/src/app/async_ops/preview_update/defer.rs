use super::super::super::App;
use crate::log_debug;
use crate::preview_source::PreviewUpdate;

impl App {
    pub fn flush_deferred_ui_updates(&mut self) {
        if self.should_defer_ui_updates() {
            return;
        }

        if let Some(panels) = self.deferred_scan_result.take() {
            self.apply_scan_panels(panels);
        }

        if let Some(update) = self.preview.deferred_preview_update.take() {
            if self.preview_navigation_debounce_active() {
                self.preview.deferred_preview_update = Some(update);
            } else if self.preview_update_matches_current_selection(&update) {
                self.apply_preview_update_result(update);
            } else {
                log_debug!(
                    "preview.load: discard_deferred_stale target={}",
                    update.target_key
                );
            }
        }

        if !self.deferred_hook_events.is_empty() {
            let events = std::mem::take(&mut self.deferred_hook_events);
            for event in events {
                self.apply_hook_event(event);
            }
        }
    }

    pub(super) fn preview_update_matches_current_selection(
        &mut self,
        update: &PreviewUpdate,
    ) -> bool {
        if let Some(selected_key) = self.selected_preview_thread().map(|thread| thread.key) {
            return selected_key == update.target_key;
        }

        self.preview.pane_id.as_deref() == Some(update.target_key.as_str())
    }
}
