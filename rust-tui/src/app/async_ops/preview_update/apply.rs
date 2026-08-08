mod panel {
    use crate::app::App;
    use crate::model::{PreviewSessionOrigin, PreviewSource, SessionCacheState};
    use crate::preview_source::PreviewUpdate;

    pub(super) fn sync_live_panel_from_preview_update(
        app: &mut App,
        update: &PreviewUpdate,
        previous_panel_cache_state: Option<SessionCacheState>,
    ) -> bool {
        let Some(panel) = update
            .live_pane_id
            .as_deref()
            .and_then(|pane_id| app.panels.iter_mut().find(|panel| panel.pane_id == pane_id))
        else {
            return false;
        };

        let should_persist_panel_session = update.session_origin != Some(PreviewSessionOrigin::App);
        if should_persist_panel_session {
            if let Some(transcript_path) = update.transcript_path.clone() {
                panel.transcript_path = Some(transcript_path);
            }
        }
        if app.preview.source == PreviewSource::Session
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
            previous_panel_cache_state != panel.session_cache_state
        } else {
            false
        }
    }
}
mod snapshot {
    use crate::app::App;
    use crate::model::{PreviewSessionOrigin, PreviewSource, PreviewView, SessionCacheState};
    use crate::preview_source::PreviewUpdate;

    pub(super) struct PreviewUpdateSnapshot {
        pub(super) previous_panel_cache_state: Option<SessionCacheState>,
        previous_pane_id: Option<String>,
        previous_source: PreviewSource,
        previous_view: PreviewView,
        previous_session_origin: Option<PreviewSessionOrigin>,
        previous_session_id: Option<String>,
        previous_selected_turn: Option<usize>,
        previous_expanded_turn: Option<usize>,
        previous_list_scroll: u16,
        previous_detail_scroll: u16,
        previous_follow_bottom: bool,
        previous_follow_selection: bool,
    }

    impl PreviewUpdateSnapshot {
        pub(super) fn capture(app: &App, update: &PreviewUpdate) -> Self {
            Self {
                previous_panel_cache_state: app
                    .panels
                    .iter()
                    .find(|panel| update.live_pane_id.as_deref() == Some(panel.pane_id.as_str()))
                    .and_then(|panel| panel.session_cache_state),
                previous_pane_id: app.preview.pane_id.clone(),
                previous_source: app.preview.source,
                previous_view: app.preview.view,
                previous_session_origin: app.preview.session_origin,
                previous_session_id: app.preview.session_id.clone(),
                previous_selected_turn: app.preview.selected_turn,
                previous_expanded_turn: app.preview.expanded_turn,
                previous_list_scroll: app.preview.list_scroll,
                previous_detail_scroll: app.preview.detail_scroll,
                previous_follow_bottom: app.preview.follow_bottom,
                previous_follow_selection: app.preview.follow_selection,
            }
        }

        pub(super) fn preview_state_changed(
            &self,
            app: &App,
            content_changed: bool,
            turns_changed: bool,
            panel_cache_state_changed: bool,
        ) -> bool {
            self.previous_pane_id != app.preview.pane_id
                || self.previous_source != app.preview.source
                || self.previous_view != app.preview.view
                || self.previous_session_origin != app.preview.session_origin
                || self.previous_session_id != app.preview.session_id
                || content_changed
                || turns_changed
                || self.previous_selected_turn != app.preview.selected_turn
                || self.previous_expanded_turn != app.preview.expanded_turn
                || self.previous_list_scroll != app.preview.list_scroll
                || self.previous_detail_scroll != app.preview.detail_scroll
                || self.previous_follow_bottom != app.preview.follow_bottom
                || self.previous_follow_selection != app.preview.follow_selection
                || panel_cache_state_changed
        }
    }
}
mod state {
    use crate::app::App;
    use crate::model::{PreviewSource, PreviewView};
    use crate::preview_source::PreviewUpdate;

    pub(super) fn apply_preview_state(app: &mut App, update: &PreviewUpdate, content: String) {
        let should_follow_bottom = app.preview.follow_bottom
            || app.preview.pane_id.is_none()
            || app.preview.pane_id.as_deref() != Some(update.target_key.as_str());
        let same_context = app.preview.pane_id.as_deref() == Some(update.target_key.as_str())
            && app.preview.source == update.source
            && app.preview.session_origin == update.session_origin
            && app.preview.session_id == update.session_id;

        app.preview.content = content;
        app.preview.pane_id = Some(update.target_key.clone());
        app.preview.source = update.source;
        app.preview.session_origin = update.session_origin;
        app.preview.session_id = update.session_id.clone();

        if app.preview.source == PreviewSource::Session && !update.turns.is_empty() {
            apply_session_preview_state(app, update, same_context);
        } else {
            apply_plain_preview_state(app, should_follow_bottom);
        }
    }

    fn apply_session_preview_state(app: &mut App, update: &PreviewUpdate, same_context: bool) {
        if !same_context {
            app.preview.selected_turn = None;
            app.preview.expanded_turn = None;
            app.preview.view = PreviewView::SessionList;
            app.preview.detail_scroll = 0;
            app.preview.list_scroll = 0;
            app.preview.follow_selection = true;
        } else {
            app.preview.selected_turn = app
                .preview
                .selected_turn
                .filter(|idx| *idx < update.turns.len());
            app.preview.expanded_turn = app
                .preview
                .expanded_turn
                .filter(|idx| *idx < update.turns.len());
            app.preview.view = if app.preview.expanded_turn.is_some() {
                PreviewView::SessionDetail
            } else {
                PreviewView::SessionList
            };
        }
        app.preview.turns = update.turns.clone();
        app.preview.follow_bottom = false;
    }

    fn apply_plain_preview_state(app: &mut App, should_follow_bottom: bool) {
        app.preview.turns = Default::default();
        app.preview.session_origin = None;
        app.preview.session_id = None;
        app.preview.selected_turn = None;
        app.preview.expanded_turn = None;
        app.preview.view = PreviewView::Plain;
        app.preview.list_scroll = 0;
        app.preview.detail_scroll = 0;
        app.preview.follow_bottom = should_follow_bottom;
        app.preview.follow_selection = true;
    }
}
mod thread_cache {
    use crate::app::{App, ThreadPreviewCacheEntry};
    use crate::model::PreviewSource;
    use crate::preview_source::PreviewUpdate;

    pub(super) fn update_thread_preview_cache(app: &mut App, update: &PreviewUpdate) {
        if update.source != PreviewSource::Session || update.turns.is_empty() {
            return;
        }

        let previous_updated_at = app
            .preview
            .thread_preview_cache
            .get(&update.target_key)
            .and_then(|entry| entry.updated_at);
        app.preview.thread_preview_cache.insert(
            update.target_key.clone(),
            ThreadPreviewCacheEntry {
                turns: update.turns.clone(),
                session_cache_state: update.session_cache_state,
                transcript_path: update.transcript_path.clone(),
                session_id: update.session_id.clone(),
                updated_at: update.updated_at,
                cached_at: crate::app::unix_now_ts(),
            },
        );
        let preview_cache_pruned = app.prune_thread_preview_cache();
        if update.updated_at != previous_updated_at || preview_cache_pruned {
            app.invalidate_sidebar_cache();
        }
    }
}

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
