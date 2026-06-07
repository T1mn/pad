use super::super::App;
use super::ScanResult;
use crate::log_debug;
use crate::model::{AgentPanel, AgentState, AgentStateSource};
use crate::scanner::scan_panels;
use std::time::Instant;
use tokio::sync::mpsc;

impl App {
    fn panels_affecting_refresh_changed(&self, next_panels: &[AgentPanel]) -> bool {
        if self.panels.len() != next_panels.len() {
            return true;
        }

        for next in next_panels {
            let Some(current) = self
                .panels
                .iter()
                .find(|panel| panel.pane_id == next.pane_id)
            else {
                return true;
            };
            if current.session != next.session
                || current.window != next.window
                || current.window_index != next.window_index
                || current.pane != next.pane
                || current.working_dir != next.working_dir
                || current.agent_type != next.agent_type
                || current.state != next.state
                || current.state_source != next.state_source
                || current.is_active != next.is_active
                || current.transcript_path != next.transcript_path
                || current.cached_preview_turns != next.cached_preview_turns
                || current.session_cache_state != next.session_cache_state
                || current.git_info != next.git_info
                || current.pid != next.pid
                || current.last_user_prompt != next.last_user_prompt
                || current.last_assistant_message != next.last_assistant_message
                || current.agent_session_id != next.agent_session_id
                || current.has_unread_stop != next.has_unread_stop
            {
                return true;
            }
        }

        false
    }

    pub(super) fn apply_scan_panels(&mut self, mut panels: Vec<AgentPanel>) {
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

    pub fn trigger_async_scan(&mut self) {
        if self.scan_in_progress {
            return;
        }

        self.scan_in_progress = true;
        let (tx, rx) = mpsc::channel::<ScanResult>(1);
        self.scan_rx = Some(rx);

        tokio::task::spawn_blocking(move || {
            if let Err(err) = crate::codex_state::normalize_pad_codex_home_rollout_paths() {
                log_debug!(
                    "async_ops: codex rollout path normalization failed: {}",
                    err
                );
            }
            let result = scan_panels();
            let _ = tx.blocking_send(result);
        });
    }

    pub fn check_scan_result(&mut self) {
        if let Some(ref mut rx) = self.scan_rx {
            match rx.try_recv() {
                Ok(Ok(panels)) => {
                    if self.should_defer_ui_updates() {
                        log_debug!("async_ops: defer scan result while in detail view");
                        self.deferred_scan_result = Some(panels);
                    } else {
                        self.apply_scan_panels(panels);
                    }
                    self.scan_in_progress = false;
                    self.scan_rx = None;
                }
                Ok(Err(e)) => {
                    log_debug!("async_ops: 扫描失败: {}", e);
                    self.scan_in_progress = false;
                    self.scan_rx = None;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    log_debug!("async_ops: 扫描 channel 断开");
                    self.scan_in_progress = false;
                    self.scan_rx = None;
                }
            }
        }
    }

    pub fn schedule_delayed_scan(&mut self, delay_ms: u64) {
        self.delayed_scan_at = Some(Instant::now() + std::time::Duration::from_millis(delay_ms));
    }

    pub fn check_delayed_scan(&mut self) {
        if let Some(at) = self.delayed_scan_at {
            if Instant::now() >= at {
                self.delayed_scan_at = None;
                if !self.scan_in_progress {
                    self.trigger_async_scan();
                }
            }
        }
    }
}

pub(super) fn should_preserve_hook_state(panel: &AgentPanel) -> bool {
    panel.state_source == AgentStateSource::Hook && matches!(panel.state, AgentState::Busy)
}
