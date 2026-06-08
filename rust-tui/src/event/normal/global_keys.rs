use crate::app::App;
use crossterm::event::KeyEvent;

mod jump;
mod opencode;
mod primary;
mod special;

pub(super) fn handle_global_key(app: &mut App, key: KeyEvent) -> bool {
    special::handle_special_key(app, key)
        || primary::handle_primary_key(app, key)
        || opencode::handle_opencode_key(app, key)
        || jump::handle_numeric_jump(app, key)
}
