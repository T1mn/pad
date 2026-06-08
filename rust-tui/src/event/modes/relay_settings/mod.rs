mod actions;
mod edit;
mod popup;
mod transfer;

use crate::app::state::{Mode, RelayPopupMode, RelayView};
use crate::app::App;
use crossterm::event::KeyCode;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelayHost {
    Standalone,
    Settings,
}

pub(crate) fn handle_relay_settings_mode(app: &mut App, key: KeyCode) {
    let _ = handle_relay_key(app, key, RelayHost::Standalone);
}

pub(crate) fn handle_relay_key(app: &mut App, key: KeyCode, host: RelayHost) -> bool {
    if app.relay_popup_mode != RelayPopupMode::None {
        return popup::handle_relay_popup_key(app, key);
    }

    if app.relay_editing {
        return edit::handle_relay_field_edit(app, key);
    }

    match app.relay_view {
        RelayView::AgentList => actions::handle_agent_list_key(app, key, host),
        RelayView::ProviderList => actions::handle_provider_list_key(app, key, host),
        RelayView::DetailPane => actions::handle_detail_pane_key(app, key, host),
    }
}

pub(super) fn relay_field_count(app: &App) -> usize {
    match selected_agent_name(app) {
        Some("opencode") => 6,
        _ => 3,
    }
}

pub(super) fn exit_relay(app: &mut App, host: RelayHost) {
    app.relay_editing = false;
    app.relay_edit_buffer.clear();
    app.clear_relay_popup_state();
    match host {
        RelayHost::Standalone => app.mode = Mode::Settings,
        RelayHost::Settings => app.leave_settings_detail(),
    }
}

pub(super) fn selected_agent_name(app: &App) -> Option<&str> {
    app.config
        .agents
        .get(app.relay_selected_agent)
        .map(|agent| agent.name.as_str())
}

#[cfg(test)]
mod tests;
