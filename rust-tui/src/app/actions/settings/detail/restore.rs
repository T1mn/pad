use super::super::super::*;

impl App {
    pub fn leave_settings_detail(&mut self) {
        self.restore_settings_detail_preview_state();
        if self.active_settings_detail == Some(SettingsDetailKind::CodexSettings) {
            self.reset_codex_settings_detail();
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

    pub(in crate::app::actions::settings::detail) fn reset_codex_settings_detail(&mut self) {
        self.codex_settings_view = crate::app::state::CodexSettingsView::Categories;
        self.codex_settings_category_selected = 0;
        self.codex_settings_selected = 0;
    }
}
