use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use super::{attach, modes};

#[cfg_attr(test, allow(dead_code))]
pub(super) fn handle_attach(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    attach::handle_attach(terminal, app)
}

pub(super) fn handle_fuzzy_picker_mode(app: &mut App, key: crossterm::event::KeyEvent) {
    modes::handle_fuzzy_picker_mode(app, key);
}

pub(super) fn handle_relay_settings_mode(app: &mut App, key: KeyCode) {
    modes::handle_relay_settings_mode(app, key);
}

pub(super) fn handle_search_mode(app: &mut App, key: KeyCode) {
    modes::handle_search_mode(app, key);
}

pub(super) fn handle_settings_mode(app: &mut App, key: KeyCode) {
    modes::handle_settings_mode(app, key);
}

pub(super) fn handle_tree_mode(app: &mut App, key: KeyCode) {
    modes::handle_tree_mode(app, key);
}

pub(super) fn handle_file_preview_mode(app: &mut App, key: KeyCode) {
    modes::handle_file_preview_mode(app, key);
}

pub(super) fn handle_tree_search_mode(app: &mut App, key: KeyCode) {
    modes::handle_tree_search_mode(app, key);
}

pub(super) fn handle_agent_launcher_mode(app: &mut App, key: KeyCode) {
    modes::handle_agent_launcher_mode(app, key);
}

pub(super) fn handle_delete_confirm_mode(app: &mut App, key: KeyCode) {
    modes::handle_delete_confirm_mode(app, key);
}

pub(super) fn handle_thread_action_confirm_mode(app: &mut App, key: KeyEvent) {
    modes::handle_thread_action_confirm_mode(app, key);
}

pub(super) fn handle_help_mode(app: &mut App, key: KeyCode) {
    modes::handle_help_mode(app, key);
}

pub(super) fn handle_agent_style_mode(app: &mut App, key: KeyCode) {
    modes::handle_agent_style_mode(app, key);
}

pub(super) fn handle_telegram_settings_mode(app: &mut App, key: KeyCode) {
    modes::handle_telegram_settings_mode(app, key);
}

pub(super) fn handle_notification_inbox_mode(app: &mut App, key: KeyCode) {
    modes::handle_notification_inbox_mode(app, key);
}
