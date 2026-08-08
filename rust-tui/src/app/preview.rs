mod cache {
    use super::super::{App, THREAD_PREVIEW_CACHE_MAX_ENTRIES};
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
            self.preview.navigation_debounce_until =
                Some(Instant::now() + Duration::from_millis(300));
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

            let before = self.preview.thread_preview_cache.len();
            for key in keys_by_freshness
                .iter()
                .skip(THREAD_PREVIEW_CACHE_MAX_ENTRIES)
                .map(|item| &item.0)
            {
                self.preview.thread_preview_cache.remove(key);
            }
            self.preview.thread_preview_cache.len() != before
        }
    }
}
mod detail_cache;
mod focus {
    use super::super::{state::FocusTarget, App};
    use std::time::{Duration, Instant};

    impl App {
        pub fn preview_is_focused(&self) -> bool {
            self.preview.focus == FocusTarget::Preview && !self.sidebar.show_tree
        }

        pub fn toggle_preview_focus(&mut self) -> bool {
            if self.sidebar.show_tree
                || (self.selected_preview_thread().is_none() && !self.terminal_is_active())
            {
                return false;
            }
            self.preview.focus = match self.preview.focus {
                FocusTarget::Panel => FocusTarget::Preview,
                FocusTarget::Preview => FocusTarget::Panel,
            };
            self.clear_unread_stop_for_selected_panel();
            self.dirty = true;
            true
        }

        pub fn focus_panel(&mut self) {
            if self.preview.focus != FocusTarget::Panel {
                self.preview.focus = FocusTarget::Panel;
            }
            self.clear_unread_stop_for_selected_panel();
            self.dirty = true;
        }

        pub fn focus_preview(&mut self) -> bool {
            if self.sidebar.show_tree
                || (self.selected_preview_thread().is_none() && !self.terminal_is_active())
            {
                return false;
            }
            if self.preview.focus != FocusTarget::Preview {
                self.preview.focus = FocusTarget::Preview;
            }
            self.dirty = true;
            true
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
    }
}
mod mouse {
    use super::super::{App, PreviewMouseSelection};

    impl App {
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
}
mod scroll {
    use super::super::App;

    impl App {
        pub fn scroll_preview_by(&mut self, delta: i32) {
            if self.preview.uses_list_scroll() {
                self.preview.follow_selection = false;
                if delta >= 0 {
                    self.preview.list_scroll =
                        self.preview.list_scroll.saturating_add(delta as u16);
                } else {
                    self.preview.list_scroll =
                        self.preview.list_scroll.saturating_sub((-delta) as u16);
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
    }
}
mod tick {
    use super::super::App;
    use crate::model::{AgentState, PreviewView};
    use std::time::Duration;

    impl App {
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

        pub fn desired_tick_rate(&self) -> Duration {
            if self.terminal_is_focused() {
                Duration::from_millis(16)
            } else if self.terminal_is_active() {
                Duration::from_millis(33)
            } else if self.has_visible_busy_threads() {
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
            if self.frame_budget_exceeded {
                Duration::from_millis(240)
            } else {
                Duration::from_millis(120)
            }
        }
    }
}
mod turn_selection;
mod turns;

#[cfg(test)]
mod preview_tests;
