use super::super::{state::FocusTarget, App};
use std::time::{Duration, Instant};

impl App {
    pub fn preview_is_focused(&self) -> bool {
        self.preview.focus == FocusTarget::Preview && !self.sidebar.show_tree
    }

    pub fn toggle_preview_focus(&mut self) -> bool {
        if self.sidebar.show_tree || self.selected_preview_thread().is_none() {
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
        if self.sidebar.show_tree || self.selected_preview_thread().is_none() {
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
