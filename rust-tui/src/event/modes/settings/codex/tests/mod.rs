mod actions;
mod navigation;

use super::super::handle_settings_mode;
use crate::app::state::{CodexSettingsView, Mode, SettingsDetailKind, SettingsFocus};
use crate::app::App;
use crossterm::event::KeyCode;

fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
    crate::test_support::with_temp_home("pad-codex-settings", name, |_| f())
}

fn open_codex_category(app: &mut App, category: usize) {
    app.codex_settings_view = CodexSettingsView::Categories;
    app.codex_settings_selected = category;
    app.codex_settings_category_selected = category;
    handle_settings_mode(app, KeyCode::Enter);
}

fn codex_settings_app() -> App {
    let mut app = App::new();
    app.mode = Mode::Settings;
    app.settings_open = true;
    app.settings_focus = SettingsFocus::Detail;
    app.active_settings_detail = Some(SettingsDetailKind::CodexSettings);
    app
}
