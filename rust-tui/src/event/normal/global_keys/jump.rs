use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};

pub(super) fn handle_numeric_jump(app: &mut App, key: KeyEvent) -> bool {
    let Some(index) = numeric_jump_index(key.code) else {
        return false;
    };
    app.jump_to(index);
    true
}

fn numeric_jump_index(code: KeyCode) -> Option<usize> {
    match code {
        KeyCode::Char('1') => Some(0),
        KeyCode::Char('2') => Some(1),
        KeyCode::Char('3') => Some(2),
        KeyCode::Char('4') => Some(3),
        KeyCode::Char('5') => Some(4),
        KeyCode::Char('6') => Some(5),
        KeyCode::Char('7') => Some(6),
        KeyCode::Char('8') => Some(7),
        KeyCode::Char('9') => Some(8),
        _ => None,
    }
}
