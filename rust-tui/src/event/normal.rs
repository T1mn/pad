use crate::app::state::Mode;
use crate::app::App;
use crate::log_debug;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
#[cfg(not(test))]
use ratatui::backend::CrosstermBackend;
use ratatui::{backend::Backend, Terminal};
use std::io;
use std::time::Duration;

#[cfg(not(test))]
use super::mode_dispatch::handle_attach;

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
    const DOUBLE_SPACE_WINDOW: Duration = Duration::from_millis(250);

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

    if !app.sidebar.show_tree && matches!(key.code, KeyCode::Tab) {
        handle_preview_tab(app);
        return Ok(());
    }

    if !is_tab {
        app.clear_panel_tab();
        app.clear_detail_exit_tab();
    }

    if key.code == KeyCode::F(2) && app.open_thread_title_editor() {
        return Ok(());
    }

    if key.code == KeyCode::Char('T') && app.open_thread_tags_editor() {
        return Ok(());
    }

    if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.open_tree_in_home();
        return Ok(());
    }

    if key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.mode = Mode::Search;
        app.is_searching = true;
        app.search_query.clear();
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();
        app.dirty = true;
        return Ok(());
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
            return Ok(());
        }
        KeyCode::Char('r') => {
            app.refresh_panels();
            app.dirty = true;
            return Ok(());
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            app.open_notification_inbox();
            return Ok(());
        }
        KeyCode::Char('R') => {
            app.restart_selected_codex_panel();
            return Ok(());
        }
        KeyCode::Char('/') => {
            app.open_settings_search();
            return Ok(());
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
            app.dirty = true;
            return Ok(());
        }
        KeyCode::F(1) => {
            app.toggle_settings();
            app.dirty = true;
            return Ok(());
        }
        KeyCode::Char('t') => {
            app.toggle_tree();
            return Ok(());
        }
        KeyCode::Char('v') | KeyCode::Char('V') => {
            app.toggle_display_session_scope_view();
            return Ok(());
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.open_fuzzy_picker();
            return Ok(());
        }
        KeyCode::Char('Z') => {
            app.toggle_archived_threads_view();
            return Ok(());
        }
        KeyCode::Char('A') => {
            let _ = app.request_archive_selected_thread();
            return Ok(());
        }
        KeyCode::Char('U') => {
            let _ = app.request_unarchive_selected_thread();
            return Ok(());
        }
        KeyCode::Char('E') => {
            let _ = app.export_selected_opencode_thread();
            return Ok(());
        }
        KeyCode::Char('S') => {
            let _ = app.export_sanitized_selected_opencode_thread();
            return Ok(());
        }
        KeyCode::Char('I') => {
            let _ = app.import_opencode_thread_from_clipboard();
            return Ok(());
        }
        KeyCode::Char('O') => {
            let _ = app.export_selected_opencode_stats();
            return Ok(());
        }
        KeyCode::Char('1') => {
            app.jump_to(0);
            return Ok(());
        }
        KeyCode::Char('2') => {
            app.jump_to(1);
            return Ok(());
        }
        KeyCode::Char('3') => {
            app.jump_to(2);
            return Ok(());
        }
        KeyCode::Char('4') => {
            app.jump_to(3);
            return Ok(());
        }
        KeyCode::Char('5') => {
            app.jump_to(4);
            return Ok(());
        }
        KeyCode::Char('6') => {
            app.jump_to(5);
            return Ok(());
        }
        KeyCode::Char('7') => {
            app.jump_to(6);
            return Ok(());
        }
        KeyCode::Char('8') => {
            app.jump_to(7);
            return Ok(());
        }
        KeyCode::Char('9') => {
            app.jump_to(8);
            return Ok(());
        }
        _ => {}
    }

    if app.preview_is_focused() {
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
        return Ok(());
    }

    if is_space && !app.sidebar.show_tree {
        if app.pending_sidebar_space_action_is_active() {
            app.clear_pending_sidebar_space_action();
            let _ = app.toggle_all_sidebar_folders();
        } else {
            let _ = app.queue_pending_sidebar_space_action(DOUBLE_SPACE_WINDOW);
        }
        return Ok(());
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
                if app
                    .selected_preview_thread()
                    .is_some_and(|thread| thread.is_live())
                {
                    attach_fn(terminal, app)?;
                } else {
                    app.invalidate_preview();
                    app.dirty = true;
                }
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

fn handle_preview_tab(app: &mut App) {
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
