use crossterm::event::{KeyEventKind, KeyEventState};

use super::*;

pub(crate) fn command_layer_accepts_explicit_prefixes() {
    assert!(is_command_chord(key(KeyCode::F(11), KeyModifiers::NONE)));
    assert!(is_command_chord(key(
        KeyCode::Char(' '),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT
    )));
    assert!(is_command_chord(key(
        KeyCode::Char(' '),
        KeyModifiers::CONTROL
    )));
    assert!(is_command_chord(key(
        KeyCode::Char('g'),
        KeyModifiers::CONTROL
    )));
    assert!(!is_command_chord(key(
        KeyCode::Char('g'),
        KeyModifiers::CONTROL | KeyModifiers::ALT
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
    assert!(is_close_chord(key(
        KeyCode::Char('w'),
        KeyModifiers::CONTROL
    )));
    assert!(is_close_chord(key(KeyCode::Char('w'), KeyModifiers::SUPER)));
    assert!(!is_close_chord(key(KeyCode::Char('w'), KeyModifiers::ALT)));
    assert!(is_quit_chord(key(KeyCode::F(16), KeyModifiers::NONE)));
    assert!(is_quit_chord(key(KeyCode::Char('q'), KeyModifiers::SUPER)));

    let mut quit_app = App::new();
    assert!(handle_terminal_key(
        &mut quit_app,
        key(KeyCode::F(16), KeyModifiers::NONE)
    ));
    assert!(quit_app.should_quit);

    #[cfg(unix)]
    crate::test_support::with_temp_home("pad-terminal-keys", "global-command", |_| {
        let mut app = App::new();
        app.start_native_terminal(TerminalSize::new(80, 24))
            .unwrap();
        app.focus_panel();
        assert!(!app.terminal_is_focused());

        assert!(handle_terminal_key(
            &mut app,
            key(KeyCode::F(11), KeyModifiers::NONE)
        ));
        assert!(app.terminal_is_focused());
        assert_eq!(
            app.terminal_interaction(),
            &TerminalInteractionState::Command
        );

        assert!(handle_terminal_key(
            &mut app,
            key(KeyCode::F(11), KeyModifiers::NONE)
        ));
        let second = app
            .create_terminal_tab(TerminalProfile::Codex, TerminalSize::new(80, 24))
            .unwrap();
        app.focus_panel();
        assert!(handle_terminal_key(
            &mut app,
            key(KeyCode::Char('w'), KeyModifiers::CONTROL)
        ));
        assert!(app.terminal_workspace().pane(second).is_none());
        assert_eq!(app.terminal_workspace().panes.len(), 1);

        assert!(handle_terminal_key(
            &mut app,
            key(KeyCode::Char('w'), KeyModifiers::CONTROL)
        ));
        assert!(app.terminal_workspace().panes.is_empty());
        assert!(!app.terminal_is_focused());

        app.shutdown_native_terminal().unwrap();
    });
}

pub(crate) fn command_actions_cover_layout_profiles_and_navigation() {
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
        terminal_command_action(key(KeyCode::Char('5'), KeyModifiers::NONE)),
        Some(TerminalCommandAction::NewTab(TerminalProfile::OpenCode))
    );
    assert_eq!(
        terminal_command_action(key(KeyCode::Char('6'), KeyModifiers::NONE)),
        Some(TerminalCommandAction::NewTab(TerminalProfile::Pi))
    );
    assert_eq!(
        terminal_command_action(key(KeyCode::Char('o'), KeyModifiers::NONE)),
        Some(TerminalCommandAction::Split(
            TerminalSplitAxis::Columns,
            TerminalProfile::OpenCode
        ))
    );
    assert_eq!(
        terminal_command_action(key(KeyCode::Char('p'), KeyModifiers::NONE)),
        Some(TerminalCommandAction::Split(
            TerminalSplitAxis::Columns,
            TerminalProfile::Pi
        ))
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

pub(crate) fn shift_navigation_keys_control_pad_scrollback() {
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
