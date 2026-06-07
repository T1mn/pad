use super::*;
use crate::theme::ProviderConfig;
use std::time::{SystemTime, UNIX_EPOCH};

fn with_temp_home<T>(name: &str, f: impl FnOnce() -> T) -> T {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("lock relay reload tests");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let home = std::env::temp_dir().join(format!("pad-relay-reload-{name}-{stamp}"));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create temp home");

    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    let result = f();

    if let Some(prev) = prev_home {
        std::env::set_var("HOME", prev);
    } else {
        std::env::remove_var("HOME");
    }
    let _ = std::fs::remove_dir_all(&home);
    result
}

fn sample_provider(label: &str, base_url: &str, api_key: &str) -> ProviderConfig {
    ProviderConfig {
        label: label.to_string(),
        base_url: base_url.to_string(),
        api_key: api_key.to_string(),
        env_key: String::new(),
        wire_api: "responses".to_string(),
        provider_key: crate::theme::normalize_provider_key(label),
        npm_package: "@ai-sdk/openai-compatible".to_string(),
        models: Vec::new(),
        test_status: Some(true),
        test_http_status: Some(200),
        test_latency_ms: Some(12),
        test_result: Some("ok".to_string()),
    }
}

#[test]
fn external_relay_reload_applies_immediately_when_not_editing() {
    with_temp_home("immediate", || {
        let mut config = Config::default();
        let codex = config
            .agents
            .iter_mut()
            .find(|agent| agent.name == "codex")
            .expect("codex agent");
        codex.providers = vec![sample_provider("a", "https://a.example/v1", "sk-a")];
        codex.active_provider = Some(0);
        config.save();

        let mut app = App::new();
        assert_eq!(app.config.agents[1].providers[0].label, "a");

        let mut updated = Config::load();
        let codex = updated
            .agents
            .iter_mut()
            .find(|agent| agent.name == "codex")
            .expect("updated codex agent");
        codex.providers = vec![sample_provider("b", "https://b.example/v1", "sk-b")];
        codex.active_provider = Some(0);
        updated.save();

        app.relay_config_last_poll_at -= RELAY_CONFIG_POLL_INTERVAL;
        app.poll_external_relay_config_if_due();

        let codex = app
            .config
            .agents
            .iter()
            .find(|agent| agent.name == "codex")
            .expect("reloaded codex agent");
        assert_eq!(codex.providers[0].label, "b");
        assert_eq!(codex.providers[0].base_url, "https://b.example/v1");
        assert_eq!(codex.providers[0].api_key, "sk-b");
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

#[test]
fn external_relay_reload_is_deferred_while_editing() {
    with_temp_home("deferred", || {
        let mut config = Config::default();
        let codex = config
            .agents
            .iter_mut()
            .find(|agent| agent.name == "codex")
            .expect("codex agent");
        codex.providers = vec![sample_provider("a", "https://a.example/v1", "sk-a")];
        codex.active_provider = Some(0);
        config.save();

        let mut app = App::new();
        app.relay_editing = true;

        let mut updated = Config::load();
        let codex = updated
            .agents
            .iter_mut()
            .find(|agent| agent.name == "codex")
            .expect("updated codex agent");
        codex.providers = vec![sample_provider("b", "https://b.example/v1", "sk-b")];
        codex.active_provider = Some(0);
        updated.save();

        app.relay_config_last_poll_at -= RELAY_CONFIG_POLL_INTERVAL;
        app.poll_external_relay_config_if_due();

        let codex = app
            .config
            .agents
            .iter()
            .find(|agent| agent.name == "codex")
            .expect("current codex agent");
        assert_eq!(codex.providers[0].label, "a");
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

        let codex = app
            .config
            .agents
            .iter()
            .find(|agent| agent.name == "codex")
            .expect("reloaded codex agent");
        assert_eq!(codex.providers[0].label, "b");
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

#[test]
fn invalid_external_relay_config_is_ignored() {
    with_temp_home("invalid", || {
        let mut config = Config::default();
        let codex = config
            .agents
            .iter_mut()
            .find(|agent| agent.name == "codex")
            .expect("codex agent");
        codex.providers = vec![sample_provider("a", "https://a.example/v1", "sk-a")];
        codex.active_provider = Some(0);
        config.save();

        let mut app = App::new();
        std::fs::write(Config::config_path(), "[[agents]\n").expect("write invalid config");

        app.relay_config_last_poll_at -= RELAY_CONFIG_POLL_INTERVAL;
        app.poll_external_relay_config_if_due();

        let codex = app
            .config
            .agents
            .iter()
            .find(|agent| agent.name == "codex")
            .expect("current codex agent");
        assert_eq!(codex.providers[0].label, "a");
        assert!(!app.pending_external_relay_reload);
    });
}

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

        app.relay_config_last_poll_at -= RELAY_CONFIG_POLL_INTERVAL;
        app.poll_external_relay_config_if_due();

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
