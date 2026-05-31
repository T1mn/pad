mod detail_cache;

use super::{App, PreviewMouseSelection, THREAD_PREVIEW_CACHE_MAX_ENTRIES};
use crate::model::{AgentState, PreviewSource, PreviewView, SharedPreviewTurns};
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

    pub fn preview_is_focused(&self) -> bool {
        self.preview.focus == super::state::FocusTarget::Preview && !self.sidebar.show_tree
    }

    pub fn toggle_preview_focus(&mut self) -> bool {
        if self.sidebar.show_tree || self.selected_preview_thread().is_none() {
            return false;
        }
        self.preview.focus = match self.preview.focus {
            super::state::FocusTarget::Panel => super::state::FocusTarget::Preview,
            super::state::FocusTarget::Preview => super::state::FocusTarget::Panel,
        };
        self.clear_unread_stop_for_selected_panel();
        self.dirty = true;
        true
    }

    pub fn focus_panel(&mut self) {
        if self.preview.focus != super::state::FocusTarget::Panel {
            self.preview.focus = super::state::FocusTarget::Panel;
        }
        self.clear_unread_stop_for_selected_panel();
        self.dirty = true;
    }

    pub fn focus_preview(&mut self) -> bool {
        if self.sidebar.show_tree || self.selected_preview_thread().is_none() {
            return false;
        }
        if self.preview.focus != super::state::FocusTarget::Preview {
            self.preview.focus = super::state::FocusTarget::Preview;
        }
        self.dirty = true;
        true
    }

    pub fn has_session_preview_turns(&self) -> bool {
        self.preview.source == PreviewSource::Session && !self.preview.turns.is_empty()
    }

    pub fn should_defer_ui_updates(&self) -> bool {
        false
    }

    pub fn note_panel_tab(&mut self) {
        self.preview.last_panel_tab_at = Some(Instant::now());
    }

    pub fn recent_panel_tab_within(&self, window: Duration) -> bool {
        self.preview
            .last_panel_tab_at
            .map(|instant| instant.elapsed() <= window)
            .unwrap_or(false)
    }

    pub fn clear_panel_tab(&mut self) {
        self.preview.last_panel_tab_at = None;
    }

    pub fn note_detail_exit_tab(&mut self) {
        self.preview.last_detail_exit_tab_at = Some(Instant::now());
    }

    pub fn recent_detail_exit_tab_within(&self, window: Duration) -> bool {
        self.preview
            .last_detail_exit_tab_at
            .map(|instant| instant.elapsed() <= window)
            .unwrap_or(false)
    }

    pub fn clear_detail_exit_tab(&mut self) {
        self.preview.last_detail_exit_tab_at = None;
    }

    #[allow(dead_code)]
    pub(crate) fn preview_uses_list_scroll(&self) -> bool {
        self.has_session_preview_turns() && self.preview.view == PreviewView::SessionList
    }

    pub(crate) fn preview_uses_detail_scroll(&self) -> bool {
        self.has_session_preview_turns() && self.preview.view == PreviewView::SessionDetail
    }

    fn flush_deferred_updates_on_preview_exit(&mut self) {
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

        self.preview.focus = super::state::FocusTarget::Preview;
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

    pub fn should_pause_preview_refresh(&self) -> bool {
        false
    }

    pub fn should_pause_busy_animations(&self) -> bool {
        false
    }

    pub fn should_tick_busy_animation(&self) -> bool {
        !self.should_pause_busy_animations()
            && self.has_visible_busy_threads()
            && self.last_busy_animation_tick.elapsed() >= self.busy_animation_interval()
    }

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
        self.preview.focus = super::state::FocusTarget::Preview;
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
            self.preview.focus = super::state::FocusTarget::Panel;
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

    pub fn scroll_preview_by(&mut self, delta: i32) {
        if self.preview.uses_list_scroll() {
            self.preview.follow_selection = false;
            if delta >= 0 {
                self.preview.list_scroll = self.preview.list_scroll.saturating_add(delta as u16);
            } else {
                self.preview.list_scroll = self.preview.list_scroll.saturating_sub((-delta) as u16);
            }
        } else if self.preview.uses_detail_scroll() {
            if delta >= 0 {
                self.preview.detail_scroll =
                    self.preview.detail_scroll.saturating_add(delta as u16);
            } else {
                self.preview.detail_scroll =
                    self.preview.detail_scroll.saturating_sub((-delta) as u16);
            }
        } else {
            self.preview.follow_bottom = false;
            if delta >= 0 {
                self.preview.scroll = self.preview.scroll.saturating_add(delta as u16);
            } else {
                self.preview.scroll = self.preview.scroll.saturating_sub((-delta) as u16);
            }
        }
        self.dirty = true;
    }

    pub fn scroll_preview_to_top(&mut self) {
        if self.preview.uses_list_scroll() {
            self.preview.list_scroll = 0;
            self.preview.follow_selection = false;
        } else if self.preview.uses_detail_scroll() {
            self.preview.detail_scroll = 0;
        } else {
            self.preview.scroll = 0;
            self.preview.follow_bottom = false;
        }
        self.dirty = true;
    }

    pub fn scroll_preview_to_bottom(&mut self) {
        if self.preview.uses_list_scroll() {
            self.preview.list_scroll = u16::MAX;
            self.preview.follow_selection = false;
        } else if self.preview.uses_detail_scroll() {
            self.preview.detail_scroll = u16::MAX;
        } else {
            self.preview.follow_bottom = true;
        }
        self.dirty = true;
    }

    pub fn desired_tick_rate(&self) -> Duration {
        if self.has_visible_busy_threads() {
            Duration::from_millis(60)
        } else if self.preview.view == PreviewView::SessionDetail {
            Duration::from_millis(90)
        } else {
            Duration::from_millis(120)
        }
    }

    fn has_visible_busy_threads(&self) -> bool {
        if self.sidebar.show_tree {
            return false;
        }
        if !self.sidebar.visible_sidebar_items_dirty {
            return self
                .sidebar
                .visible_sidebar_items_cache
                .iter()
                .filter_map(|item| item.as_thread())
                .any(|thread| matches!(thread.state, AgentState::Busy));
        }

        self.panels
            .iter()
            .any(|panel| matches!(panel.state, AgentState::Busy))
            || self
                .sidebar
                .app_thread_activity
                .values()
                .any(|thread| matches!(thread.state, AgentState::Busy))
    }

    pub fn busy_animation_interval(&self) -> Duration {
        Duration::from_millis(120)
    }

    pub fn begin_preview_mouse_selection(&mut self, column: u16, row: u16) {
        self.preview.mouse_selection = Some(PreviewMouseSelection {
            anchor_column: column,
            anchor_row: row,
            current_column: column,
            current_row: row,
        });
        self.dirty = true;
    }

    pub fn update_preview_mouse_selection(&mut self, column: u16, row: u16) -> bool {
        let Some(selection) = self.preview.mouse_selection.as_mut() else {
            return false;
        };

        if selection.current_column == column && selection.current_row == row {
            return false;
        }

        selection.current_column = column;
        selection.current_row = row;
        self.dirty = true;
        true
    }

    #[allow(dead_code)]
    pub fn preview_mouse_selection(&self) -> Option<&PreviewMouseSelection> {
        self.preview.mouse_selection.as_ref()
    }

    pub fn clear_preview_mouse_selection(&mut self) -> bool {
        if self.preview.mouse_selection.take().is_some() {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn finish_preview_mouse_selection(&mut self) -> Option<PreviewMouseSelection> {
        let selection = self.preview.mouse_selection.take();
        if selection.is_some() {
            self.dirty = true;
        }
        selection
    }
}

#[cfg(test)]
mod preview_tests;
