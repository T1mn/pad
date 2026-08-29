#[path = "items/values.rs"]
mod values {
    use crate::i18n::{t, Locale};
    use crate::theme::Config;

    pub(super) fn toggle_label(locale: Locale, enabled: bool) -> String {
        t(
            locale,
            if enabled {
                "settings.on"
            } else {
                "settings.off"
            },
        )
        .to_string()
    }

    pub(super) fn preview_mode_label(config: &Config, locale: Locale) -> String {
        let _ = config;
        t(locale, "settings.preview_mode_session").to_string()
    }

    pub(super) fn display_mode_label(config: &Config, locale: Locale) -> String {
        let key = match config.display.session_scope.as_str() {
            "all" => "settings.display_mode_all",
            _ => "settings.display_mode_live",
        };
        crate::i18n::t(locale, key).to_string()
    }

    pub(super) fn sound_summary(config: &Config, locale: Locale) -> String {
        let enabled_events = [
            config.sound.completion.enabled,
            config.sound.approval.enabled,
            config.sound.timeout.enabled,
            config.sound.failure.enabled,
        ]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();

        if config.sound.enabled {
            format!("{} · {enabled_events}/4", t(locale, "settings.on"))
        } else {
            t(locale, "settings.off").to_string()
        }
    }

    pub(super) fn codex_summary(config: &Config, locale: Locale) -> String {
        format!(
            "YOLO {}  ·  Fast {}  ·  Goal {}  ·  MA {}  ·  Web {}  ·  SL {}/3  ·  Sum {}",
            on_off(locale, config.agent_permissions.codex_auto_full_access),
            on_off(locale, config.codex.fast_mode),
            on_off(locale, config.codex.goals),
            on_off(locale, config.codex.multi_agent),
            t(
                locale,
                codex_web_search_key(config.codex.web_search.as_str())
            ),
            config.codex.status_line_items().len(),
            on_off(locale, config.codex.title_summary)
        )
    }

    pub(super) fn profile_summary(config: &Config, locale: Locale) -> String {
        let mode = match config.profile.effective_permission_mode() {
            crate::theme::ProfileConfig::SYSTEM_FULL_ACCESS => {
                "settings.profile_mode_system_full_access"
            }
            crate::theme::ProfileConfig::WORKSPACE_FULL_ACCESS => {
                "settings.profile_mode_workspace_full_access"
            }
            _ => "settings.profile_mode_guarded",
        };
        format!(
            "{}  ·  {} {}",
            t(locale, mode),
            t(locale, "settings.profile_unattended"),
            on_off(locale, config.profile.unattended),
        )
    }

    fn on_off(locale: Locale, enabled: bool) -> &'static str {
        t(
            locale,
            if enabled {
                "settings.on"
            } else {
                "settings.off"
            },
        )
    }

    fn codex_web_search_key(value: &str) -> &'static str {
        match value {
            "cached" => "settings.codex_web_search_cached",
            "live" => "settings.codex_web_search_live",
            "disabled" => "settings.codex_web_search_disabled",
            _ => "settings.codex_web_search_default",
        }
    }
}

use super::super::helpers::settings_item_matches_search;
use super::super::*;
use values::{
    codex_summary, display_mode_label, preview_mode_label, profile_summary, sound_summary,
    toggle_label,
};

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
                "profile",
                profile_summary(&self.config, l),
                "settings.profile",
                "settings.profile_desc",
                false,
            ),
            (
                "profile_permission_mode",
                Self::profile_permission_mode_label(&self.config, l),
                "settings.profile_permission_mode",
                "settings.profile_permission_mode_desc",
                true,
            ),
            (
                "profile_full_access",
                toggle_label(l, self.config.profile.full_access),
                "settings.profile_full_access",
                "settings.profile_full_access_desc",
                true,
            ),
            (
                "profile_unattended",
                toggle_label(l, self.config.profile.unattended),
                "settings.profile_unattended",
                "settings.profile_unattended_desc",
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

    fn profile_permission_mode_label(
        config: &crate::theme::Config,
        locale: crate::i18n::Locale,
    ) -> String {
        let key = match config.profile.default_permission_mode.as_str() {
            crate::theme::ProfileConfig::WORKSPACE_FULL_ACCESS => {
                "settings.profile_mode_workspace_full_access"
            }
            crate::theme::ProfileConfig::SYSTEM_FULL_ACCESS => {
                "settings.profile_mode_system_full_access"
            }
            _ => "settings.profile_mode_guarded",
        };
        crate::i18n::t(locale, key).to_string()
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
