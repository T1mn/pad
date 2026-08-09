use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::TerminalMode;

/// Encodes a crossterm key press for an xterm-compatible child process.
///
/// Modifier-only, lock, media, and platform keys without a portable xterm
/// representation return `None` instead of inventing bytes.
pub fn encode_key_event(key: KeyEvent, mode: TerminalMode) -> Option<Vec<u8>> {
    let modifiers = key.modifiers;
    let bytes = match key.code {
        KeyCode::Char(character) => encode_character(character, modifiers),
        KeyCode::Enter => with_alt(b"\r".to_vec(), modifiers),
        KeyCode::Tab => with_alt(b"\t".to_vec(), modifiers),
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Backspace => with_alt(vec![0x7f], modifiers),
        KeyCode::Esc => with_alt(vec![0x1b], modifiers),
        KeyCode::Null => with_alt(vec![0], modifiers),
        KeyCode::Up => encode_cursor('A', modifiers, mode.application_cursor),
        KeyCode::Down => encode_cursor('B', modifiers, mode.application_cursor),
        KeyCode::Right => encode_cursor('C', modifiers, mode.application_cursor),
        KeyCode::Left => encode_cursor('D', modifiers, mode.application_cursor),
        KeyCode::Home => encode_cursor('H', modifiers, mode.application_cursor),
        KeyCode::End => encode_cursor('F', modifiers, mode.application_cursor),
        KeyCode::Insert => encode_tilde_key(2, modifiers),
        KeyCode::Delete => encode_tilde_key(3, modifiers),
        KeyCode::PageUp => encode_tilde_key(5, modifiers),
        KeyCode::PageDown => encode_tilde_key(6, modifiers),
        KeyCode::F(number) => encode_function_key(number, modifiers)?,
        KeyCode::KeypadBegin => encode_cursor('E', modifiers, false),
        KeyCode::CapsLock
        | KeyCode::ScrollLock
        | KeyCode::NumLock
        | KeyCode::PrintScreen
        | KeyCode::Pause
        | KeyCode::Menu
        | KeyCode::Media(_)
        | KeyCode::Modifier(_) => return None,
    };
    Some(bytes)
}

pub fn encode_paste(text: &str, bracketed_paste: bool) -> Vec<u8> {
    if !bracketed_paste {
        return text.as_bytes().to_vec();
    }
    let mut bytes = Vec::with_capacity(text.len() + 12);
    bytes.extend_from_slice(b"\x1b[200~");
    bytes.extend_from_slice(text.as_bytes());
    bytes.extend_from_slice(b"\x1b[201~");
    bytes
}

/// Encodes an SGR mouse event using coordinates relative to the pane's inner
/// terminal grid. Legacy mouse encodings are intentionally not fabricated.
pub fn encode_mouse_event(event: MouseEvent, inner: Rect, mode: TerminalMode) -> Option<Vec<u8>> {
    if !mode.mouse_reporting || !mode.sgr_mouse || !inner.contains((event.column, event.row).into())
    {
        return None;
    }

    let (mut code, released) = match event.kind {
        MouseEventKind::Down(button) => (mouse_button_code(button), false),
        MouseEventKind::Up(button) => (mouse_button_code(button), true),
        MouseEventKind::Drag(button) => (32 + mouse_button_code(button), false),
        MouseEventKind::Moved => (35, false),
        MouseEventKind::ScrollUp => (64, false),
        MouseEventKind::ScrollDown => (65, false),
        MouseEventKind::ScrollLeft => (66, false),
        MouseEventKind::ScrollRight => (67, false),
    };
    if event.modifiers.contains(KeyModifiers::SHIFT) {
        code += 4;
    }
    if event
        .modifiers
        .intersects(KeyModifiers::ALT | KeyModifiers::META)
    {
        code += 8;
    }
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        code += 16;
    }
    let column = event.column - inner.x + 1;
    let row = event.row - inner.y + 1;
    let final_byte = if released { 'm' } else { 'M' };
    Some(format!("\x1b[<{code};{column};{row}{final_byte}").into_bytes())
}

fn mouse_button_code(button: MouseButton) -> u8 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    }
}

fn encode_character(character: char, modifiers: KeyModifiers) -> Vec<u8> {
    let mut bytes = if modifiers.contains(KeyModifiers::CONTROL) {
        control_character(character)
            .map(|byte| vec![byte])
            .unwrap_or_else(|| character.to_string().into_bytes())
    } else {
        character.to_string().into_bytes()
    };
    if modifiers.intersects(KeyModifiers::ALT | KeyModifiers::META) {
        bytes.insert(0, 0x1b);
    }
    bytes
}

fn control_character(character: char) -> Option<u8> {
    match character {
        '@' | ' ' | '2' => Some(0),
        'a'..='z' => Some(character as u8 - b'a' + 1),
        'A'..='Z' => Some(character as u8 - b'A' + 1),
        '[' | '3' => Some(0x1b),
        '\\' | '4' => Some(0x1c),
        ']' | '5' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '7' | '/' => Some(0x1f),
        '?' | '8' => Some(0x7f),
        _ => None,
    }
}

fn with_alt(mut bytes: Vec<u8>, modifiers: KeyModifiers) -> Vec<u8> {
    if modifiers.intersects(KeyModifiers::ALT | KeyModifiers::META) {
        bytes.insert(0, 0x1b);
    }
    bytes
}

fn encode_cursor(final_byte: char, modifiers: KeyModifiers, application_cursor: bool) -> Vec<u8> {
    let modifier = xterm_modifier(modifiers);
    if modifier == 1 {
        if application_cursor {
            format!("\x1bO{final_byte}").into_bytes()
        } else {
            format!("\x1b[{final_byte}").into_bytes()
        }
    } else {
        format!("\x1b[1;{modifier}{final_byte}").into_bytes()
    }
}

fn encode_tilde_key(number: u8, modifiers: KeyModifiers) -> Vec<u8> {
    let modifier = xterm_modifier(modifiers);
    if modifier == 1 {
        format!("\x1b[{number}~").into_bytes()
    } else {
        format!("\x1b[{number};{modifier}~").into_bytes()
    }
}

fn encode_function_key(number: u8, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    let modifier = xterm_modifier(modifiers);
    let ss3_final = match number {
        1 => Some('P'),
        2 => Some('Q'),
        3 => Some('R'),
        4 => Some('S'),
        _ => None,
    };
    if let Some(final_byte) = ss3_final {
        return Some(if modifier == 1 {
            format!("\x1bO{final_byte}").into_bytes()
        } else {
            format!("\x1b[1;{modifier}{final_byte}").into_bytes()
        });
    }

    let number = match number {
        5 => 15,
        6 => 17,
        7 => 18,
        8 => 19,
        9 => 20,
        10 => 21,
        11 => 23,
        12 => 24,
        _ => return None,
    };
    Some(if modifier == 1 {
        format!("\x1b[{number}~").into_bytes()
    } else {
        format!("\x1b[{number};{modifier}~").into_bytes()
    })
}

fn xterm_modifier(modifiers: KeyModifiers) -> u8 {
    1 + u8::from(modifiers.contains(KeyModifiers::SHIFT))
        + 2 * u8::from(modifiers.intersects(KeyModifiers::ALT | KeyModifiers::META))
        + 4 * u8::from(modifiers.contains(KeyModifiers::CONTROL))
        + 8 * u8::from(modifiers.contains(KeyModifiers::SUPER))
        + 16 * u8::from(modifiers.contains(KeyModifiers::HYPER))
}

#[cfg(test)]
#[path = "input_tests.rs"]
pub(crate) mod tests;
