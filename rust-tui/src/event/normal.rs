pub(crate) mod global_keys;
mod preview_keys {
    use crate::app::App;
    use crossterm::event::{KeyCode, KeyEvent};

    pub(super) fn handle_preview_key(app: &mut App, key: KeyEvent) -> bool {
        if !app.preview_is_focused() {
            return false;
        }

        match key.code {
            KeyCode::Esc => {
                app.step_back_preview_focus();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                app.scroll_preview_by(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.scroll_preview_by(-1);
            }
            KeyCode::Char('J') => {
                if app.has_session_preview_turns() {
                    app.select_next_preview_turn();
                } else {
                    app.scroll_preview_by(10);
                }
            }
            KeyCode::Char('K') => {
                if app.has_session_preview_turns() {
                    app.select_previous_preview_turn();
                } else {
                    app.scroll_preview_by(-10);
                }
            }
            KeyCode::PageDown => {
                app.scroll_preview_by(20);
            }
            KeyCode::PageUp => {
                app.scroll_preview_by(-20);
            }
            KeyCode::Home => {
                app.scroll_preview_to_top();
            }
            KeyCode::End => {
                app.scroll_preview_to_bottom();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let _ = app.toggle_preview_turn_expanded();
            }
            _ => {}
        }

        true
    }
}
mod sidebar_keys {
    use crate::app::state::Mode;
    use crate::app::App;
    use crossterm::event::{KeyCode, KeyEvent};
    use std::time::Duration;

    pub(super) fn handle_sidebar_key(app: &mut App, key: KeyEvent, is_space: bool) {
        const DOUBLE_SPACE_WINDOW: Duration = Duration::from_millis(250);

        if is_space && !app.sidebar.show_tree {
            if app.pending_sidebar_space_action_is_active() {
                app.clear_pending_sidebar_space_action();
                let _ = app.toggle_all_sidebar_folders();
            } else {
                let _ = app.queue_pending_sidebar_space_action(DOUBLE_SPACE_WINDOW);
            }
            return;
        }

        match key.code {
            KeyCode::Esc => {}
            KeyCode::Char('J') => {
                let _ = app.move_selected_sidebar_item_down();
            }
            KeyCode::Char('K') => {
                let _ = app.move_selected_sidebar_item_up();
            }
            KeyCode::Char('j') | KeyCode::Down => app.next(),
            KeyCode::Char('k') | KeyCode::Up => app.previous(),
            KeyCode::Char('h') | KeyCode::Left => {
                let _ = app.collapse_selected_folder();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let _ = app.expand_selected_folder();
            }
            KeyCode::Char('d') => {
                if let Some(panel) = app.selected_panel() {
                    app.sidebar.delete_target = Some(panel.clone());
                    app.mode = Mode::DeleteConfirm;
                    app.dirty = true;
                }
            }
            KeyCode::Char(' ') => {
                if app.sidebar.show_tree {
                    if let Some(ref mut tree) = app.sidebar.file_tree {
                        tree.toggle();
                    }
                    app.dirty = true;
                } else {
                    match app.selected_sidebar_item() {
                        Some(item) if item.as_folder().is_some() => {
                            let _ = app.toggle_selected_folder();
                        }
                        Some(item) if item.as_thread().is_some() => {
                            let _ = app.collapse_parent_folder_for_selected_thread();
                        }
                        _ => {}
                    }
                }
            }
            KeyCode::Enter => match app.selected_sidebar_item() {
                Some(item) if item.as_folder().is_some() => {
                    let _ = app.toggle_selected_folder();
                }
                Some(item) if item.as_thread().is_some() => {
                    if let Some(live_pane_id) = app
                        .selected_preview_thread()
                        .filter(|thread| thread.is_live())
                        .and_then(|thread| thread.live_pane_id)
                    {
                        if App::is_native_agent_terminal_id(&live_pane_id) {
                            match app.focus_native_agent_terminal(&live_pane_id) {
                                Ok(true) => {}
                                Ok(false) => show_unavailable_terminal(app, &live_pane_id),
                                Err(error) => {
                                    app.show_action_toast("PAD Terminal", &error.to_string())
                                }
                            }
                        } else {
                            show_unavailable_terminal(app, &live_pane_id);
                        }
                    } else {
                        app.invalidate_preview();
                        app.dirty = true;
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn show_unavailable_terminal(app: &mut App, pane_id: &str) {
        app.show_action_toast(
            "Terminal unavailable",
            &format!("Native mode cannot open this legacy live entry ({pane_id})."),
        );
    }
}
mod tab {
    use crate::app::App;
    use std::time::Duration;

    pub(super) fn handle_preview_tab(app: &mut App) {
        const DOUBLE_TAB_WINDOW: Duration = Duration::from_millis(350);

        if app.preview_is_focused() {
            if app.preview.view == crate::model::PreviewView::SessionDetail {
                app.note_detail_exit_tab();
                app.toggle_preview_focus();
                app.clear_panel_tab();
                return;
            }
            if app.recent_panel_tab_within(DOUBLE_TAB_WINDOW) && app.open_latest_preview_turn() {
                app.clear_panel_tab();
                return;
            }
            app.toggle_preview_focus();
            app.clear_panel_tab();
            return;
        }

        if app.recent_detail_exit_tab_within(DOUBLE_TAB_WINDOW)
            && app.selected_thread_matches_preview_target()
            && app.restore_preview_turns_list()
        {
            app.focus_panel();
            app.clear_detail_exit_tab();
            app.clear_panel_tab();
            return;
        }

        if app.toggle_preview_focus() {
            app.note_panel_tab();
            app.clear_detail_exit_tab();
        } else {
            app.clear_panel_tab();
            app.clear_detail_exit_tab();
        }
    }
}
pub(crate) mod terminal_keys;

use crate::app::App;
use crate::log_debug;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{backend::Backend, Terminal};
use std::io;

pub(super) fn handle_normal_mode<B: Backend>(
    _terminal: &mut Terminal<B>,
    app: &mut App,
    key: KeyEvent,
) -> io::Result<()> {
    log_debug!(
        "normal_mode key={:?} show_tree={} panels={}",
        key.code,
        app.sidebar.show_tree,
        app.panels.len()
    );

    if global_keys::handle_sidebar_layout_key(app, key) {
        return Ok(());
    }

    if terminal_keys::handle_terminal_key(app, key) {
        return Ok(());
    }

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

    sidebar_keys::handle_sidebar_key(app, key, is_space);
    Ok(())
}
