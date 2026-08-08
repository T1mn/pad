mod apply;
mod cache {
    use super::super::super::App;
    use crate::model::{PreviewSource, PreviewTurn};

    pub(super) struct CachedDetailContext {
        target_key: String,
        turn_index: usize,
        turn: PreviewTurn,
        width: u16,
        theme_name: String,
    }

    pub(super) struct CachedPlainContext {
        target_key: String,
        theme_name: String,
        content_revision: u64,
    }

    impl App {
        pub(super) fn preview_detail_cache_context(&self) -> Option<CachedDetailContext> {
            let cache = self.preview.detail_cache.as_ref()?;
            let turn = self.preview.turns.get(cache.turn_index)?.clone();
            Some(CachedDetailContext {
                target_key: cache.target_key.clone(),
                turn_index: cache.turn_index,
                turn,
                width: cache.width,
                theme_name: cache.theme_name.clone(),
            })
        }

        pub(super) fn preview_plain_cache_context(&self) -> Option<CachedPlainContext> {
            self.preview
                .plain_cache
                .as_ref()
                .map(|cache| CachedPlainContext {
                    target_key: cache.target_key.clone(),
                    theme_name: cache.theme_name.clone(),
                    content_revision: cache.content_revision,
                })
        }

        pub(super) fn preview_detail_cache_still_valid(
            &mut self,
            context: Option<&CachedDetailContext>,
        ) -> bool {
            context.is_some_and(|context| {
                self.cached_preview_detail_for(
                    &context.target_key,
                    context.turn_index,
                    context.width,
                    &context.theme_name,
                    &context.turn.question,
                    &context.turn.answer,
                )
                .is_some()
            })
        }

        pub(super) fn preview_plain_cache_still_valid(
            &self,
            context: Option<&CachedPlainContext>,
        ) -> bool {
            context.is_some_and(|context| {
                self.preview.pane_id.as_deref() == Some(context.target_key.as_str())
                    && self.preview.source == PreviewSource::Plain
                    && self.preview.view == crate::model::PreviewView::Plain
                    && self.theme.name == context.theme_name
                    && self.preview.content_revision == context.content_revision
            })
        }

        pub(super) fn clear_preview_detail_render_cache(&mut self) {
            self.preview.detail_cache = None;
            self.preview.detail_lru.clear();
            self.preview.detail_render_in_progress = false;
            self.preview.detail_render_rx = None;
            self.preview.detail_pending_request = None;
        }
    }
}
mod defer {
    use super::super::super::App;
    use crate::log_debug;
    use crate::preview_source::PreviewUpdate;

    impl App {
        pub fn flush_deferred_ui_updates(&mut self) {
            if self.should_defer_ui_updates() {
                return;
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
}
mod load;
