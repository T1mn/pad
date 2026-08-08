use crossterm::event::{KeyEventKind, KeyEventState};

use super::*;

#[test]
fn command_layer_has_one_explicit_prefix() {
    assert!(is_command_chord(key(KeyCode::F(11), KeyModifiers::NONE)));
    assert!(is_command_chord(key(
        KeyCode::Char(' '),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT
    )));
    assert!(!is_command_chord(key(
        KeyCode::Char('q'),
        KeyModifiers::NONE
    )));
    assert!(!is_command_chord(key(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL
    )));
    assert!(!is_command_chord(key(KeyCode::Tab, KeyModifiers::NONE)));
}

#[test]
fn command_actions_cover_layout_profiles_and_navigation() {
    assert_eq!(
        terminal_command_action(key(KeyCode::Char('v'), KeyModifiers::NONE)),
        Some(TerminalCommandAction::Split(
            TerminalSplitAxis::Columns,
            TerminalProfile::Shell
        ))
    );
    assert_eq!(
        terminal_command_action(key(KeyCode::Char('c'), KeyModifiers::NONE)),
        Some(TerminalCommandAction::Split(
            TerminalSplitAxis::Columns,
            TerminalProfile::Codex
        ))
    );
    assert_eq!(
        terminal_command_action(key(KeyCode::Char('3'), KeyModifiers::NONE)),
        Some(TerminalCommandAction::NewTab(TerminalProfile::Claude))
    );
    assert_eq!(
        terminal_command_action(key(KeyCode::Char(']'), KeyModifiers::NONE)),
        Some(TerminalCommandAction::NextTab)
    );
    assert_eq!(
        terminal_command_action(key(KeyCode::Char('r'), KeyModifiers::NONE)),
        Some(TerminalCommandAction::RenamePane)
    );
    assert_eq!(
        terminal_command_action(key(KeyCode::Char('z'), KeyModifiers::NONE)),
        None
    );
}

#[test]
fn shift_navigation_keys_control_pad_scrollback() {
    assert_eq!(
        terminal_scroll(key(KeyCode::PageUp, KeyModifiers::SHIFT)),
        Some(TerminalScroll::PageUp)
    );
    assert_eq!(
        terminal_scroll(key(KeyCode::PageDown, KeyModifiers::SHIFT)),
        Some(TerminalScroll::PageDown)
    );
    assert_eq!(
        terminal_scroll(key(KeyCode::Home, KeyModifiers::SHIFT)),
        Some(TerminalScroll::Top)
    );
    assert_eq!(
        terminal_scroll(key(KeyCode::End, KeyModifiers::SHIFT)),
        Some(TerminalScroll::Bottom)
    );
    assert_eq!(
        terminal_scroll(key(KeyCode::PageUp, KeyModifiers::NONE)),
        None
    );
    assert_eq!(terminal_scroll(key(KeyCode::Up, KeyModifiers::SHIFT)), None);
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}
