use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(crate) fn handle_thread_action_confirm_mode(app: &mut App, key: KeyEvent) {
    if app.sidebar.thread_meta_editing {
        match key.code {
            KeyCode::Esc => {
                app.cancel_thread_meta_edit();
            }
            KeyCode::Enter => {
                app.commit_thread_meta_edit();
            }
            KeyCode::Backspace => {
                app.sidebar.thread_meta_buffer.pop();
                app.dirty = true;
            }
            KeyCode::Delete if key.modifiers.contains(KeyModifiers::SHIFT) => {
                app.sidebar.thread_meta_buffer.clear();
                app.dirty = true;
            }
            KeyCode::Char(c) => {
                app.sidebar.thread_meta_buffer.push(c);
                app.dirty = true;
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.confirm_thread_action();
        }
        _ => {
            app.close_thread_action_confirm();
        }
    }
}

#[cfg(test)]
#[path = "thread_action_confirm_tests.rs"]
mod tests;
