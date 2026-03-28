use crate::app::App;
use crossterm::event::KeyCode;

pub(crate) fn handle_language_selector_mode(app: &mut App, key: KeyCode) {
    let locales = App::available_locales();
    match key {
        KeyCode::Esc => {
            app.close_language_selector();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = locales.len().saturating_sub(1);
            if app.language_selected < max {
                app.language_selected += 1;
            }
            if let Some(l) = locales.get(app.language_selected) {
                app.locale = *l;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.language_selected > 0 {
                app.language_selected -= 1;
            }
            if let Some(l) = locales.get(app.language_selected) {
                app.locale = *l;
            }
            app.dirty = true;
        }
        KeyCode::Enter => {
            if let Some(l) = locales.get(app.language_selected) {
                app.locale = *l;
                app.config.language = l.as_str().to_string();
                app.config.save();
            }
            app.mode = crate::app::state::Mode::Settings;
            app.dirty = true;
        }
        _ => {}
    }
}
