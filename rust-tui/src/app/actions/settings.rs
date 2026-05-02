use super::helpers::settings_item_search_blob;
use super::*;
use crate::app::state::RelayView;

impl App {
    pub fn toggle_settings(&mut self) {
        self.settings_open = !self.settings_open;
        if self.settings_open {
            self.mode = Mode::Settings;
            self.theme_selector_open = false;
            self.settings_selected = 0;
            self.settings_focus = SettingsFocus::List;
            self.active_settings_detail = None;
            self.settings_searching = false;
            self.settings_search.clear();
        } else {
            self.close_settings();
            return;
        }
        self.dirty = true;
    }

    pub fn open_settings_search(&mut self) {
        self.settings_open = true;
        self.mode = Mode::Settings;
        self.theme_selector_open = false;
        self.settings_selected = 0;
        self.settings_focus = SettingsFocus::List;
        self.active_settings_detail = None;
        self.settings_searching = true;
        self.settings_search.clear();
        self.dirty = true;
    }

    pub fn close_settings(&mut self) {
        self.restore_settings_detail_preview_state();
        self.settings_open = false;
        self.theme_selector_open = false;
        self.settings_focus = SettingsFocus::List;
        self.active_settings_detail = None;
        self.settings_searching = false;
        self.settings_search.clear();
        self.mode = Mode::Normal;
        self.dirty = true;
    }

    pub fn current_settings_item_id(&self) -> Option<&'static str> {
        self.filtered_settings_items()
            .get(self.settings_selected)
            .map(|(id, _, _, _, _)| *id)
    }

    pub fn current_settings_detail_kind(&self) -> Option<SettingsDetailKind> {
        if self.settings_focus == SettingsFocus::Detail {
            return self.active_settings_detail;
        }
        self.settings_detail_kind_from_item_id(self.current_settings_item_id()?)
    }

    fn settings_detail_kind_from_item_id(
        &self,
        item_id: &'static str,
    ) -> Option<SettingsDetailKind> {
        Some(match item_id {
            "theme" => SettingsDetailKind::Theme,
            "auto_refresh" => SettingsDetailKind::AutoRefresh,
            "codex_settings" => SettingsDetailKind::CodexSettings,
            "claude_full_access" => SettingsDetailKind::ClaudeFullAccess,
            "sound" => SettingsDetailKind::Sound,
            "relay" => SettingsDetailKind::Relay,
            "telegram" => SettingsDetailKind::Telegram,
            "agent_style" => SettingsDetailKind::AgentStyle,
            "preview_mode" => SettingsDetailKind::PreviewMode,
            "display_mode" => SettingsDetailKind::DisplayMode,
            "trash" => SettingsDetailKind::Trash,
            "language" => SettingsDetailKind::Language,
            "version" => SettingsDetailKind::Version,
            _ => return None,
        })
    }

    pub fn enter_settings_detail(&mut self) {
        let Some(kind) = self.current_settings_detail_kind() else {
            self.settings_focus = SettingsFocus::List;
            self.active_settings_detail = None;
            self.settings_searching = false;
            self.dirty = true;
            return;
        };
        self.active_settings_detail = Some(kind);

        match kind {
            SettingsDetailKind::Theme => {
                self.preview.theme_before_preview = Some(self.config.theme.clone());
                self.theme_selected = Self::available_themes()
                    .iter()
                    .position(|(name, _)| *name == self.config.theme)
                    .unwrap_or(0);
            }
            SettingsDetailKind::Language => {
                self.locale = crate::i18n::Locale::from_str(&self.config.language);
                let locales = Self::available_locales();
                self.language_selected = locales
                    .iter()
                    .position(|l| l.as_str() == self.config.language)
                    .unwrap_or(0);
            }
            SettingsDetailKind::Relay => {
                self.relay_view = RelayView::AgentList;
                self.relay_selected_agent = self
                    .relay_selected_agent
                    .min(self.config.agents.len().saturating_sub(1));
                self.relay_selected_provider = 0;
                self.relay_edit_field = 0;
                self.relay_editing = false;
                self.relay_edit_buffer.clear();
                self.clear_relay_popup_state();
            }
            SettingsDetailKind::Telegram => {
                self.telegram_selected_field = 0;
                self.telegram_editing = false;
                self.telegram_edit_buffer.clear();
            }
            SettingsDetailKind::AgentStyle => {
                self.agent_style_selected = 0;
            }
            SettingsDetailKind::CodexSettings => {
                self.codex_settings_selected = 0;
            }
            SettingsDetailKind::Sound => {
                self.sound_settings_selected = 0;
            }
            SettingsDetailKind::Trash => {}
            _ => {}
        }
        self.settings_focus = SettingsFocus::Detail;
        self.settings_searching = false;
        self.dirty = true;
    }

    pub fn leave_settings_detail(&mut self) {
        self.restore_settings_detail_preview_state();
        self.settings_focus = SettingsFocus::List;
        self.active_settings_detail = None;
        self.relay_editing = false;
        self.relay_edit_buffer.clear();
        self.clear_relay_popup_state();
        self.telegram_editing = false;
        self.telegram_edit_buffer.clear();
        self.dirty = true;
    }

    pub fn restore_settings_detail_preview_state(&mut self) {
        match self.current_settings_detail_kind() {
            Some(SettingsDetailKind::Theme) => {
                if let Some(prev) = self.preview.theme_before_preview.take() {
                    self.theme = crate::theme::Theme::by_name(&prev);
                    self.clear_preview_render_caches();
                }
            }
            Some(SettingsDetailKind::Language) => {
                self.locale = crate::i18n::Locale::from_str(&self.config.language);
            }
            _ => {}
        }
    }

    #[allow(dead_code)]
    pub fn open_theme_selector(&mut self) {
        self.preview.theme_before_preview = Some(self.config.theme.clone());
        self.theme_selector_open = true;
        self.mode = Mode::ThemeSelector;
        self.theme_selected = 0;
        self.dirty = true;
    }

    pub fn close_theme_selector(&mut self) {
        if let Some(ref prev) = self.preview.theme_before_preview.take() {
            self.theme = crate::theme::Theme::by_name(prev);
        }
        self.theme_selector_open = false;
        self.mode = Mode::Settings;
        self.dirty = true;
    }

    pub fn available_locales() -> Vec<crate::i18n::Locale> {
        use crate::i18n::Locale;
        vec![
            Locale::En,
            Locale::ZhCN,
            Locale::ZhTW,
            Locale::Ja,
            Locale::De,
            Locale::Fr,
        ]
    }

    #[allow(dead_code)]
    pub fn open_language_selector(&mut self) {
        let locales = Self::available_locales();
        self.language_selected = locales.iter().position(|l| *l == self.locale).unwrap_or(0);
        self.mode = Mode::LanguageSelector;
        self.dirty = true;
    }

    pub fn close_language_selector(&mut self) {
        self.locale = crate::i18n::Locale::from_str(&self.config.language);
        self.mode = Mode::Settings;
        self.dirty = true;
    }

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

    pub fn available_themes() -> Vec<(&'static str, &'static str)> {
        vec![
            ("default", "Default"),
            ("dark", "Dark"),
            ("dracula", "Dracula"),
            ("nord", "Nord"),
            ("gruvbox", "Gruvbox"),
            ("catppuccin", "Catppuccin"),
            ("tokyo-night", "Tokyo Night"),
            ("monokai", "Monokai"),
            ("solarized-dark", "Solarized Dark"),
            ("solarized-light", "Solarized Light"),
            ("rose-pine", "Rose Pine"),
            ("one-dark", "One Dark"),
            ("github-light", "GitHub Light"),
            ("github-dark", "GitHub Dark"),
            ("everforest", "Everforest"),
        ]
    }
}
