use crate::app::state::Mode;
use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{backend::Backend, Terminal};
use std::io;
use std::time::Duration;

pub(super) fn handle_sidebar_key<B: Backend, F>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    key: KeyEvent,
    is_space: bool,
    attach_fn: &mut F,
) -> io::Result<()>
where
    F: FnMut(&mut Terminal<B>, &mut App) -> io::Result<()>,
{
    const DOUBLE_SPACE_WINDOW: Duration = Duration::from_millis(250);

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
