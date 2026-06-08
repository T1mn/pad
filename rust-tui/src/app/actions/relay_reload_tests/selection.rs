use super::support::{poll_external_relay_config, sample_provider, with_temp_home};

#[test]
fn external_relay_reload_clamps_provider_selection() {
    with_temp_home("clamp", || {
        let mut config = Config::default();
        let codex = config
            .agents
            .iter_mut()
            .find(|agent| agent.name == "codex")
            .expect("codex agent");
        codex.providers = vec![
            sample_provider("a", "https://a.example/v1", "sk-a"),
            sample_provider("b", "https://b.example/v1", "sk-b"),
        ];
        codex.active_provider = Some(1);
        config.save();

        let mut app = App::new();
        app.relay_selected_agent = app
            .config
            .agents
            .iter()
            .position(|agent| agent.name == "codex")
            .expect("codex index");
        app.relay_selected_provider = 1;
        app.relay_view = RelayView::DetailPane;
        app.relay_edit_field = 2;

        let mut updated = Config::load();
        let codex = updated
            .agents
            .iter_mut()
            .find(|agent| agent.name == "codex")
            .expect("updated codex agent");
        codex.providers = vec![sample_provider(
            "only",
            "https://only.example/v1",
            "sk-only",
        )];
        codex.active_provider = Some(0);
        updated.save();

        poll_external_relay_config(&mut app);

        assert_eq!(app.relay_selected_provider, 0);
        assert!(matches!(app.relay_view, RelayView::DetailPane));
        assert_eq!(app.relay_edit_field, 2);

        let codex = app
            .config
            .agents
            .iter()
            .find(|agent| agent.name == "codex")
            .expect("reloaded codex agent");
        assert_eq!(codex.providers.len(), 1);
        assert_eq!(codex.providers[0].label, "only");
    });
}
