mod global_keys;
mod preview_keys;
mod sidebar_keys;
mod tab;

use crate::app::App;
use crate::log_debug;
use crossterm::event::{KeyCode, KeyEvent};
#[cfg(not(test))]
use ratatui::backend::CrosstermBackend;
use ratatui::{backend::Backend, Terminal};
use std::io;

#[cfg(not(test))]
use super::attach::handle_attach;

#[cfg(not(test))]
pub(super) fn handle_normal_mode(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    key: KeyEvent,
) -> io::Result<()> {
    handle_normal_mode_impl(terminal, app, key, handle_attach)
}

#[cfg(test)]
pub(super) fn handle_normal_mode<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    key: KeyEvent,
) -> io::Result<()> {
    handle_normal_mode_impl(terminal, app, key, |_terminal, _app| {
        Err(io::Error::other("attach is not supported in event tests"))
    })
}

pub(super) fn handle_normal_mode_impl<B: Backend, F>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    key: KeyEvent,
    mut attach_fn: F,
) -> io::Result<()>
where
    F: FnMut(&mut Terminal<B>, &mut App) -> io::Result<()>,
{
    log_debug!(
        "normal_mode key={:?} show_tree={} panels={}",
        key.code,
        app.sidebar.show_tree,
        app.panels.len()
    );

    let is_tab = matches!(key.code, KeyCode::Tab);
    let is_space = matches!(key.code, KeyCode::Char(' '));

    if !is_space {
        app.flush_pending_sidebar_space_action();
    }

    if !app.sidebar.show_tree && is_tab {
        tab::handle_preview_tab(app);
        return Ok(());
    }

    if !is_tab {
        app.clear_panel_tab();
        app.clear_detail_exit_tab();
    }

    if global_keys::handle_global_key(app, key) {
        return Ok(());
    }

    if preview_keys::handle_preview_key(app, key) {
        return Ok(());
    }

    sidebar_keys::handle_sidebar_key(terminal, app, key, is_space, &mut attach_fn)
}
