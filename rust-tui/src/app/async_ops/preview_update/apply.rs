use super::super::super::App;
use crate::model::{PreviewSessionOrigin, PreviewSource, PreviewView};
use crate::preview_source::PreviewUpdate;
use std::time::Instant;

impl App {
    pub(super) fn apply_preview_update_result(&mut self, update: PreviewUpdate) {
        let cached_detail_context = self.preview_detail_cache_context();
        let cached_plain_context = self.preview_plain_cache_context();
        let previous_panel_cache_state = self
            .panels
            .iter()
            .find(|panel| update.live_pane_id.as_deref() == Some(panel.pane_id.as_str()))
            .and_then(|panel| panel.session_cache_state);
        let previous_pane_id = self.preview.pane_id.clone();
        let previous_source = self.preview.source;
        let previous_view = self.preview.view;
        let previous_session_origin = self.preview.session_origin;
        let previous_session_id = self.preview.session_id.clone();
        let previous_selected_turn = self.preview.selected_turn;
        let previous_expanded_turn = self.preview.expanded_turn;
        let previous_list_scroll = self.preview.list_scroll;
        let previous_detail_scroll = self.preview.detail_scroll;
        let previous_follow_bottom = self.preview.follow_bottom;
        let previous_follow_selection = self.preview.follow_selection;
        let content_changed = self.preview.content != update.content;
        let turns_changed = self.preview.turns != update.turns;
        let should_follow_bottom = self.preview.follow_bottom
            || self.preview.pane_id.is_none()
            || self.preview.pane_id.as_deref() != Some(update.target_key.as_str());
        let same_context = self.preview.pane_id.as_deref() == Some(update.target_key.as_str())
            && self.preview.source == update.source
            && self.preview.session_origin == update.session_origin
            && self.preview.session_id == update.session_id;
        if content_changed {
            self.preview.content_revision = self.preview.content_revision.wrapping_add(1);
        }
        self.preview.content = update.content;
        self.preview.pane_id = Some(update.target_key.clone());
        self.preview.source = update.source;
        self.preview.session_origin = update.session_origin;
        self.preview.session_id = update.session_id.clone();
        if self.preview.source == PreviewSource::Session && !update.turns.is_empty() {
            if !same_context {
                self.preview.selected_turn = None;
                self.preview.expanded_turn = None;
                self.preview.view = PreviewView::SessionList;
                self.preview.detail_scroll = 0;
                self.preview.list_scroll = 0;
                self.preview.follow_selection = true;
            } else {
                self.preview.selected_turn = self
                    .preview
                    .selected_turn
                    .filter(|idx| *idx < update.turns.len());
                self.preview.expanded_turn = self
                    .preview
                    .expanded_turn
                    .filter(|idx| *idx < update.turns.len());
                self.preview.view = if self.preview.expanded_turn.is_some() {
                    PreviewView::SessionDetail
                } else {
                    PreviewView::SessionList
                };
            }
            self.preview.turns = update.turns.clone();
            self.preview.follow_bottom = false;
        } else {
            self.preview.turns = Default::default();
            self.preview.session_origin = None;
            self.preview.session_id = None;
            self.preview.selected_turn = None;
            self.preview.expanded_turn = None;
            self.preview.view = PreviewView::Plain;
            self.preview.list_scroll = 0;
            self.preview.detail_scroll = 0;
            self.preview.follow_bottom = should_follow_bottom;
            self.preview.follow_selection = true;
        }

        if !self.preview_detail_cache_still_valid(cached_detail_context.as_ref()) {
            self.clear_preview_detail_render_cache();
        }
        if !self.preview_plain_cache_still_valid(cached_plain_context.as_ref()) {
            self.preview.plain_cache = None;
        }

        let mut panel_cache_state_changed = false;
        if update.source == PreviewSource::Session && !update.turns.is_empty() {
            let previous_updated_at = self
                .preview
                .thread_preview_cache
                .get(&update.target_key)
                .and_then(|entry| entry.updated_at);
            self.preview.thread_preview_cache.insert(
                update.target_key.clone(),
                crate::app::ThreadPreviewCacheEntry {
                    turns: update.turns.clone(),
                    session_cache_state: update.session_cache_state,
                    transcript_path: update.transcript_path.clone(),
                    session_id: update.session_id.clone(),
                    updated_at: update.updated_at,
                    cached_at: crate::app::unix_now_ts(),
                },
            );
            let preview_cache_pruned = self.prune_thread_preview_cache();
            if update.updated_at != previous_updated_at || preview_cache_pruned {
                self.invalidate_sidebar_cache();
            }
        }
        if let Some(panel) = update.live_pane_id.as_deref().and_then(|pane_id| {
            self.panels
                .iter_mut()
                .find(|panel| panel.pane_id == pane_id)
        }) {
            let should_persist_panel_session =
                update.session_origin != Some(PreviewSessionOrigin::App);
            if should_persist_panel_session {
                if let Some(transcript_path) = update.transcript_path.clone() {
                    panel.transcript_path = Some(transcript_path);
                }
            }
            if self.preview.source == PreviewSource::Session
                && !update.turns.is_empty()
                && should_persist_panel_session
            {
                panel.cached_preview_turns = update.turns.clone();
                panel.last_user_prompt = update.turns.first().map(|turn| turn.question.clone());
                panel.last_assistant_message =
                    update.turns.first().and_then(|turn| turn.answer.clone());
                if let Some(state) = update.session_cache_state {
                    panel.session_cache_state = Some(state);
                }
            }
            if should_persist_panel_session {
                if let Some(session_id) = update.session_id.clone() {
                    panel.agent_session_id = Some(session_id);
                }
            }
            if should_persist_panel_session {
                panel_cache_state_changed = previous_panel_cache_state != panel.session_cache_state;
            }
        }

        self.preview.last_preview_update = Instant::now();
        if previous_pane_id != self.preview.pane_id
            || previous_source != self.preview.source
            || previous_view != self.preview.view
            || previous_session_origin != self.preview.session_origin
            || previous_session_id != self.preview.session_id
            || content_changed
            || turns_changed
            || previous_selected_turn != self.preview.selected_turn
            || previous_expanded_turn != self.preview.expanded_turn
            || previous_list_scroll != self.preview.list_scroll
            || previous_detail_scroll != self.preview.detail_scroll
            || previous_follow_bottom != self.preview.follow_bottom
            || previous_follow_selection != self.preview.follow_selection
            || panel_cache_state_changed
        {
            self.dirty = true;
        }
    }
}
