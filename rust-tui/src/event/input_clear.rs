use crate::app::state::Mode;
use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn handle_shift_delete(app: &mut App, key: KeyEvent) -> bool {
    if key.code != KeyCode::Delete || !key.modifiers.contains(KeyModifiers::SHIFT) {
        return false;
    }

    clear_active_text_input(app)
}

fn clear_active_text_input(app: &mut App) -> bool {
    if app.relay_popup_editing {
        app.relay_popup_buffer.clear();
        app.dirty = true;
        return true;
    }

    if app.relay_editing {
        app.relay_edit_buffer.clear();
        app.dirty = true;
        return true;
    }

    if app.telegram_editing {
        app.telegram_edit_buffer.clear();
        app.dirty = true;
        return true;
    }

    if app.sidebar.thread_meta_editing {
        app.sidebar.thread_meta_buffer.clear();
        app.dirty = true;
        return true;
    }

    if app.mode == Mode::Settings && app.settings_searching {
        app.settings_search.clear();
        app.settings_selected = 0;
        app.dirty = true;
        return true;
    }

    match app.mode {
        Mode::Search => {
            app.search_query.clear();
            app.invalidate_sidebar_visible_cache();
            app.sync_sidebar_selection();
            app.invalidate_preview();
            app.dirty = true;
            true
        }
        Mode::TreeSearch => {
            let Some(tree) = app.sidebar.file_tree.as_mut() else {
                return false;
            };
            tree.clear_search_query();
            app.update_file_preview();
            app.dirty = true;
            true
        }
        Mode::FuzzyPicker => {
            let Some(picker) = app.fuzzy_picker.as_mut() else {
                return false;
            };
            picker.clear_query();
            app.dirty = true;
            true
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "input_clear_tests.rs"]
mod input_clear_tests;
