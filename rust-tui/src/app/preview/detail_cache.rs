mod lookup;
mod matchers {
    use crate::app::PreviewDetailCache;
    use crate::model::SharedPreviewTurns;

    pub(super) fn detail_cache_matches_current_turns(
        cache: &PreviewDetailCache,
        turns: &SharedPreviewTurns,
        target_key: &str,
        turn_index: usize,
        width: u16,
        theme_name: &str,
    ) -> bool {
        cache.target_key == target_key
            && cache.turn_index == turn_index
            && cache.width == width
            && cache.theme_name == theme_name
            && cache.turns.shares_allocation_with(turns)
    }

    pub(super) fn detail_cache_matches_request(
        cache: &PreviewDetailCache,
        target_key: &str,
        turn_index: usize,
        width: u16,
        theme_name: &str,
        question: &str,
        answer: &Option<String>,
    ) -> bool {
        cache.target_key == target_key
            && cache.turn_index == turn_index
            && cache.width == width
            && cache.theme_name == theme_name
            && cache.question == question
            && cache.answer == *answer
    }
}
mod request {
    use crate::app::{App, PreviewDetailRenderRequest};

    impl App {
        pub fn current_preview_detail_request(&self) -> Option<PreviewDetailRenderRequest> {
            let selected = self.preview.expanded_turn?;
            let turn = self.preview.turns.get(selected)?;
            Some(PreviewDetailRenderRequest {
                target_key: self.preview.pane_id.clone().unwrap_or_default(),
                turns: self.preview.turns.clone(),
                turn_index: selected,
                width: 0,
                theme_name: self.theme.name.to_string(),
                question: turn.question.clone(),
                answer: turn.answer.clone(),
            })
        }
    }
}
mod store {
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
}
