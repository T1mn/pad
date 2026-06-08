use crate::app::state::Mode;
use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn handle_global_key(app: &mut App, key: KeyEvent) -> bool {
    if key.code == KeyCode::F(2) && app.open_thread_title_editor() {
        return true;
    }

    if key.code == KeyCode::Char('T') && app.open_thread_tags_editor() {
        return true;
    }

    if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.open_tree_in_home();
        return true;
    }

    if key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.mode = Mode::Search;
        app.is_searching = true;
        app.search_query.clear();
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();
        app.dirty = true;
        return true;
    }

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
        KeyCode::Char('E') => {
            let _ = app.export_selected_opencode_thread();
            true
        }
        KeyCode::Char('S') => {
            let _ = app.export_sanitized_selected_opencode_thread();
            true
        }
        KeyCode::Char('I') => {
            let _ = app.import_opencode_thread_from_clipboard();
            true
        }
        KeyCode::Char('O') => {
            let _ = app.export_selected_opencode_stats();
            true
        }
        KeyCode::Char('P') => {
            let _ = app.export_opencode_diagnostics();
            true
        }
        KeyCode::Char('Y') => {
            let _ = app.attach_opencode_from_clipboard();
            true
        }
        KeyCode::Char('W') => {
            let _ = app.open_opencode_web_for_selected_thread();
            true
        }
        KeyCode::Char('X') => {
            let _ = app.run_opencode_prompt_from_clipboard();
            true
        }
        KeyCode::Char('B') => {
            let _ = app.serve_opencode_for_selected_thread();
            true
        }
        KeyCode::Char('G') => {
            let _ = app.open_opencode_pr_from_clipboard();
            true
        }
        KeyCode::Char('L') => {
            let _ = app.install_opencode_plugin_from_clipboard();
            true
        }
        KeyCode::Char('H') => {
            let _ = app.install_opencode_github_agent();
            true
        }
        KeyCode::Char('1') => {
            app.jump_to(0);
            true
        }
        KeyCode::Char('2') => {
            app.jump_to(1);
            true
        }
        KeyCode::Char('3') => {
            app.jump_to(2);
            true
        }
        KeyCode::Char('4') => {
            app.jump_to(3);
            true
        }
        KeyCode::Char('5') => {
            app.jump_to(4);
            true
        }
        KeyCode::Char('6') => {
            app.jump_to(5);
            true
        }
        KeyCode::Char('7') => {
            app.jump_to(6);
            true
        }
        KeyCode::Char('8') => {
            app.jump_to(7);
            true
        }
        KeyCode::Char('9') => {
            app.jump_to(8);
            true
        }
        _ => false,
    }
}
