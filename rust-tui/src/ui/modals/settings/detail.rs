mod simple;
mod telegram;

use self::simple::{detail_body_line, draw_simple_detail, simple_value_line};
use self::telegram::draw_telegram_detail;
use super::super::relay::draw_relay_in_area;
use super::codex::draw_codex_detail;
use super::detail_lists::{
    draw_agent_style_detail, draw_language_detail, draw_sound_detail, draw_theme_detail,
};
use crate::app::state::SettingsDetailKind;
use crate::app::App;
use crate::i18n::t;
use ratatui::{layout::Rect, Frame};

pub(super) fn draw_settings_detail_panel(f: &mut Frame, app: &App, area: Rect) {
    let Some(kind) = app.current_settings_detail_kind() else {
        return;
    };

    match kind {
        SettingsDetailKind::Theme => draw_theme_detail(f, app, area),
        SettingsDetailKind::AutoRefresh => draw_simple_detail(
            f,
            app,
            area,
            t(app.locale, "settings.auto_refresh"),
            simple_value_line(app, kind),
            vec![detail_body_line(app.locale, kind)],
            "Enter/Space toggle · Esc back",
        ),
        SettingsDetailKind::CodexSettings => draw_codex_detail(f, app, area),
        SettingsDetailKind::ClaudeFullAccess => draw_simple_detail(
            f,
            app,
            area,
            t(app.locale, "settings.claude_full_access"),
            simple_value_line(app, kind),
            vec![detail_body_line(app.locale, kind)],
            "Enter/Space toggle · Esc back",
        ),
        SettingsDetailKind::Sound => draw_sound_detail(f, app, area),
        SettingsDetailKind::Relay => draw_relay_in_area(f, app, area),
        SettingsDetailKind::Telegram => draw_telegram_detail(f, app, area),
        SettingsDetailKind::AgentStyle => draw_agent_style_detail(f, app, area),
        SettingsDetailKind::PreviewMode => draw_simple_detail(
            f,
            app,
            area,
            t(app.locale, "settings.preview_mode"),
            simple_value_line(app, kind),
            vec![detail_body_line(app.locale, kind)],
            "Enter/Space cycle · Esc back",
        ),
        SettingsDetailKind::DisplayMode => draw_simple_detail(
            f,
            app,
            area,
            t(app.locale, "settings.display_mode"),
            simple_value_line(app, kind),
            vec![detail_body_line(app.locale, kind)],
            "Enter/Space toggle · Esc back",
        ),
        SettingsDetailKind::Trash => draw_simple_detail(
            f,
            app,
            area,
            t(app.locale, "settings.trash"),
            simple_value_line(app, kind),
            vec![detail_body_line(app.locale, kind)],
            "Enter/Space open · Esc back",
        ),
        SettingsDetailKind::Language => draw_language_detail(f, app, area),
        SettingsDetailKind::Version => draw_simple_detail(
            f,
            app,
            area,
            t(app.locale, "settings.version"),
            simple_value_line(app, kind),
            vec![detail_body_line(app.locale, kind)],
            "Read only · Esc back",
        ),
    }
}
