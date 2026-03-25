use super::App;
use crate::log_debug;
use crate::model::{AgentPanel, AgentState, AgentStateSource};
use crate::preview_source::{self, PreviewUpdate};
use crate::scanner::scan_panels;
use std::error::Error;
use std::time::Instant;
use tokio::sync::mpsc;

/// Async scan result channel type
pub type ScanResult = Result<Vec<AgentPanel>, Box<dyn Error + Send + Sync>>;

impl App {
    pub fn trigger_async_scan(&mut self) {
        if self.scan_in_progress {
            return;
        }

        self.scan_in_progress = true;
        let (tx, rx) = mpsc::channel::<ScanResult>(1);
        self.scan_rx = Some(rx);

        tokio::task::spawn_blocking(move || {
            let result = scan_panels();
            let _ = tx.blocking_send(result);
        });
    }

    pub fn check_scan_result(&mut self) {
        if let Some(ref mut rx) = self.scan_rx {
            match rx.try_recv() {
                Ok(Ok(mut panels)) => {
                    log_debug!("async_ops: 扫描完成，检测到 {} 个面板", panels.len());

                    // Preserve hook-driven state/session info by pane_id
                    for panel in &mut panels {
                        if let Some(existing) =
                            self.panels.iter().find(|p| p.pane_id == panel.pane_id)
                        {
                            if existing.agent_session_id.is_some() {
                                panel.agent_session_id = existing.agent_session_id.clone();
                            }
                            if existing.last_user_prompt.is_some() {
                                panel.last_user_prompt = existing.last_user_prompt.clone();
                            }
                            if existing.last_assistant_message.is_some() {
                                panel.last_assistant_message =
                                    existing.last_assistant_message.clone();
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
                            if existing.state_source == AgentStateSource::Hook
                                && matches!(existing.state, AgentState::Busy | AgentState::Waiting)
                            {
                                panel.state = existing.state.clone();
                                panel.state_source = existing.state_source.clone();
                                panel.is_active = existing.is_active;
                            }
                        }
                    }

                    if let Err(err) = crate::session_cache::preload_panels(&mut panels) {
                        log_debug!("session_cache: preload after scan failed: {}", err);
                    }

                    self.panels = panels;
                    if self.selected_panel().is_none() {
                        self.focus_panel();
                    }
                    self.last_refresh = Instant::now();
                    self.invalidate_preview();
                    self.scan_in_progress = false;
                    self.scan_rx = None;
                    self.dirty = true;
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

    pub fn trigger_async_preview_update(&mut self, panel: AgentPanel) {
        if self.preview_update_in_progress {
            return;
        }

        self.preview_update_in_progress = true;
        let locale = self.locale;
        let preview_mode = self.config.preview.mode.clone();
        let (tx, rx) = mpsc::channel::<PreviewUpdate>(1);
        self.preview_rx = Some(rx);

        tokio::task::spawn_blocking(move || {
            let update = preview_source::load_preview(&panel, &preview_mode, locale);
            let _ = tx.blocking_send(update);
        });
    }

    pub fn check_preview_result(&mut self) {
        if let Some(ref mut rx) = self.preview_rx {
            match rx.try_recv() {
                Ok(update) => {
                    let previous_panel_cache_state = self
                        .panels
                        .iter()
                        .find(|panel| panel.pane_id == update.pane_id)
                        .and_then(|panel| panel.session_cache_state);
                    let previous_pane_id = self.preview_pane_id.clone();
                    let previous_source = self.preview_source;
                    let previous_content = self.preview_content.clone();
                    let previous_turns = self.preview_turns.clone();
                    let previous_selected_turn = self.preview_selected_turn;
                    let previous_expanded_turn = self.preview_expanded_turn;
                    let previous_list_scroll = self.preview_list_scroll;
                    let previous_follow_bottom = self.preview_follow_bottom;
                    let previous_follow_selection = self.preview_follow_selection;
                    let should_follow_bottom = self.preview_follow_bottom
                        || self.preview_pane_id.is_none()
                        || self.preview_pane_id.as_deref() != Some(update.pane_id.as_str());
                    let same_context = self.preview_pane_id.as_deref()
                        == Some(update.pane_id.as_str())
                        && self.preview_source == update.source;
                    self.preview_content = update.content;
                    self.preview_pane_id = Some(update.pane_id.clone());
                    self.preview_source = update.source;
                    if self.preview_source == crate::model::PreviewSource::Session
                        && !update.turns.is_empty()
                    {
                        if !same_context {
                            self.preview_selected_turn = None;
                            self.preview_expanded_turn = None;
                            self.preview_scroll = 0;
                            self.preview_list_scroll = 0;
                            self.preview_follow_selection = true;
                        } else {
                            self.preview_selected_turn = self
                                .preview_selected_turn
                                .filter(|idx| *idx < update.turns.len());
                            self.preview_expanded_turn = self
                                .preview_expanded_turn
                                .filter(|idx| *idx < update.turns.len());
                        }
                        self.preview_turns = update.turns.clone();
                        self.preview_follow_bottom = false;
                    } else {
                        self.preview_turns.clear();
                        self.preview_selected_turn = None;
                        self.preview_expanded_turn = None;
                        self.preview_list_scroll = 0;
                        self.preview_follow_bottom = should_follow_bottom;
                        self.preview_follow_selection = true;
                    }

                    let mut panel_cache_state_changed = false;
                    if let Some(panel) = self
                        .panels
                        .iter_mut()
                        .find(|panel| panel.pane_id == update.pane_id)
                    {
                        if let Some(transcript_path) = update.transcript_path.clone() {
                            panel.transcript_path = Some(transcript_path);
                        }
                        if self.preview_source == crate::model::PreviewSource::Session
                            && !update.turns.is_empty()
                        {
                            panel.cached_preview_turns = update.turns.clone();
                            panel.last_user_prompt =
                                update.turns.first().map(|turn| turn.question.clone());
                            panel.last_assistant_message =
                                update.turns.first().and_then(|turn| turn.answer.clone());
                            if let Some(state) = update.session_cache_state {
                                panel.session_cache_state = Some(state);
                            }
                        }
                        panel_cache_state_changed =
                            previous_panel_cache_state != panel.session_cache_state;
                    }

                    self.preview_update_in_progress = false;
                    self.preview_rx = None;
                    self.last_preview_update = Instant::now();
                    if previous_pane_id != self.preview_pane_id
                        || previous_source != self.preview_source
                        || previous_content != self.preview_content
                        || previous_turns != self.preview_turns
                        || previous_selected_turn != self.preview_selected_turn
                        || previous_expanded_turn != self.preview_expanded_turn
                        || previous_list_scroll != self.preview_list_scroll
                        || previous_follow_bottom != self.preview_follow_bottom
                        || previous_follow_selection != self.preview_follow_selection
                        || panel_cache_state_changed
                    {
                        self.dirty = true;
                    }
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.preview_update_in_progress = false;
                    self.preview_rx = None;
                }
            }
        }
    }

    pub fn check_preview_update(&mut self) {
        if self.preview_update_in_progress || self.scan_in_progress {
            return;
        }

        let panel = self.selected_panel().cloned();

        if let Some(panel) = panel {
            let refresh_ms = preview_source::preview_refresh_interval_ms(&panel);
            if self.last_preview_update.elapsed() < std::time::Duration::from_millis(refresh_ms) {
                return;
            }
            self.trigger_async_preview_update(panel);
        }
    }

    pub fn trigger_provider_test(&mut self, agent_idx: usize, provider_idx: usize) {
        if self.provider_test_in_progress {
            return;
        }
        let agent = match self.config.agents.get(agent_idx) {
            Some(a) => a,
            None => return,
        };
        let prov = match agent.providers.get(provider_idx) {
            Some(p) => p,
            None => return,
        };

        let base_url = prov.base_url.clone();
        let api_key = prov.api_key.clone();

        if base_url.is_empty() || api_key.is_empty() {
            // Mark as failed immediately
            if let Some(agent) = self.config.agents.get_mut(agent_idx) {
                if let Some(prov) = agent.providers.get_mut(provider_idx) {
                    prov.test_status = Some(false);
                    prov.test_result = Some("Base URL or API Key is empty".to_string());
                }
            }
            self.dirty = true;
            return;
        }

        self.provider_test_in_progress = true;
        let (tx, rx) = mpsc::channel(1);
        self.provider_test_rx = Some(rx);

        tokio::spawn(async move {
            let url = format!("{}/v1/models", base_url.trim_end_matches('/'));
            let output = tokio::process::Command::new("curl")
                .args([
                    "-s",
                    "--max-time",
                    "5",
                    "-H",
                    &format!("Authorization: Bearer {}", api_key),
                    &url,
                ])
                .output()
                .await;

            let (success, message) = match output {
                Ok(out) if out.status.success() => {
                    let body = String::from_utf8_lossy(&out.stdout).to_string();
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(data) = val.get("data").and_then(|d| d.as_array()) {
                            let models: Vec<String> = data
                                .iter()
                                .filter_map(|m| {
                                    m.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())
                                })
                                .take(10)
                                .collect();
                            if models.is_empty() {
                                (true, "Connected (no models listed)".to_string())
                            } else {
                                (true, format!("Models:\n{}", models.join("\n")))
                            }
                        } else if let Some(err) = val.get("error") {
                            let msg = err
                                .get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown error");
                            (false, format!("API error: {}", msg))
                        } else {
                            (true, format!("OK: {}", &body[..200.min(body.len())]))
                        }
                    } else {
                        (
                            false,
                            format!("Invalid JSON: {}", &body[..200.min(body.len())]),
                        )
                    }
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let msg = if !stderr.is_empty() { stderr } else { stdout };
                    (false, format!("Error: {}", &msg[..200.min(msg.len())]))
                }
                Err(e) => (false, format!("Failed: {}", e)),
            };

            let _ = tx.send((agent_idx, provider_idx, success, message)).await;
        });
    }

    pub fn check_provider_test_result(&mut self) {
        if let Some(ref mut rx) = self.provider_test_rx {
            match rx.try_recv() {
                Ok((agent_idx, prov_idx, success, message)) => {
                    if let Some(agent) = self.config.agents.get_mut(agent_idx) {
                        if let Some(prov) = agent.providers.get_mut(prov_idx) {
                            prov.test_status = Some(success);
                            prov.test_result = Some(message);
                        }
                    }
                    self.provider_test_in_progress = false;
                    self.provider_test_rx = None;
                    self.dirty = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.provider_test_in_progress = false;
                    self.provider_test_rx = None;
                }
            }
        }
    }
}
