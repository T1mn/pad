#[path = "codex_cli/commands.rs"]
mod commands;
#[path = "codex_cli/toast.rs"]
mod toast;

use self::commands::{detect_codex_cli_version_info, update_codex_cli};
use self::toast::{show_codex_update_failure_toast, show_codex_update_success_toast};
use crate::app::App;
use std::thread;
use tokio::sync::mpsc;

impl App {
    pub fn trigger_codex_cli_version_check(&mut self) {
        if self.codex_cli_check_in_progress {
            return;
        }

        self.codex_cli_check_in_progress = true;
        let (tx, rx) = mpsc::channel(1);
        self.codex_cli_check_rx = Some(rx);
        self.dirty = true;

        thread::spawn(move || {
            let _ = tx.blocking_send(detect_codex_cli_version_info());
        });
    }

    pub fn check_codex_cli_version_result(&mut self) {
        if let Some(ref mut rx) = self.codex_cli_check_rx {
            match rx.try_recv() {
                Ok(info) => {
                    self.codex_cli_version_info = Some(info);
                    self.codex_cli_check_in_progress = false;
                    self.codex_cli_check_rx = None;
                    self.dirty = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.codex_cli_check_in_progress = false;
                    self.codex_cli_check_rx = None;
                    self.dirty = true;
                }
            }
        }
    }

    pub fn trigger_codex_cli_update(&mut self) {
        if self.codex_cli_update_in_progress {
            return;
        }

        self.codex_cli_update_in_progress = true;
        let (tx, rx) = mpsc::channel(1);
        self.codex_cli_update_rx = Some(rx);
        self.dirty = true;

        thread::spawn(move || {
            let _ = tx.blocking_send(update_codex_cli());
        });
    }

    pub fn check_codex_cli_update_result(&mut self) {
        if let Some(ref mut rx) = self.codex_cli_update_rx {
            match rx.try_recv() {
                Ok(result) => {
                    self.codex_cli_update_in_progress = false;
                    self.codex_cli_update_rx = None;
                    match result {
                        Ok(info) => {
                            self.codex_cli_version_info = Some(info.clone());
                            show_codex_update_success_toast(self, &info);
                        }
                        Err(err) => show_codex_update_failure_toast(self, &err),
                    }
                    self.dirty = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.codex_cli_update_in_progress = false;
                    self.codex_cli_update_rx = None;
                    self.dirty = true;
                }
            }
        }
    }
}
