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
}
