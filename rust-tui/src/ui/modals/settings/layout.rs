use crate::app::state::{RelayView, SettingsDetailKind, SettingsFocus};
use crate::app::App;
use crate::ui::selection::render::recommended_list_modal_height;

pub(super) fn settings_modal_size(app: &App) -> (u16, u16) {
    if app.settings_focus == SettingsFocus::Detail && !app.settings_searching {
        settings_detail_modal_size(app)
    } else {
        settings_list_modal_size(app)
    }
}

fn settings_list_modal_size(app: &App) -> (u16, u16) {
    let locale = app.locale;
    let items = app.filtered_settings_items();
    let max_name = items
        .iter()
        .map(|(_, _, key, _, _)| crate::i18n::t(locale, key).len())
        .max()
        .unwrap_or(12) as u16;
    let max_value = items
        .iter()
        .map(|(_, value, _, _, _)| value.len())
        .max()
        .unwrap_or(8) as u16;
    let content_w = (max_name + max_value + 26).clamp(48, 72);
    let row_count = items.len().max(1) as u16;
    let content_h = recommended_list_modal_height(row_count, 2, 1, 1).clamp(12, 22);
    (content_w, content_h)
}

fn settings_detail_modal_size(app: &App) -> (u16, u16) {
    match app.current_settings_detail_kind() {
        Some(SettingsDetailKind::Theme) => {
            let row_count = App::available_themes().len().max(1) as u16;
            (
                58,
                recommended_list_modal_height(row_count, 2, 1, 1).clamp(16, 24),
            )
        }
        Some(SettingsDetailKind::Language) => {
            let row_count = App::available_locales().len().max(1) as u16;
            (
                56,
                recommended_list_modal_height(row_count, 2, 1, 1).clamp(12, 18),
            )
        }
        Some(SettingsDetailKind::AgentStyle) => (64, 12),
        Some(SettingsDetailKind::CodexSettings) => (76, 25),
        Some(SettingsDetailKind::Sound) => {
            (78, recommended_list_modal_height(9, 2, 1, 1).clamp(16, 22))
        }
        Some(SettingsDetailKind::AutoRefresh)
        | Some(SettingsDetailKind::ClaudeFullAccess)
        | Some(SettingsDetailKind::PreviewMode)
        | Some(SettingsDetailKind::DisplayMode)
        | Some(SettingsDetailKind::Trash)
        | Some(SettingsDetailKind::Version) => (60, 10),
        Some(SettingsDetailKind::Relay) => relay_modal_size(app),
        Some(SettingsDetailKind::Telegram) => (72, 13),
        None => settings_list_modal_size(app),
    }
}

fn relay_modal_size(app: &App) -> (u16, u16) {
    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let agent_name = selected_agent.map(|agent| agent.name.as_str());
    match app.relay_view {
        RelayView::AgentList => {
            let row_count = app.config.agents.len().max(1) as u16;
            (
                58,
                recommended_list_modal_height(row_count, 2, 1, 1).max(12),
            )
        }
        RelayView::ProviderList => {
            let row_count = selected_agent
                .map(|agent| agent.providers.len().max(1) as u16)
                .unwrap_or(1);
            (
                76,
                recommended_list_modal_height(row_count, 2, 1, 1).max(12),
            )
        }
        RelayView::DetailPane => {
            let provider =
                selected_agent.and_then(|agent| agent.providers.get(app.relay_selected_provider));
            let base_lines = match agent_name {
                Some("codex") => 18u16,
                Some("opencode") => 22u16,
                _ => 14u16,
            };
            let test_lines = if app.provider_test_in_progress {
                2
            } else if provider
                .map(|prov| prov.test_result.is_some())
                .unwrap_or(false)
            {
                4
            } else {
                0
            };
            (
                match agent_name {
                    Some("codex") => 82,
                    Some("opencode") => 78,
                    _ => 68,
                },
                base_lines + test_lines,
            )
        }
    }
}
