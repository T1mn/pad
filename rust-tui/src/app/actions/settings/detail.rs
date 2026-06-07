use super::super::*;
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
                self.codex_settings_view = crate::app::state::CodexSettingsView::Categories;
                self.codex_settings_category_selected = 0;
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
        if self.active_settings_detail == Some(SettingsDetailKind::CodexSettings) {
            self.codex_settings_view = crate::app::state::CodexSettingsView::Categories;
            self.codex_settings_category_selected = 0;
            self.codex_settings_selected = 0;
        }
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
}
