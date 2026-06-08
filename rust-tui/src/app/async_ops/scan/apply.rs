use super::super::super::App;
use crate::log_debug;
use crate::model::{AgentPanel, AgentState, AgentStateSource};
use std::time::Instant;

impl App {
    pub(in crate::app::async_ops) fn apply_scan_panels(&mut self, mut panels: Vec<AgentPanel>) {
        log_debug!("async_ops: 扫描完成，检测到 {} 个面板", panels.len());

        for panel in &mut panels {
            if let Some(existing) = self.panels.iter().find(|p| p.pane_id == panel.pane_id) {
                if existing.agent_session_id.is_some() {
                    panel.agent_session_id = existing.agent_session_id.clone();
                }
                if existing.last_user_prompt.is_some() {
                    panel.last_user_prompt = existing.last_user_prompt.clone();
                }
                if existing.last_assistant_message.is_some() {
                    panel.last_assistant_message = existing.last_assistant_message.clone();
                }
                if existing.transcript_path.is_some() {
                    panel.transcript_path = existing.transcript_path.clone();
                }
                if !existing.cached_preview_turns.is_empty() {
                    panel.cached_preview_turns = existing.cached_preview_turns.clone();
                }
                if existing.session_cache_state.is_some() {
                    panel.session_cache_state = existing.session_cache_state;
                }
                panel.has_unread_stop = existing.has_unread_stop;
                if should_preserve_hook_state(existing) {
                    panel.state = existing.state.clone();
                    panel.state_source = existing.state_source.clone();
                    panel.is_active = existing.is_active;
                }
            }
        }

        if let Err(err) = crate::session_cache::preload_panels(&mut panels) {
            log_debug!("session_cache: preload after scan failed: {}", err);
        }

        let refresh_changed = self.panels_affecting_refresh_changed(&panels);
        let sidebar_cache_empty =
            !panels.is_empty() && self.sidebar.visible_sidebar_items_cache.is_empty();
        self.panels = panels;
        let startup_sort_seeded = self.seed_startup_thread_sort_activity_once();
        if startup_sort_seeded || refresh_changed || sidebar_cache_empty {
            self.invalidate_sidebar_cache();
            self.sync_sidebar_selection();
        }
        if self.selected_panel().is_none() {
            self.focus_panel();
        }
        self.last_refresh = Instant::now();
        if refresh_changed {
            self.invalidate_preview();
        }
        if startup_sort_seeded || refresh_changed || sidebar_cache_empty {
            self.dirty = true;
        }
    }
}

pub(in crate::app::async_ops) fn should_preserve_hook_state(panel: &AgentPanel) -> bool {
    panel.state_source == AgentStateSource::Hook && matches!(panel.state, AgentState::Busy)
}
