use crate::app::App;
use crossterm::event::KeyCode;

pub(crate) fn handle_theme_selector_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.close_theme_selector();
            app.dirty = true;
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
        KeyCode::Enter => {
            let themes = App::available_themes();
            if let Some((name, _)) = themes.get(app.theme_selected) {
                app.apply_theme(name);
                app.theme_selector_open = false;
                app.mode = crate::app::state::Mode::Settings;
            }
            app.dirty = true;
        }
        _ => {}
    }
}
