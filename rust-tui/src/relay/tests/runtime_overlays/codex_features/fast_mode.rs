use super::support::{codex_agent_without_relay_provider, seed_pad_codex_config};

#[test]
fn runtime_configs_apply_codex_fast_mode_without_relay_provider() {
    with_temp_home("codex-fast-mode", |_home| {
        let config_path = seed_pad_codex_config("model = \"gpt-5\"\n");
        let agent = codex_agent_without_relay_provider();

        let mut codex = sample_codex_config();
        codex.fast_mode = true;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("service_tier = \"fast\""));
        assert!(value.contains("fast_mode = true"));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_previous_codex_fast_fields_when_disabled() {
    with_temp_home("codex-fast-mode-restore", |_home| {
        let config_path = seed_pad_codex_config(
            "model = \"gpt-5\"\nservice_tier = \"default\"\n[features]\nfast_mode = false\n",
        );
        let agent = codex_agent_without_relay_provider();

        let mut codex = sample_codex_config();
        codex.fast_mode = true;
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        codex.fast_mode = false;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("service_tier = \"default\""));
        assert!(value.contains("fast_mode = false"));
    });
}
