use super::super::{state::FocusTarget, App};
use crate::model::PreviewView;

impl App {
    pub fn select_next_preview_turn(&mut self) -> bool {
        if !self.has_session_preview_turns() {
            return false;
        }
        let max = self.preview.turns.len().saturating_sub(1);
        let next = match self.preview.selected_turn {
            Some(idx) => (idx + 1).min(max),
            None => 0,
        };
        self.preview.selected_turn = Some(next);
        if self.preview.view == PreviewView::SessionDetail {
            self.preview.expanded_turn = Some(next);
            self.preview.detail_scroll = 0;
        }
        self.preview.follow_bottom = false;
        self.preview.follow_selection = true;
        self.clear_preview_render_caches();
        self.dirty = true;
        true
    }

    pub fn select_previous_preview_turn(&mut self) -> bool {
        if !self.has_session_preview_turns() {
            return false;
        }
        let prev = match self.preview.selected_turn {
            Some(idx) => idx.saturating_sub(1),
            None => 0,
        };
        self.preview.selected_turn = Some(prev);
        if self.preview.view == PreviewView::SessionDetail {
            self.preview.expanded_turn = Some(prev);
            self.preview.detail_scroll = 0;
        }
        self.preview.follow_bottom = false;
        self.preview.follow_selection = true;
        self.clear_preview_render_caches();
        self.dirty = true;
        true
    }

    pub fn select_preview_turn(&mut self, index: usize) -> bool {
        if !self.has_session_preview_turns() || index >= self.preview.turns.len() {
            return false;
        }
        self.preview.focus = FocusTarget::Preview;
        self.preview.selected_turn = Some(index);
        self.preview.expanded_turn = None;
        self.preview.view = PreviewView::SessionList;
        self.preview.detail_scroll = 0;
        self.preview.follow_bottom = false;
        self.preview.follow_selection = true;
        self.clear_preview_render_caches();
        self.dirty = true;
        true
    }

    pub fn step_back_preview_focus(&mut self) -> bool {
        if self.preview.view == PreviewView::SessionDetail {
            self.preview.view = PreviewView::SessionList;
            self.preview.expanded_turn = None;
            self.preview.detail_scroll = 0;
            self.preview.follow_selection = true;
            self.flush_deferred_updates_on_preview_exit();
            self.clear_preview_render_caches();
            self.dirty = true;
            return true;
        }
        if self.preview.selected_turn.is_some() {
            self.preview.selected_turn = None;
            self.preview.view = if self.has_session_preview_turns() {
                PreviewView::SessionList
            } else {
                PreviewView::Plain
            };
            self.preview.follow_selection = false;
            self.clear_preview_render_caches();
            self.dirty = true;
            return true;
        }
        if self.preview.is_focused() {
            self.preview.focus = FocusTarget::Panel;
            self.clear_unread_stop_for_selected_panel();
            self.dirty = true;
            return true;
        }
        false
    }

    pub fn toggle_preview_turn_expanded(&mut self) -> bool {
        if !self.has_session_preview_turns() {
            return false;
        }
        let Some(selected) = self.preview.selected_turn else {
            return false;
        };
        if self.preview.view == PreviewView::SessionDetail
            && self.preview.expanded_turn == Some(selected)
        {
            self.preview.view = PreviewView::SessionList;
            self.preview.expanded_turn = None;
            self.flush_deferred_updates_on_preview_exit();
        } else {
            self.preview.view = PreviewView::SessionDetail;
            self.preview.expanded_turn = Some(selected);
        }
        self.preview.detail_scroll = 0;
        self.preview.follow_bottom = false;
        self.preview.follow_selection = true;
        self.clear_preview_render_caches();
        self.dirty = true;
        true
    }
}
