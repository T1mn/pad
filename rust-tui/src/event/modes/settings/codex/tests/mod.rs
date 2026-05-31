mod actions;
mod navigation;

use super::super::handle_settings_mode;
use crate::app::state::{CodexSettingsView, Mode, SettingsDetailKind, SettingsFocus};
use crate::app::App;
use crossterm::event::KeyCode;
use std::time::{SystemTime, UNIX_EPOCH};

fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("lock codex settings tests");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let home = std::env::temp_dir().join(format!("pad-codex-settings-{name}-{stamp}"));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create temp home");

    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);
    let result = f();
    if let Some(prev) = prev_home {
        std::env::set_var("HOME", prev);
    } else {
        std::env::remove_var("HOME");
    }
    let _ = std::fs::remove_dir_all(&home);
    result
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
