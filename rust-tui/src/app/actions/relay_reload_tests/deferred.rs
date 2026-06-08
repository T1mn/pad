use super::support::{current_codex_provider, poll_external_relay_config, seed_codex_provider, update_codex_provider, with_temp_home};

#[test]
fn external_relay_reload_is_deferred_while_editing() {
    with_temp_home("deferred", || {
        seed_codex_provider("a", "https://a.example/v1", "sk-a");

        let mut app = App::new();
        app.relay_editing = true;

        update_codex_provider("b", "https://b.example/v1", "sk-b");
        poll_external_relay_config(&mut app);

        let codex = current_codex_provider(&app);
        assert_eq!(codex.label, "a");
        assert!(app.pending_external_relay_reload);
        assert_eq!(
            app.preview
                .copy_toast
                .as_ref()
                .map(|toast| toast.title.as_str()),
            Some("Relay reload deferred")
        );

        app.relay_editing = false;
        app.apply_pending_external_relay_reload_if_ready();

        let codex = current_codex_provider(&app);
        assert_eq!(codex.label, "b");
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
