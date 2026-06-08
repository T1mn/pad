#[path = "items/values.rs"]
mod values;

use super::super::helpers::settings_item_matches_search;
use super::super::*;
use values::{codex_summary, display_mode_label, preview_mode_label, sound_summary, toggle_label};

type SettingsItem = (&'static str, String, &'static str, &'static str, bool);

impl App {
    pub fn settings_items(&self) -> Vec<SettingsItem> {
        let l = self.locale;
        let trash_count = crate::thread_meta::deleted_thread_count().unwrap_or_default();
        vec![
            (
                "theme",
                self.config.theme.clone(),
                "settings.theme",
                "settings.theme",
                true,
            ),
            (
                "auto_refresh",
                toggle_label(l, self.config.auto_refresh),
                "settings.auto_refresh",
                "settings.auto_refresh",
                true,
            ),
            (
                "codex_settings",
                codex_summary(&self.config, l),
                "settings.codex_settings",
                "settings.codex_settings",
                true,
            ),
            (
                "claude_full_access",
                toggle_label(l, self.config.agent_permissions.claude_auto_full_access),
                "settings.claude_full_access",
                "settings.claude_full_access",
                true,
            ),
            (
                "sound",
                sound_summary(&self.config, l),
                "settings.sound",
                "settings.sound_desc",
                true,
            ),
            (
                "relay",
                crate::i18n::t(l, "settings.configure").to_string(),
                "settings.relay",
                "settings.relay",
                true,
            ),
            (
                "telegram",
                toggle_label(l, self.config.telegram.enabled),
                "settings.telegram",
                "settings.telegram",
                true,
            ),
            (
                "agent_style",
                crate::i18n::t(l, "settings.configure").to_string(),
                "settings.agent_style",
                "settings.agent_style",
                true,
            ),
            (
                "preview_mode",
                preview_mode_label(&self.config, l),
                "settings.preview_mode",
                "settings.preview_mode",
                true,
            ),
            (
                "display_mode",
                display_mode_label(&self.config, l),
                "settings.display_mode",
                "settings.display_mode",
                true,
            ),
            (
                "trash",
                trash_count.to_string(),
                "settings.trash",
                "settings.trash",
                true,
            ),
            (
                "language",
                self.locale.display_name().to_string(),
                "settings.language",
                "settings.language",
                true,
            ),
            (
                "version",
                env!("CARGO_PKG_VERSION").to_string(),
                "settings.version",
                "settings.version",
                false,
            ),
        ]
    }

    pub fn filtered_settings_items(&self) -> Vec<SettingsItem> {
        let items = self.settings_items();
        if self.settings_search.is_empty() {
            return items;
        }
        let l = self.locale;
        items
            .into_iter()
            .filter(|(id, value, name_key, desc_key, _)| {
                settings_item_matches_search(
                    l,
                    id,
                    value,
                    name_key,
                    desc_key,
                    &self.settings_search,
                )
            })
            .collect()
    }
}
