use crate::app::state::Mode;
use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};

pub(super) fn handle_primary_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
            true
        }
        KeyCode::Char('r') => {
            app.refresh_panels();
            app.dirty = true;
            true
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            app.open_notification_inbox();
            true
        }
        KeyCode::Char('R') => {
            app.restart_selected_codex_panel();
            true
        }
        KeyCode::Char('/') => {
            app.open_settings_search();
            true
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
            app.dirty = true;
            true
        }
        KeyCode::F(1) => {
            app.toggle_settings();
            app.dirty = true;
            true
        }
        KeyCode::Char('t') => {
            app.toggle_tree();
            true
        }
        KeyCode::Char('v') | KeyCode::Char('V') => {
            app.toggle_display_session_scope_view();
            true
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.open_fuzzy_picker();
            true
        }
        KeyCode::Char('Z') => {
            app.toggle_archived_threads_view();
            true
        }
        KeyCode::Char('A') => {
            let _ = app.request_archive_selected_thread();
            true
        }
        KeyCode::Char('U') => {
            let _ = app.request_unarchive_selected_thread();
            true
        }
        _ => false,
    }
}
