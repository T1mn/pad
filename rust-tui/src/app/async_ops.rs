use super::App;
use crate::model::AgentPanel;
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
                Ok(Ok(panels)) => {
                    self.panels = panels;
                    self.last_refresh = Instant::now();
                    self.preview_pane_id = None;
                    self.scan_in_progress = false;
                    self.scan_rx = None;
                    self.dirty = true;
                }
                Ok(Err(_)) => {
                    self.scan_in_progress = false;
                    self.scan_rx = None;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
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

    pub fn trigger_async_preview_update(&mut self, pane_id: String) {
        if self.preview_update_in_progress {
            return;
        }

        self.preview_update_in_progress = true;
        let (tx, rx) = mpsc::channel::<(String, String)>(1);
        self.preview_rx = Some(rx);

        tokio::task::spawn_blocking(move || {
            let content = match crate::pty::capture_pane(&pane_id, 50) {
                Ok(content) => content,
                Err(_) => String::from("Failed to capture pane"),
            };
            let _ = tx.blocking_send((pane_id, content));
        });
    }

    pub fn check_preview_result(&mut self) {
        if let Some(ref mut rx) = self.preview_rx {
            match rx.try_recv() {
                Ok((pane_id, content)) => {
                    self.preview_content = content;
                    self.preview_pane_id = Some(pane_id);
                    self.preview_scroll = 0;
                    self.preview_update_in_progress = false;
                    self.preview_rx = None;
                    self.last_preview_update = Instant::now();
                    self.dirty = true;
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
        if self.last_preview_update.elapsed() < std::time::Duration::from_millis(500) {
            return;
        }

        if self.preview_update_in_progress || self.scan_in_progress {
            return;
        }

        let pane_id = self.selected_panel().map(|p| p.pane_id.clone());

        if let Some(pane_id) = pane_id {
            let needs_update = match &self.preview_pane_id {
                None => true,
                Some(id) if id != &pane_id => true,
                _ => false,
            };

            if needs_update {
                self.trigger_async_preview_update(pane_id);
            }
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
                    "--max-time", "5",
                    "-H", &format!("Authorization: Bearer {}", api_key),
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
                        (false, format!("Invalid JSON: {}", &body[..200.min(body.len())]))
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
