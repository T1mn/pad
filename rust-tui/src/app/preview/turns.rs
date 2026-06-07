use super::super::{state::FocusTarget, App};
use crate::model::{PreviewSource, PreviewView, SharedPreviewTurns};
use std::time::{Duration, Instant};

impl App {
    pub fn has_session_preview_turns(&self) -> bool {
        self.preview.source == PreviewSource::Session && !self.preview.turns.is_empty()
    }

    pub fn should_defer_ui_updates(&self) -> bool {
        false
    }

    #[allow(dead_code)]
    pub(crate) fn preview_uses_list_scroll(&self) -> bool {
        self.has_session_preview_turns() && self.preview.view == PreviewView::SessionList
    }

    pub(crate) fn preview_uses_detail_scroll(&self) -> bool {
        self.has_session_preview_turns() && self.preview.view == PreviewView::SessionDetail
    }

    pub(super) fn flush_deferred_updates_on_preview_exit(&mut self) {
        if self.should_defer_ui_updates() {
            return;
        }
        self.preview.last_preview_update = Instant::now() - Duration::from_secs(1);
    }

    pub fn open_latest_preview_turn(&mut self) -> bool {
        if self.sidebar.show_tree {
            return false;
        }

        let Some(thread) = self.selected_preview_thread() else {
            return false;
        };

        let same_session_preview = self.preview.source == PreviewSource::Session
            && self.preview.pane_id.as_deref() == Some(thread.key.as_str())
            && !self.preview.turns.is_empty();
        let resolved_turns = if same_session_preview && !thread.is_live() {
            self.preview.turns.clone()
        } else if !thread.cached_preview_turns.is_empty() {
            thread.cached_preview_turns.clone()
        } else if same_session_preview {
            self.preview.turns.clone()
        } else {
            SharedPreviewTurns::default()
        };

        if !resolved_turns.is_empty()
            && (!same_session_preview || self.preview.turns != resolved_turns)
        {
            self.preview.turns = resolved_turns;
            self.preview.pane_id = Some(thread.key.clone());
            self.preview.session_origin = thread.preview_origin();
            self.preview.session_id = thread.session_id.clone();
            self.preview.source = PreviewSource::Session;
        }

        if !self.has_session_preview_turns() {
            return false;
        }

        self.preview.focus = FocusTarget::Preview;
        self.preview.selected_turn = Some(0);
        self.preview.expanded_turn = Some(0);
        self.preview.view = PreviewView::SessionDetail;
        self.preview.detail_scroll = 0;
        self.preview.list_scroll = 0;
        self.preview.follow_bottom = false;
        self.preview.follow_selection = true;
        self.clear_detail_exit_tab();
        self.clear_preview_render_caches();
        self.dirty = true;
        true
    }

    pub fn selected_thread_matches_preview_target(&mut self) -> bool {
        self.selected_preview_thread()
            .map(|thread| Some(thread.key) == self.preview.pane_id)
            .unwrap_or(false)
    }

    pub fn restore_preview_turns_list(&mut self) -> bool {
        if !self.has_session_preview_turns() || self.preview.view != PreviewView::SessionDetail {
            return false;
        }

        self.preview.view = PreviewView::SessionList;
        self.preview.expanded_turn = None;
        self.preview.detail_scroll = 0;
        self.preview.follow_selection = true;
        self.flush_deferred_updates_on_preview_exit();
        self.clear_preview_render_caches();
        self.dirty = true;
        true
    }
}
