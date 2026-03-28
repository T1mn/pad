use crate::app::App;
use crossterm::event::KeyEvent;

pub(crate) fn handle_fuzzy_picker_mode(app: &mut App, key: KeyEvent) {
    if let Some(ref mut picker) = app.fuzzy_picker {
        match picker.handle_input(key) {
            None => {
                app.dirty = true;
            }
            Some(None) => {
                app.close_fuzzy_picker();
            }
            Some(Some(path)) => {
                app.fuzzy_picker = None;
                app.open_agent_launcher(std::path::PathBuf::from(path));
            }
        }
    }
}
