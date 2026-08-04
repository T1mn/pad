use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn handle_terminal_key(app: &mut App, key: KeyEvent) -> bool {
    if !app.terminal_is_focused() {
        return false;
    }
    if is_terminal_escape(key) {
        app.focus_panel();
        return true;
    }
    if let Some(bytes) = crate::terminal_runtime::encode_key_event(key, app.terminal_mode()) {
        let _ = app.send_terminal_input(bytes);
    }
    true
}

fn is_terminal_escape(key: KeyEvent) -> bool {
    key.code == KeyCode::F(12)
        || (key.code == KeyCode::Char(' ')
            && key
                .modifiers
                .contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT))
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyEventKind, KeyEventState};

    use super::*;

    #[test]
    fn only_explicit_command_layer_keys_escape_the_terminal() {
        assert!(is_terminal_escape(key(KeyCode::F(12), KeyModifiers::NONE)));
        assert!(is_terminal_escape(key(
            KeyCode::Char(' '),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT
        )));
        assert!(!is_terminal_escape(key(
            KeyCode::Char('q'),
            KeyModifiers::NONE
        )));
        assert!(!is_terminal_escape(key(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL
        )));
        assert!(!is_terminal_escape(key(KeyCode::Tab, KeyModifiers::NONE)));
    }

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }
}
