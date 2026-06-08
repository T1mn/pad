use super::support::{current_codex_provider, poll_external_relay_config, seed_codex_provider, update_codex_provider, with_temp_home};

#[test]
fn external_relay_reload_applies_immediately_when_not_editing() {
    with_temp_home("immediate", || {
        seed_codex_provider("a", "https://a.example/v1", "sk-a");

        let mut app = App::new();
        assert_eq!(app.config.agents[1].providers[0].label, "a");

        update_codex_provider("b", "https://b.example/v1", "sk-b");
        poll_external_relay_config(&mut app);

        let codex = current_codex_provider(&app);
        assert_eq!(codex.label, "b");
        assert_eq!(codex.base_url, "https://b.example/v1");
        assert_eq!(codex.api_key, "sk-b");
        assert!(!app.pending_external_relay_reload);
        assert_eq!(
            app.preview
                .copy_toast
                .as_ref()
                .map(|toast| toast.title.as_str()),
            Some("Relay reloaded")
        );
    });
}
