use super::handle_thread_action_confirm_mode;
use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn shift_delete_clears_thread_meta_buffer() {
    let mut app = editing_app("Very long title");

    handle_thread_action_confirm_mode(
        &mut app,
        KeyEvent::new(KeyCode::Delete, KeyModifiers::SHIFT),
    );

    assert!(app.sidebar.thread_meta_buffer.is_empty());
    assert!(app.dirty);
}

#[test]
fn plain_delete_does_not_clear_thread_meta_buffer() {
    let mut app = editing_app("Keep me");

    handle_thread_action_confirm_mode(&mut app, KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));

    assert_eq!(app.sidebar.thread_meta_buffer, "Keep me");
}

fn editing_app(buffer: &str) -> App {
    let mut app = App::new();
    app.sidebar.thread_meta_editing = true;
    app.sidebar.thread_meta_buffer = buffer.to_string();
    app.dirty = false;
    app
}
