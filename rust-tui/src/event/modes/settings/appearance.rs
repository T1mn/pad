use crate::app::state::SettingsFocus;
use crate::app::App;
use crossterm::event::KeyCode;

pub(super) fn handle_theme_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            app.leave_settings_detail();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = App::available_themes().len().saturating_sub(1);
            if app.theme_selected < max {
                app.theme_selected += 1;
            }
            let themes = App::available_themes();
            if let Some((name, _)) = themes.get(app.theme_selected) {
                app.preview_theme(name);
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.theme_selected > 0 {
                app.theme_selected -= 1;
            }
            let themes = App::available_themes();
            if let Some((name, _)) = themes.get(app.theme_selected) {
                app.preview_theme(name);
            }
            app.dirty = true;
        }
        KeyCode::Char(c @ '1'..='9') => {
            let idx = (c as usize) - ('1' as usize);
            app.theme_selected = idx.min(App::available_themes().len().saturating_sub(1));
            let themes = App::available_themes();
            if let Some((name, _)) = themes.get(app.theme_selected) {
                app.preview_theme(name);
            }
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let themes = App::available_themes();
            if let Some((name, _)) = themes.get(app.theme_selected) {
                app.apply_theme(name);
                app.settings_focus = SettingsFocus::List;
                app.dirty = true;
            }
        }
        _ => {}
    }
    true
}

pub(super) fn handle_language_detail_mode(app: &mut App, key: KeyCode) -> bool {
    let locales = App::available_locales();
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            app.leave_settings_detail();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = locales.len().saturating_sub(1);
            if app.language_selected < max {
                app.language_selected += 1;
            }
            if let Some(locale) = locales.get(app.language_selected) {
                app.locale = *locale;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.language_selected > 0 {
                app.language_selected -= 1;
            }
            if let Some(locale) = locales.get(app.language_selected) {
                app.locale = *locale;
            }
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if let Some(locale) = locales.get(app.language_selected) {
                app.locale = *locale;
                app.config.language = locale.as_str().to_string();
                app.config.save();
            }
            app.settings_focus = SettingsFocus::List;
            app.dirty = true;
        }
        _ => {}
    }
    true
}

pub(super) fn handle_agent_style_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            app.leave_settings_detail();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.agent_style_selected < 1 {
                app.agent_style_selected += 1;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.agent_style_selected > 0 {
                app.agent_style_selected -= 1;
            }
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            match app.agent_style_selected {
                0 => {
                    app.config.desired_agent_style.zoom =
                        if app.config.desired_agent_style.zoom == "auto" {
                            "keep".to_string()
                        } else {
                            "auto".to_string()
                        };
                }
                1 => {
                    app.config.desired_agent_style.status =
                        match app.config.desired_agent_style.status.as_str() {
                            "show" => "hide".to_string(),
                            "hide" => "keep".to_string(),
                            _ => "show".to_string(),
                        };
                }
                _ => {}
            }
            app.config.save();
            app.dirty = true;
        }
        _ => {}
    }
    true
}
