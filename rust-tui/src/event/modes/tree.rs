use crate::app::state::Mode;
use crate::app::App;
use crate::log_debug;
use crossterm::event::KeyCode;

pub(crate) fn handle_tree_mode(app: &mut App, key: KeyCode) {
    if let Some(ref mut tree) = app.sidebar.file_tree {
        log_debug!(
            "tree_mode key={:?} path={} selected={:?}",
            key,
            tree.current_path.display(),
            tree.state.selected()
        );
        match key {
            KeyCode::Esc => {
                app.close_tree();
                app.dirty = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                tree.next();
                app.update_file_preview();
                app.dirty = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                tree.previous();
                app.update_file_preview();
                app.dirty = true;
            }
            KeyCode::Char(' ') => {
                tree.toggle();
                app.update_file_preview();
                app.dirty = true;
            }
            KeyCode::Enter => {
                let entry_name = tree.selected().map(|e| e.name.clone()).unwrap_or_default();
                log_debug!("tree_mode enter: entry={}", entry_name);
                let selected_is_dir = tree.selected().map(|e| e.is_dir).unwrap_or(false);
                if selected_is_dir {
                    tree.enter();
                    app.update_file_preview();
                } else {
                    app.mode = Mode::FilePreview;
                    app.preview.file_preview_scroll = 0;
                }
                app.dirty = true;
            }
            KeyCode::Backspace => {
                tree.go_up();
                app.update_file_preview();
                app.dirty = true;
            }
            KeyCode::Char('/') => {
                app.mode = Mode::TreeSearch;
                tree.start_search();
                app.dirty = true;
            }
            KeyCode::Char('c') => {
                let target_path = tree.selected().filter(|e| e.is_dir).map(|e| e.path.clone());
                if let Some(path) = target_path {
                    log_debug!("tree_mode: open agent launcher at {}", path.display());
                    app.open_agent_launcher(path);
                }
            }
            KeyCode::Char('T') => {
                app.open_tree_in_home();
            }
            KeyCode::Char('t') => {
                app.toggle_tree();
            }
            KeyCode::Char('J') => {
                app.preview.file_preview_scroll = app.preview.file_preview_scroll.saturating_add(3);
                app.dirty = true;
            }
            KeyCode::Char('K') => {
                app.preview.file_preview_scroll = app.preview.file_preview_scroll.saturating_sub(3);
                app.dirty = true;
            }
            KeyCode::PageDown => {
                app.preview.file_preview_scroll =
                    app.preview.file_preview_scroll.saturating_add(10);
                app.dirty = true;
            }
            KeyCode::PageUp => {
                app.preview.file_preview_scroll =
                    app.preview.file_preview_scroll.saturating_sub(10);
                app.dirty = true;
            }
            _ => {}
        }
    }
}
