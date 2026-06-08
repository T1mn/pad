use super::super::{handle_relay_key, RelayHost};
use super::support::{agent_index, sample_provider, with_temp_home};
use crate::app::state::RelayView;
use crate::app::App;
use crate::theme::Config;
use crossterm::event::KeyCode;

#[test]
fn provider_toggle_updates_active_provider_and_persists_overlay() {
    with_temp_home("relay-active-provider", || {
        let mut app = App::new();
        let codex_idx = agent_index(&app, "codex");
        app.config.agents[codex_idx]
            .providers
            .push(sample_provider("relay-primary"));
        app.relay_selected_agent = codex_idx;
        app.relay_selected_provider = 0;
        app.relay_view = RelayView::ProviderList;

        handle_relay_key(&mut app, KeyCode::Char(' '), RelayHost::Standalone);
        assert_eq!(app.config.agents[codex_idx].active_provider, Some(0));

        let saved = Config::load();
        let saved_codex = saved
            .agents
            .iter()
            .find(|agent| agent.name == "codex")
            .expect("saved codex");
        assert_eq!(saved_codex.active_provider, Some(0));

        handle_relay_key(&mut app, KeyCode::Char(' '), RelayHost::Standalone);
        assert_eq!(app.config.agents[codex_idx].active_provider, None);

        let saved = Config::load();
        let saved_codex = saved
            .agents
            .iter()
            .find(|agent| agent.name == "codex")
            .expect("saved codex");
        assert_eq!(saved_codex.active_provider, None);
    });
}
