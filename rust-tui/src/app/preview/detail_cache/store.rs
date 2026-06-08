use super::matchers::detail_cache_matches_request;
use crate::app::{App, PreviewDetailCache};

impl App {
    pub fn store_preview_detail_cache(&mut self, cache: PreviewDetailCache) {
        self.preview.detail_lru.retain(|existing| {
            !detail_cache_matches_request(
                existing,
                &cache.target_key,
                cache.turn_index,
                cache.width,
                &cache.theme_name,
                &cache.question,
                &cache.answer,
            )
        });
        self.preview.detail_lru.insert(0, cache.clone());
        self.preview.detail_lru.truncate(6);
        self.preview.detail_cache = Some(cache);
    }
}
