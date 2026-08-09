use super::App;
use crate::i18n::Locale;
use crate::theme::{Config, ConfigRecovery};

impl App {
    /// 保存配置。失败时同时写 debug log 和弹 toast —— 用户改了 api_key 却没落盘
    /// 是必须看得见的错误，不能像以前那样 `let _ = fs::write(..)` 吞掉。
    pub fn save_config(&mut self) -> bool {
        match self.config.save() {
            Ok(()) => true,
            Err(err) => {
                crate::log_debug!("config: save failed: {}", err);
                let body = format!("{}\n{err}", Config::config_path().display());
                self.show_action_toast(save_failed_title(self.locale), &body);
                false
            }
        }
    }

    /// 启动时配置解析失败会静默回退默认值，这里把"这次用的是默认值 + 备份在哪"告诉用户。
    pub(super) fn notify_config_recovery(&mut self, recovery: &ConfigRecovery) {
        let body = recovery.describe();
        self.show_action_toast(recovery_title(self.locale), &body);
    }
}

fn save_failed_title(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCN => "配置保存失败",
        Locale::ZhTW => "設定儲存失敗",
        _ => "Config save failed",
    }
}

fn recovery_title(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCN => "配置损坏，已回退默认值",
        Locale::ZhTW => "設定損毀，已回退預設值",
        _ => "Broken config, using defaults",
    }
}

#[cfg(test)]
#[path = "config_persist_tests.rs"]
pub(crate) mod tests;
