use super::support::{codex_agent_without_relay_provider, seed_pad_codex_config};

#[test]
fn runtime_configs_apply_codex_goals_without_relay_provider() {
    with_temp_home("codex-goals", |_home| {
        let config_path = seed_pad_codex_config("model = \"gpt-5\"\n");
        let agent = codex_agent_without_relay_provider();

        let mut codex = sample_codex_config();
        codex.goals = true;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("goals = true"));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_previous_codex_goals_when_disabled() {
    with_temp_home("codex-goals-restore", |_home| {
        let config_path = seed_pad_codex_config("model = \"gpt-5\"\n[features]\ngoals = false\n");
        let agent = codex_agent_without_relay_provider();

        let mut codex = sample_codex_config();
        codex.goals = true;
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        codex.goals = false;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("goals = false"));
    });
}
