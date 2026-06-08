use super::support::{codex_agent_without_relay_provider, seed_pad_codex_config};

#[test]
fn runtime_configs_apply_codex_web_search_without_relay_provider() {
    with_temp_home("codex-web-search", |_home| {
        let config_path = seed_pad_codex_config("model = \"gpt-5\"\n");
        let agent = codex_agent_without_relay_provider();

        let mut codex = sample_codex_config();
        codex.web_search = "live".into();
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("web_search = \"live\""));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_previous_codex_web_search_when_defaulted() {
    with_temp_home("codex-web-search-restore", |_home| {
        let config_path = seed_pad_codex_config("model = \"gpt-5\"\nweb_search = \"cached\"\n");
        let agent = codex_agent_without_relay_provider();

        let mut codex = sample_codex_config();
        codex.web_search = "live".into();
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        codex.web_search = "default".into();
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("web_search = \"cached\""));
    });
}
