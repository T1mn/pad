mod codex_restart;
mod helpers;
mod notification_inbox;
mod opencode_attach;
mod opencode_cli;
mod opencode_diagnostics;
mod opencode_export;
mod opencode_github;
mod opencode_import;
mod opencode_plugin;
mod opencode_pr;
mod opencode_run;
mod opencode_serve;
mod opencode_stats;
mod opencode_web;
mod panel_width {
    use super::App;

    const AGENT_PANEL_WIDTH_STEP: u16 = 6;
    const MAX_AGENT_PANEL_WIDTH: u16 = 90;

    impl App {
        pub fn widen_agent_panel_width(&mut self, current_width: u16) {
            let next = current_width
                .saturating_add(AGENT_PANEL_WIDTH_STEP)
                .min(MAX_AGENT_PANEL_WIDTH);
            self.config.display.agent_panel_width = Some(next);
            self.sidebar.preferred_panel_width_cache = None;
            if self.save_config() {
                self.show_action_toast(
                    panel_width_toast_title(self.locale),
                    &panel_width_toast_body(self.locale, next),
                );
            }
            self.dirty = true;
        }
    }

    fn panel_width_toast_title(locale: crate::i18n::Locale) -> &'static str {
        match locale {
            crate::i18n::Locale::ZhCN => "左侧宽度已保存",
            crate::i18n::Locale::ZhTW => "左側寬度已儲存",
            _ => "Sidebar width saved",
        }
    }

    fn panel_width_toast_body(locale: crate::i18n::Locale, width: u16) -> String {
        match locale {
            crate::i18n::Locale::ZhCN => format!("Agent 列表宽度：{width}"),
            crate::i18n::Locale::ZhTW => format!("Agent 列表寬度：{width}"),
            _ => format!("Agent list width: {width}"),
        }
    }
}
mod relay_reload;
mod settings;
mod thread_actions;
mod thread_meta_edit;
mod thread_panel_delete;
mod tree;

use super::state::{Mode, SettingsDetailKind, SettingsFocus};
use super::{App, ThreadActionKind, ThreadMetaEditKind};
use crate::i18n::Locale;
use crate::log_debug;
use crate::model::AgentType;
use crate::sidebar::{SidebarItem, SidebarThread};

pub(crate) use helpers::settings_item_search_blob;

#[cfg(test)]
#[path = "actions/opencode_tests.rs"]
mod opencode_tests;

#[cfg(test)]
mod tests;
