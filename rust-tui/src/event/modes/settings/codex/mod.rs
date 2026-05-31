mod actions;
#[cfg(test)]
mod tests;

use crate::app::state::CodexSettingsView;
use crate::app::App;
use crossterm::event::KeyCode;

pub(super) fn handle_codex_settings_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            if app.codex_settings_view == CodexSettingsView::Categories {
                app.leave_settings_detail();
            } else {
                app.codex_settings_view = CodexSettingsView::Categories;
                app.codex_settings_selected = app.codex_settings_category_selected;
                app.dirty = true;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => move_selection(app, 1),
        KeyCode::Char('k') | KeyCode::Up => move_selection(app, -1),
        KeyCode::Enter | KeyCode::Char(' ') => actions::apply_selected_codex_action(app),
        KeyCode::Char('u') if app.codex_settings_view == CodexSettingsView::Cli => {
            app.trigger_codex_cli_update();
            app.dirty = true;
        }
        _ => {}
    }
    true
}

fn move_selection(app: &mut App, delta: isize) {
    let count = app.codex_settings_view.item_count();
    if count == 0 {
        return;
    }

    let max_index = count.saturating_sub(1);
    let current = app.codex_settings_selected.min(max_index);
    let next = if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current.saturating_add(delta as usize).min(max_index)
    };

    app.codex_settings_selected = next;
    if app.codex_settings_view == CodexSettingsView::Categories {
        app.codex_settings_category_selected = next;
    }
    app.dirty = true;
}
