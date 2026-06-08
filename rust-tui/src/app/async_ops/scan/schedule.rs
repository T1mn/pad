use super::super::super::App;
use super::super::ScanResult;
use crate::log_debug;
use crate::scanner::scan_panels;
use std::time::Instant;
use tokio::sync::mpsc;

impl App {
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
