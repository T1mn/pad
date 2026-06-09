use super::super::super::*;
use crate::app::state::RelayView;

impl App {
    pub fn enter_settings_detail(&mut self) {
        let Some(kind) = self.current_settings_detail_kind() else {
            self.return_to_settings_list();
            return;
        };
        let route_query = self.settings_search.clone();
        self.active_settings_detail = Some(kind);
        self.prepare_settings_detail(kind);
        super::search_route::apply_settings_search_route(self, kind, &route_query);
        self.settings_focus = SettingsFocus::Detail;
        self.settings_searching = false;
        self.dirty = true;
    }

    fn return_to_settings_list(&mut self) {
        self.settings_focus = SettingsFocus::List;
        self.active_settings_detail = None;
        self.settings_searching = false;
        self.dirty = true;
    }

    fn prepare_settings_detail(&mut self, kind: SettingsDetailKind) {
        match kind {
            SettingsDetailKind::Theme => self.prepare_theme_detail(),
            SettingsDetailKind::Language => self.prepare_language_detail(),
            SettingsDetailKind::Relay => self.prepare_relay_detail(),
            SettingsDetailKind::Telegram => self.prepare_telegram_detail(),
            SettingsDetailKind::AgentStyle => self.agent_style_selected = 0,
            SettingsDetailKind::CodexSettings => self.reset_codex_settings_detail(),
            SettingsDetailKind::Sound => self.sound_settings_selected = 0,
            SettingsDetailKind::Trash => {}
            _ => {}
        }
    }

    fn prepare_theme_detail(&mut self) {
        self.preview.theme_before_preview = Some(self.config.theme.clone());
        self.theme_selected = Self::available_themes()
            .iter()
            .position(|(name, _)| *name == self.config.theme)
            .unwrap_or(0);
    }

    fn prepare_language_detail(&mut self) {
        self.locale = crate::i18n::Locale::from_str(&self.config.language);
        let locales = Self::available_locales();
        self.language_selected = locales
            .iter()
            .position(|l| l.as_str() == self.config.language)
            .unwrap_or(0);
    }

    fn prepare_relay_detail(&mut self) {
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

    fn prepare_telegram_detail(&mut self) {
        self.telegram_selected_field = 0;
        self.telegram_editing = false;
        self.telegram_edit_buffer.clear();
    }
}
