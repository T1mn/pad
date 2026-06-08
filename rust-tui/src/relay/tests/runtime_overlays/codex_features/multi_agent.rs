use super::support::{codex_agent_without_relay_provider, seed_pad_codex_config};

#[test]
fn runtime_configs_apply_codex_multi_agent_without_relay_provider() {
    with_temp_home("codex-multi-agent", |_home| {
        let config_path = seed_pad_codex_config("model = \"gpt-5\"\n");
        let agent = codex_agent_without_relay_provider();

        let mut codex = sample_codex_config();
        codex.multi_agent = true;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("multi_agent = true"));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_previous_codex_multi_agent_when_disabled() {
    with_temp_home("codex-multi-agent-restore", |_home| {
        let config_path = seed_pad_codex_config(
            "model = \"gpt-5\"\n[features]\nmulti_agent = false\n",
        );
        let agent = codex_agent_without_relay_provider();

        let mut codex = sample_codex_config();
        codex.multi_agent = true;
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        codex.multi_agent = false;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("multi_agent = false"));
    });
}
