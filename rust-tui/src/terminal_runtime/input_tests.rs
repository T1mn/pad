use crossterm::event::{KeyEventKind, KeyEventState, MouseEvent};

use super::*;

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

#[test]
fn characters_control_and_alt_are_encoded_without_shell_interpretation() {
    assert_eq!(
        encode_key_event(key(KeyCode::Char('界'), KeyModifiers::NONE), mode()),
        Some("界".as_bytes().to_vec())
    );
    assert_eq!(
        encode_key_event(key(KeyCode::Char('c'), KeyModifiers::CONTROL), mode()),
        Some(vec![3])
    );
    assert_eq!(
        encode_key_event(
            key(
                KeyCode::Char('x'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            ),
            mode()
        ),
        Some(vec![0x1b, 0x18])
    );
    assert_eq!(
        encode_key_event(key(KeyCode::Char('2'), KeyModifiers::CONTROL), mode()),
        Some(vec![0])
    );
}

#[test]
fn cursor_and_function_keys_honor_modes_and_modifiers() {
    assert_eq!(
        encode_key_event(key(KeyCode::Up, KeyModifiers::NONE), mode()),
        Some(b"\x1b[A".to_vec())
    );
    assert_eq!(
        encode_key_event(
            key(KeyCode::Up, KeyModifiers::NONE),
            TerminalMode {
                application_cursor: true,
                ..mode()
            }
        ),
        Some(b"\x1bOA".to_vec())
    );
    assert_eq!(
        encode_key_event(key(KeyCode::Left, KeyModifiers::CONTROL), mode()),
        Some(b"\x1b[1;5D".to_vec())
    );
    assert_eq!(
        encode_key_event(key(KeyCode::F(1), KeyModifiers::ALT), mode()),
        Some(b"\x1b[1;3P".to_vec())
    );
    assert_eq!(
        encode_key_event(key(KeyCode::F(12), KeyModifiers::SHIFT), mode()),
        Some(b"\x1b[24;2~".to_vec())
    );
    assert_eq!(
        encode_key_event(key(KeyCode::F(13), KeyModifiers::NONE), mode()),
        None
    );
}

#[test]
fn editing_keys_and_bracketed_paste_match_xterm_sequences() {
    assert_eq!(
        encode_key_event(key(KeyCode::Enter, KeyModifiers::NONE), mode()),
        Some(b"\r".to_vec())
    );
    assert_eq!(
        encode_key_event(key(KeyCode::Backspace, KeyModifiers::ALT), mode()),
        Some(vec![0x1b, 0x7f])
    );
    assert_eq!(
        encode_key_event(key(KeyCode::Delete, KeyModifiers::CONTROL), mode()),
        Some(b"\x1b[3;5~".to_vec())
    );
    assert_eq!(encode_paste("a\n界", false), "a\n界".as_bytes());
    assert_eq!(
        encode_paste("a\n界", true),
        "\x1b[200~a\n界\x1b[201~".as_bytes()
    );
}

#[test]
fn sgr_mouse_uses_inner_relative_coordinates_and_filters_borders() {
    let inner = Rect::new(10, 5, 20, 8);
    let mouse_mode = TerminalMode {
        mouse_reporting: true,
        sgr_mouse: true,
        ..mode()
    };
    assert_eq!(
        encode_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 12,
                row: 7,
                modifiers: KeyModifiers::CONTROL,
            },
            inner,
            mouse_mode,
        ),
        Some(b"\x1b[<16;3;3M".to_vec())
    );
    assert_eq!(
        encode_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column: 12,
                row: 7,
                modifiers: KeyModifiers::NONE,
            },
            inner,
            mouse_mode,
        ),
        Some(b"\x1b[<0;3;3m".to_vec())
    );
    assert_eq!(
        encode_mouse_event(
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 10,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            inner,
            mouse_mode,
        ),
        Some(b"\x1b[<65;1;1M".to_vec())
    );
    assert_eq!(
        encode_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 9,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            inner,
            mouse_mode,
        ),
        None
    );
}

fn mode() -> TerminalMode {
    TerminalMode::default()
}
