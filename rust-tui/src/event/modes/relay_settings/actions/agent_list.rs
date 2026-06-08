use super::super::{exit_relay, RelayHost};
use crate::app::state::RelayView;
use crate::app::App;
use crossterm::event::KeyCode;

pub(in crate::event::modes::relay_settings) fn handle_agent_list_key(
    app: &mut App,
    key: KeyCode,
    host: RelayHost,
) -> bool {
    match key {
        KeyCode::Esc => {
            exit_relay(app, host);
            app.dirty = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.config.agents.len().saturating_sub(1);
            if app.relay_selected_agent < max {
                app.relay_selected_agent += 1;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.relay_selected_agent > 0 {
                app.relay_selected_agent -= 1;
            }
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            app.relay_view = RelayView::ProviderList;
            let active = app
                .config
                .agents
                .get(app.relay_selected_agent)
                .and_then(|agent| agent.active_provider)
                .unwrap_or(0);
            app.relay_selected_provider = active;
            app.dirty = true;
        }
        _ => {}
    }
    true
}
