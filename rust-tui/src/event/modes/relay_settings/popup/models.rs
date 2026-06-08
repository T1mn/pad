mod edit;
mod modify;
mod selection;

use crate::app::App;
use crossterm::event::KeyCode;

pub(super) fn handle_relay_popup_edit(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc => {
            app.relay_popup_editing = false;
            app.relay_popup_buffer.clear();
        }
        KeyCode::Enter => edit::commit_opencode_model_field_edit(app),
        KeyCode::Backspace => {
            app.relay_popup_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.relay_popup_buffer.push(c);
        }
        _ => {}
    }
    app.dirty = true;
    true
}

pub(super) fn handle_opencode_models_popup(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            app.clear_relay_popup_state();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(models_len) = selection::selected_provider_models_len(app) {
                let max = models_len.saturating_sub(1);
                if app.relay_popup_selected < max {
                    app.relay_popup_selected += 1;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up if app.relay_popup_selected > 0 => {
            app.relay_popup_selected -= 1;
        }
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
            app.relay_popup_field = (app.relay_popup_field + 1) % 2;
        }
        KeyCode::Enter => {
            edit::open_opencode_model_field_edit(app);
        }
        KeyCode::Char('a') => {
            modify::add_opencode_model(app);
        }
        KeyCode::Char('d') => {
            modify::delete_opencode_model(app);
        }
        _ => {}
    }
    app.dirty = true;
    true
}
