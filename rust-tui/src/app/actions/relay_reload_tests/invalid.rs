use super::support::{current_codex_provider, poll_external_relay_config, seed_codex_provider, with_temp_home};

#[test]
fn invalid_external_relay_config_is_ignored() {
    with_temp_home("invalid", || {
        seed_codex_provider("a", "https://a.example/v1", "sk-a");

        let mut app = App::new();
        std::fs::write(Config::config_path(), "[[agents]\n").expect("write invalid config");

        poll_external_relay_config(&mut app);

        let codex = current_codex_provider(&app);
        assert_eq!(codex.label, "a");
        assert!(!app.pending_external_relay_reload);
    });
}
