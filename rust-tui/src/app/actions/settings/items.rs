use super::super::helpers::settings_item_search_blob;
use super::super::*;

impl App {
    pub fn settings_items(&self) -> Vec<(&'static str, String, &'static str, &'static str, bool)> {
        let l = self.locale;
        let preview_mode = match self.config.preview.mode.as_str() {
            "tmux" => crate::i18n::t(l, "settings.preview_mode_tmux"),
            "session" => crate::i18n::t(l, "settings.preview_mode_session"),
            _ => crate::i18n::t(l, "settings.preview_mode_auto"),
        };
        let display_mode = match self.config.display.session_scope.as_str() {
            "all" => crate::i18n::t(l, "settings.display_mode_all"),
            _ => crate::i18n::t(l, "settings.display_mode_live"),
        };
        let enabled_sound_events = [
            self.config.sound.completion.enabled,
            self.config.sound.approval.enabled,
            self.config.sound.timeout.enabled,
            self.config.sound.failure.enabled,
        ]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();
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
                if self.config.auto_refresh {
                    crate::i18n::t(l, "settings.on").to_string()
                } else {
                    crate::i18n::t(l, "settings.off").to_string()
                },
                "settings.auto_refresh",
                "settings.auto_refresh",
                true,
            ),
            (
                "codex_settings",
                format!(
                    "YOLO {}  ·  Fast {}  ·  Goal {}  ·  MA {}  ·  Web {}  ·  SL {}/3  ·  Sum {}",
                    if self.config.agent_permissions.codex_auto_full_access {
                        crate::i18n::t(l, "settings.on")
                    } else {
                        crate::i18n::t(l, "settings.off")
                    },
                    if self.config.codex.fast_mode {
                        crate::i18n::t(l, "settings.on")
                    } else {
                        crate::i18n::t(l, "settings.off")
                    },
                    if self.config.codex.goals {
                        crate::i18n::t(l, "settings.on")
                    } else {
                        crate::i18n::t(l, "settings.off")
                    },
                    if self.config.codex.multi_agent {
                        crate::i18n::t(l, "settings.on")
                    } else {
                        crate::i18n::t(l, "settings.off")
                    },
                    crate::i18n::t(
                        l,
                        match self.config.codex.web_search.as_str() {
                            "cached" => "settings.codex_web_search_cached",
                            "live" => "settings.codex_web_search_live",
                            "disabled" => "settings.codex_web_search_disabled",
                            _ => "settings.codex_web_search_default",
                        }
                    ),
                    self.config.codex.status_line_items().len(),
                    if self.config.codex.title_summary {
                        crate::i18n::t(l, "settings.on")
                    } else {
                        crate::i18n::t(l, "settings.off")
                    }
                ),
                "settings.codex_settings",
                "settings.codex_settings",
                true,
            ),
            (
                "claude_full_access",
                if self.config.agent_permissions.claude_auto_full_access {
                    crate::i18n::t(l, "settings.on").to_string()
                } else {
                    crate::i18n::t(l, "settings.off").to_string()
                },
                "settings.claude_full_access",
                "settings.claude_full_access",
                true,
            ),
            (
                "sound",
                if self.config.sound.enabled {
                    format!(
                        "{} · {enabled_sound_events}/4",
                        crate::i18n::t(l, "settings.on")
                    )
                } else {
                    crate::i18n::t(l, "settings.off").to_string()
                },
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
                if self.config.telegram.enabled {
                    crate::i18n::t(l, "settings.on").to_string()
                } else {
                    crate::i18n::t(l, "settings.off").to_string()
                },
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
                preview_mode.to_string(),
                "settings.preview_mode",
                "settings.preview_mode",
                true,
            ),
            (
                "display_mode",
                display_mode.to_string(),
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

    pub fn filtered_settings_items(
        &self,
    ) -> Vec<(&'static str, String, &'static str, &'static str, bool)> {
        let items = self.settings_items();
        if self.settings_search.is_empty() {
            return items;
        }
        let query = self.settings_search.to_lowercase();
        let l = self.locale;
        items
            .into_iter()
            .filter(|(id, value, name_key, desc_key, _)| {
                settings_item_search_blob(l, id, value, name_key, desc_key).contains(&query)
            })
            .collect()
    }
}
