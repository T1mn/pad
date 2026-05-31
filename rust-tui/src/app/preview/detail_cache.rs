use super::super::{App, PreviewDetailCache, PreviewDetailRenderRequest};
use crate::model::SharedPreviewTurns;

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

    pub fn ensure_preview_detail_cache_for_current_turns(
        &mut self,
        target_key: &str,
        turn_index: usize,
        width: u16,
        theme_name: &str,
    ) -> bool {
        if self
            .current_preview_detail_cache_for_current_turns(
                target_key, turn_index, width, theme_name,
            )
            .is_some()
        {
            return true;
        }

        if let Some(idx) = self.preview.detail_lru.iter().position(|cache| {
            detail_cache_matches_current_turns(
                cache,
                &self.preview.turns,
                target_key,
                turn_index,
                width,
                theme_name,
            )
        }) {
            let cache = self.preview.detail_lru.remove(idx);
            self.preview.detail_cache = Some(cache);
            return true;
        }

        false
    }

    pub fn current_preview_detail_cache_for_current_turns(
        &self,
        target_key: &str,
        turn_index: usize,
        width: u16,
        theme_name: &str,
    ) -> Option<&PreviewDetailCache> {
        self.preview.detail_cache.as_ref().filter(|cache| {
            detail_cache_matches_current_turns(
                cache,
                &self.preview.turns,
                target_key,
                turn_index,
                width,
                theme_name,
            )
        })
    }

    pub fn cached_preview_detail_for(
        &mut self,
        target_key: &str,
        turn_index: usize,
        width: u16,
        theme_name: &str,
        question: &str,
        answer: &Option<String>,
    ) -> Option<PreviewDetailCache> {
        let current_turns = self.preview.turns.clone();
        if let Some(cache) = self.preview.detail_cache.as_mut().filter(|cache| {
            detail_cache_matches_request(
                cache, target_key, turn_index, width, theme_name, question, answer,
            )
        }) {
            cache.turns = current_turns;
            return Some(cache.clone());
        }

        if let Some(idx) = self.preview.detail_lru.iter().position(|cache| {
            detail_cache_matches_request(
                cache, target_key, turn_index, width, theme_name, question, answer,
            )
        }) {
            let mut cache = self.preview.detail_lru.remove(idx);
            cache.turns = self.preview.turns.clone();
            self.preview.detail_lru.insert(0, cache.clone());
            self.preview.detail_cache = Some(cache.clone());
            return Some(cache);
        }

        None
    }

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
fn detail_cache_matches_current_turns(
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

fn detail_cache_matches_request(
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
