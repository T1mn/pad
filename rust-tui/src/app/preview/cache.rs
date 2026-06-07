use super::super::{App, THREAD_PREVIEW_CACHE_MAX_ENTRIES};
use std::collections::HashSet;
use std::time::{Duration, Instant};

impl App {
    pub fn clear_preview_render_caches(&mut self) {
        self.preview.detail_cache = None;
        self.preview.detail_lru.clear();
        self.preview.detail_render_in_progress = false;
        self.preview.detail_render_rx = None;
        self.preview.detail_pending_request = None;
        self.preview.plain_cache = None;
        self.preview.session_list_cache = None;
    }

    pub fn debounce_preview_after_navigation(&mut self) {
        self.preview.navigation_debounce_until = Some(Instant::now() + Duration::from_millis(300));
    }

    pub fn preview_navigation_debounce_active(&self) -> bool {
        self.preview
            .navigation_debounce_until
            .is_some_and(|until| Instant::now() < until)
    }

    pub fn invalidate_preview(&mut self) {
        self.preview.last_preview_update = Instant::now() - Duration::from_secs(1);
        self.preview.priority_refresh = true;
        self.preview.plain_cache = None;
        self.preview.session_list_cache = None;
    }

    pub(crate) fn prune_thread_preview_cache(&mut self) -> bool {
        if self.preview.thread_preview_cache.len() <= THREAD_PREVIEW_CACHE_MAX_ENTRIES {
            return false;
        }

        let mut keys_by_freshness = self
            .preview
            .thread_preview_cache
            .iter()
            .map(|(key, entry)| {
                (
                    key.clone(),
                    entry.updated_at.unwrap_or(entry.cached_at),
                    entry.cached_at,
                )
            })
            .collect::<Vec<_>>();
        keys_by_freshness.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| right.2.cmp(&left.2))
                .then_with(|| left.0.cmp(&right.0))
        });

        let keep = keys_by_freshness
            .into_iter()
            .take(THREAD_PREVIEW_CACHE_MAX_ENTRIES)
            .map(|item| item.0)
            .collect::<HashSet<_>>();
        let before = self.preview.thread_preview_cache.len();
        self.preview
            .thread_preview_cache
            .retain(|key, _| keep.contains(key));
        self.preview.thread_preview_cache.len() != before
    }
}
