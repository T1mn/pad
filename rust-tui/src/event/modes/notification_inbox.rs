use crate::app::App;
use crossterm::event::KeyCode;

pub(crate) fn handle_notification_inbox_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Left | KeyCode::Char('h') => {
            app.close_notification_inbox();
        }
        KeyCode::Char('j') | KeyCode::Down => app.move_notification_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_notification_selection(-1),
        KeyCode::Enter | KeyCode::Char('m') | KeyCode::Char(' ') => {
            app.mark_selected_notification_read();
        }
        KeyCode::Char('a') => {
            app.mark_all_notifications_read();
        }
        KeyCode::Char('d') => {
            app.delete_selected_notification();
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "notification_inbox_tests.rs"]
mod tests;
