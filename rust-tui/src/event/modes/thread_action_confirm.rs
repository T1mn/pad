use crate::app::App;
use crossterm::event::KeyCode;

pub(crate) fn handle_thread_action_confirm_mode(app: &mut App, key: KeyCode) {
    if app.sidebar.thread_meta_editing {
        match key {
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
            KeyCode::Char(c) => {
                app.sidebar.thread_meta_buffer.push(c);
                app.dirty = true;
            }
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.confirm_thread_action();
        }
        _ => {
            app.close_thread_action_confirm();
        }
    }
}
