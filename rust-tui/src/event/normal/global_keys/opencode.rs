use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};

pub(super) fn handle_opencode_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
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
        _ => false,
    }
}
