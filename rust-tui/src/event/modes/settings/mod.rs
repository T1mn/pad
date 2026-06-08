mod appearance;
mod codex;
mod general;
mod list;
mod telegram;

use super::relay_settings::{handle_relay_key, RelayHost};
use crate::app::state::{SettingsDetailKind, SettingsFocus};
use crate::app::App;
use crossterm::event::KeyCode;

pub(crate) fn handle_settings_mode(app: &mut App, key: KeyCode) {
    if app.settings_searching {
        let _ = list::handle_settings_search_key(app, key);
        return;
    }

    if app.settings_focus == SettingsFocus::Detail && handle_settings_detail_mode(app, key) {
        return;
    }

    list::handle_settings_list_key(app, key);
}

fn handle_settings_detail_mode(app: &mut App, key: KeyCode) -> bool {
    if key == KeyCode::F(1) {
        app.close_settings();
        return true;
    }
    if key == KeyCode::Char('/') {
        app.leave_settings_detail();
        app.settings_searching = true;
        app.settings_search.clear();
        app.dirty = true;
        return true;
    }
    match app.current_settings_detail_kind() {
        Some(SettingsDetailKind::Theme) => appearance::handle_theme_detail_mode(app, key),
        Some(SettingsDetailKind::Language) => appearance::handle_language_detail_mode(app, key),
        Some(SettingsDetailKind::AgentStyle) => {
            appearance::handle_agent_style_detail_mode(app, key)
        }
        Some(SettingsDetailKind::CodexSettings) => {
            codex::handle_codex_settings_detail_mode(app, key)
        }
        Some(SettingsDetailKind::ClaudeFullAccess) => {
            general::handle_claude_full_access_detail_mode(app, key)
        }
        Some(SettingsDetailKind::Sound) => general::handle_sound_detail_mode(app, key),
        Some(SettingsDetailKind::Relay) => handle_relay_detail_mode(app, key),
        Some(SettingsDetailKind::Telegram) => telegram::handle_telegram_detail_mode(app, key),
        Some(SettingsDetailKind::AutoRefresh) => general::handle_auto_refresh_detail_mode(app, key),
        Some(SettingsDetailKind::PreviewMode) => general::handle_preview_mode_detail_mode(app, key),
        Some(SettingsDetailKind::DisplayMode) => general::handle_display_mode_detail_mode(app, key),
        Some(SettingsDetailKind::Trash) => general::handle_trash_detail_mode(app, key),
        Some(SettingsDetailKind::Version) => {
            match key {
                KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
                _ => {}
            }
            true
        }
        None => false,
    }
}

fn handle_relay_detail_mode(app: &mut App, key: KeyCode) -> bool {
    handle_relay_key(app, key, RelayHost::Settings)
}

#[cfg(test)]
mod tests;
