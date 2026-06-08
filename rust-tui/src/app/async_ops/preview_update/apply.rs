mod panel;
mod snapshot;
mod state;
mod thread_cache;

use super::super::super::App;
use crate::preview_source::PreviewUpdate;
use snapshot::PreviewUpdateSnapshot;
use std::time::Instant;

impl App {
    pub(super) fn apply_preview_update_result(&mut self, mut update: PreviewUpdate) {
        let cached_detail_context = self.preview_detail_cache_context();
        let cached_plain_context = self.preview_plain_cache_context();
        let snapshot = PreviewUpdateSnapshot::capture(self, &update);
        let content_changed = self.preview.content != update.content;
        let turns_changed = self.preview.turns != update.turns;
        let content = std::mem::take(&mut update.content);

        if content_changed {
            self.preview.content_revision = self.preview.content_revision.wrapping_add(1);
        }
        state::apply_preview_state(self, &update, content);

        if !self.preview_detail_cache_still_valid(cached_detail_context.as_ref()) {
            self.clear_preview_detail_render_cache();
        }
        if !self.preview_plain_cache_still_valid(cached_plain_context.as_ref()) {
            self.preview.plain_cache = None;
        }

        thread_cache::update_thread_preview_cache(self, &update);
        let panel_cache_state_changed = panel::sync_live_panel_from_preview_update(
            self,
            &update,
            snapshot.previous_panel_cache_state,
        );

        self.preview.last_preview_update = Instant::now();
        if snapshot.preview_state_changed(
            self,
            content_changed,
            turns_changed,
            panel_cache_state_changed,
        ) {
            self.dirty = true;
        }
    }
}
