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
        if self.frame_budget_exceeded {
            Duration::from_millis(240)
        } else {
            Duration::from_millis(120)
        }
    }
}
