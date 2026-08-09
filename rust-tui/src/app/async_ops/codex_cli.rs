#[path = "codex_cli/commands.rs"]
pub(crate) mod commands;
#[path = "codex_cli/toast.rs"]
mod toast {
    use super::types::CodexCliVersionInfo;
    use crate::app::App;

    pub(super) fn show_codex_update_success_toast(app: &mut App, info: &CodexCliVersionInfo) {
        let local = info
            .local_version
            .clone()
            .unwrap_or_else(|| "?".to_string());
        let latest = info
            .latest_version
            .clone()
            .unwrap_or_else(|| "?".to_string());
        if is_chinese_locale(app) {
            app.show_action_toast("Codex 升级完成", &format!("当前 {local} · 最新 {latest}"));
        } else {
            app.show_action_toast(
                "Codex updated",
                &format!("Current {local} · latest {latest}"),
            );
        }
    }

    pub(super) fn show_codex_update_failure_toast(app: &mut App, err: &str) {
        if is_chinese_locale(app) {
            app.show_action_toast("Codex 升级失败", err);
        } else {
            app.show_action_toast("Codex update failed", err);
        }
    }

    fn is_chinese_locale(app: &App) -> bool {
        matches!(
            app.locale,
            crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
        )
    }
}
#[path = "codex_cli/types.rs"]
mod types {
    #[derive(Clone, Debug, Default)]
    pub struct CodexCliVersionInfo {
        pub binary_path: Option<String>,
        pub local_version: Option<String>,
        pub latest_version: Option<String>,
    }

    pub(crate) type CodexCliVersionCheckResult = CodexCliVersionInfo;
    pub(crate) type CodexCliUpdateResult = Result<CodexCliVersionInfo, String>;
}

use self::commands::{detect_codex_cli_version_info, update_codex_cli};
use self::toast::{show_codex_update_failure_toast, show_codex_update_success_toast};
pub use self::types::CodexCliVersionInfo;
pub(crate) use self::types::{CodexCliUpdateResult, CodexCliVersionCheckResult};
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
