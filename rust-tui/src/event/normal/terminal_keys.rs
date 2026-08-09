use crate::app::{App, TerminalInteractionState, TerminalProfile, TerminalSplitAxis};
use crate::terminal_runtime::{TerminalScroll, TerminalSize};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn handle_terminal_key(app: &mut App, key: KeyEvent) -> bool {
    if !app.terminal_is_focused() {
        return false;
    }
    if key.code == KeyCode::F(12) {
        app.cancel_terminal_command_layer();
        app.focus_panel();
        return true;
    }
    if matches!(
        app.terminal_interaction(),
        TerminalInteractionState::Rename { .. }
    ) {
        return handle_terminal_rename_key(app, key);
    }
    if is_command_chord(key) {
        if matches!(
            app.terminal_interaction(),
            TerminalInteractionState::Command
        ) {
            app.cancel_terminal_command_layer();
        } else {
            app.enter_terminal_command_layer();
        }
        return true;
    }
    if matches!(
        app.terminal_interaction(),
        TerminalInteractionState::Command
    ) {
        return handle_terminal_command_key(app, key);
    }
    if let Some(scroll) = terminal_scroll(key) {
        let _ = app.scroll_terminal(scroll);
        return true;
    }
    if let Some(bytes) = crate::terminal_runtime::encode_key_event(key, app.terminal_mode()) {
        let _ = app.send_terminal_input(bytes);
    }
    true
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalCommandAction {
    Cancel,
    NewTab(TerminalProfile),
    Split(TerminalSplitAxis, TerminalProfile),
    PreviousPane,
    NextPane,
    PreviousTab,
    NextTab,
    ClosePane,
    RenamePane,
}

fn handle_terminal_command_key(app: &mut App, key: KeyEvent) -> bool {
    let Some(action) = terminal_command_action(key) else {
        return true;
    };
    if action == TerminalCommandAction::RenamePane {
        app.begin_terminal_rename();
        return true;
    }
    app.cancel_terminal_command_layer();
    let size = focused_terminal_size(app);
    match action {
        TerminalCommandAction::Cancel => {}
        TerminalCommandAction::NewTab(profile) => {
            if let Err(error) = app.create_terminal_tab(profile, size) {
                app.show_action_toast("PAD Terminal", &error.to_string());
            }
        }
        TerminalCommandAction::Split(axis, profile) => {
            if let Err(error) = app.split_focused_terminal(axis, profile, size) {
                app.show_action_toast("PAD Terminal", &error.to_string());
            }
        }
        TerminalCommandAction::PreviousPane => {
            let result = app.cycle_terminal_pane(-1);
            show_terminal_result(app, result);
        }
        TerminalCommandAction::NextPane => {
            let result = app.cycle_terminal_pane(1);
            show_terminal_result(app, result);
        }
        TerminalCommandAction::PreviousTab => {
            let result = app.cycle_terminal_tab(-1);
            show_terminal_result(app, result);
        }
        TerminalCommandAction::NextTab => {
            let result = app.cycle_terminal_tab(1);
            show_terminal_result(app, result);
        }
        TerminalCommandAction::ClosePane => {
            // Keep one live pane so the workspace always has a keyboard entry
            // point for opening more tabs/splits.
            if app.terminal_workspace().panes.len() > 1 {
                let result = app.close_focused_terminal();
                show_terminal_result(app, result);
            }
        }
        TerminalCommandAction::RenamePane => unreachable!("rename handled above"),
    }
    true
}

fn show_terminal_result(
    app: &mut App,
    result: Result<bool, crate::terminal_runtime::TerminalError>,
) {
    if let Err(error) = result {
        app.show_action_toast("PAD Terminal", &error.to_string());
    }
}

fn handle_terminal_rename_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => app.cancel_terminal_command_layer(),
        KeyCode::Enter => {
            if let Err(error) = app.commit_terminal_rename() {
                app.show_action_toast("PAD Terminal", &error.to_string());
            }
        }
        KeyCode::Backspace => {
            app.backspace_terminal_rename();
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.clear_terminal_rename();
        }
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            app.append_terminal_rename_text(&character.to_string());
        }
        _ => {}
    }
    true
}

fn terminal_command_action(key: KeyEvent) -> Option<TerminalCommandAction> {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return (key.code == KeyCode::Esc).then_some(TerminalCommandAction::Cancel);
    }
    match key.code {
        KeyCode::Esc => Some(TerminalCommandAction::Cancel),
        KeyCode::Char('n' | 't' | '1') => {
            Some(TerminalCommandAction::NewTab(TerminalProfile::Shell))
        }
        KeyCode::Char('2') => Some(TerminalCommandAction::NewTab(TerminalProfile::Codex)),
        KeyCode::Char('3') => Some(TerminalCommandAction::NewTab(TerminalProfile::Claude)),
        KeyCode::Char('4') => Some(TerminalCommandAction::NewTab(TerminalProfile::GithubCli)),
        KeyCode::Char('5') => Some(TerminalCommandAction::NewTab(TerminalProfile::OpenCode)),
        KeyCode::Char('v') => Some(TerminalCommandAction::Split(
            TerminalSplitAxis::Columns,
            TerminalProfile::Shell,
        )),
        KeyCode::Char('s') => Some(TerminalCommandAction::Split(
            TerminalSplitAxis::Rows,
            TerminalProfile::Shell,
        )),
        KeyCode::Char('c') => Some(TerminalCommandAction::Split(
            TerminalSplitAxis::Columns,
            TerminalProfile::Codex,
        )),
        KeyCode::Char('a') => Some(TerminalCommandAction::Split(
            TerminalSplitAxis::Columns,
            TerminalProfile::Claude,
        )),
        KeyCode::Char('g') => Some(TerminalCommandAction::Split(
            TerminalSplitAxis::Columns,
            TerminalProfile::GithubCli,
        )),
        KeyCode::Char('o') => Some(TerminalCommandAction::Split(
            TerminalSplitAxis::Columns,
            TerminalProfile::OpenCode,
        )),
        KeyCode::Char('h' | 'k') | KeyCode::Left | KeyCode::Up => {
            Some(TerminalCommandAction::PreviousPane)
        }
        KeyCode::Char('j' | 'l') | KeyCode::Right | KeyCode::Down => {
            Some(TerminalCommandAction::NextPane)
        }
        KeyCode::Char('[') => Some(TerminalCommandAction::PreviousTab),
        KeyCode::Char(']') => Some(TerminalCommandAction::NextTab),
        KeyCode::Char('x') => Some(TerminalCommandAction::ClosePane),
        KeyCode::Char('r') => Some(TerminalCommandAction::RenamePane),
        _ => None,
    }
}

fn focused_terminal_size(app: &App) -> TerminalSize {
    app.focused_terminal_pane()
        .and_then(|pane| pane.size())
        .unwrap_or_else(|| TerminalSize::new(80, 24))
}

fn terminal_scroll(key: KeyEvent) -> Option<TerminalScroll> {
    if !key.modifiers.contains(KeyModifiers::SHIFT) {
        return None;
    }
    match key.code {
        KeyCode::PageUp => Some(TerminalScroll::PageUp),
        KeyCode::PageDown => Some(TerminalScroll::PageDown),
        KeyCode::Home => Some(TerminalScroll::Top),
        KeyCode::End => Some(TerminalScroll::Bottom),
        _ => None,
    }
}

fn is_command_chord(key: KeyEvent) -> bool {
    key.code == KeyCode::F(11)
        || (key.code == KeyCode::Char(' ')
            && key
                .modifiers
                .contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT))
}

#[cfg(test)]
#[path = "terminal_keys_tests.rs"]
pub(crate) mod tests;
