mod models;
mod picker;

use crate::app::state::RelayPopupMode;
use crate::app::App;
use crossterm::event::KeyCode;

pub(super) fn handle_relay_popup_key(app: &mut App, key: KeyCode) -> bool {
    if app.relay_popup_editing {
        return models::handle_relay_popup_edit(app, key);
    }

    match app.relay_popup_mode {
        RelayPopupMode::OpenCodeModels => models::handle_opencode_models_popup(app, key),
        RelayPopupMode::OpenCodeDefaultModel | RelayPopupMode::OpenCodeSmallModel => {
            picker::handle_opencode_model_picker_popup(app, key)
        }
        RelayPopupMode::None => false,
    }
}

pub(super) fn selected_model_picker_index(app: &App, include_none: bool) -> usize {
    picker::selected_model_picker_index(app, include_none)
}
