use crate::app::App;

use super::super::bindings::{current_root_binding, restore_binding_cmd, PAD_SIDER_TOGGLE_KEYS};

pub(in crate::session) fn save_current_return_bindings(app: &mut App) {
    app.saved_tmux_bindings.clear();
    if let Some(line) = current_root_binding("F12") {
        app.saved_tmux_bindings.push(line);
    }
    if let Some(line) = current_root_binding("C-q") {
        app.saved_tmux_bindings.push(line);
    }
    for key in PAD_SIDER_TOGGLE_KEYS {
        if let Some(line) = current_root_binding(key) {
            app.saved_tmux_bindings.push(line);
        }
    }
}

pub(super) fn saved_binding_restore_cmd(app: &App, key: &str) -> String {
    restore_binding_cmd(
        app.saved_tmux_bindings
            .iter()
            .find(|line| line.contains(&format!(" {} ", key)))
            .map(String::as_str),
        key,
    )
}
