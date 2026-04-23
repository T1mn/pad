use crate::app::state::SettingsFocus;
use crate::app::App;
use crossterm::event::KeyCode;

pub(super) fn handle_settings_search_key(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc => {
            app.settings_searching = false;
            app.settings_search.clear();
            app.dirty = true;
        }
        KeyCode::Enter => {
            app.settings_searching = false;
            if !app.filtered_settings_items().is_empty() {
                app.enter_settings_detail();
            } else {
                app.dirty = true;
            }
        }
        KeyCode::Down => {
            move_settings_selection_down(app);
            app.dirty = true;
        }
        KeyCode::Up => {
            move_settings_selection_up(app);
            app.dirty = true;
        }
        KeyCode::Char(c) => {
            app.settings_search.push(c);
            app.settings_selected = 0;
            app.dirty = true;
        }
        KeyCode::Backspace => {
            app.settings_search.pop();
            app.settings_selected = 0;
            app.dirty = true;
        }
        _ => {}
    }
    true
}

pub(super) fn handle_settings_list_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::F(1) => {
            app.close_settings();
        }
        KeyCode::Char('/') => {
            app.settings_focus = SettingsFocus::List;
            app.settings_searching = true;
            app.settings_search.clear();
            app.settings_selected = 0;
            app.dirty = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            move_settings_selection_down(app);
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            move_settings_selection_up(app);
            app.dirty = true;
        }
        KeyCode::Char('1') => set_settings_selection(app, 0),
        KeyCode::Char('2') => set_settings_selection(app, 1),
        KeyCode::Char('3') => set_settings_selection(app, 2),
        KeyCode::Char('4') => set_settings_selection(app, 3),
        KeyCode::Char('5') => set_settings_selection(app, 4),
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            app.enter_settings_detail();
        }
        _ => {}
    }
}

fn set_settings_selection(app: &mut App, index: usize) {
    app.settings_selected = index.min(app.filtered_settings_items().len().saturating_sub(1));
    app.dirty = true;
}

pub(super) fn move_settings_selection_down(app: &mut App) {
    let max = app.filtered_settings_items().len().saturating_sub(1);
    if app.settings_selected < max {
        app.settings_selected += 1;
    }
}

pub(super) fn move_settings_selection_up(app: &mut App) {
    if app.settings_selected > 0 {
        app.settings_selected -= 1;
    }
}
