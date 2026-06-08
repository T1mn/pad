use super::super::super::*;

impl App {
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
}
