use crate::app::state::Mode;
use crate::app::App;
use crossterm::event::KeyCode;

pub(crate) fn handle_tree_search_mode(app: &mut App, key: KeyCode) {
    if let Some(ref mut tree) = app.sidebar.file_tree {
        match key {
            KeyCode::Esc => {
                tree.cancel_search();
                app.mode = Mode::Tree;
                app.update_file_preview();
                app.dirty = true;
            }
            KeyCode::Enter => {
                tree.cancel_search();
                app.mode = Mode::Tree;
                app.update_file_preview();
                app.dirty = true;
            }
            KeyCode::Char(c) => {
                tree.search_input(c);
                app.update_file_preview();
                app.dirty = true;
            }
            KeyCode::Backspace => {
                tree.search_backspace();
                app.update_file_preview();
                app.dirty = true;
            }
            _ => {}
        }
    }
}
