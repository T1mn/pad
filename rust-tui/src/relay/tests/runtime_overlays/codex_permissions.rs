#[test]
fn runtime_configs_apply_codex_full_access_without_relay_provider() {
    with_temp_home("codex-permissions", |_home| {
        let codex_dir = crate::paths::pad_codex_home_dir();
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(
            &config_path,
            "model = \"gpt-5\"\napproval_policy = \"on-request\"\n",
        )
        .expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        apply_runtime_configs(&[agent], &sample_permissions(), &sample_codex_config());

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("approval_policy = \"never\""));
        assert!(value.contains("sandbox_mode = \"danger-full-access\""));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_previous_codex_permission_fields_when_disabled() {
    with_temp_home("codex-permissions-restore", |_home| {
        let codex_dir = crate::paths::pad_codex_home_dir();
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(
            &config_path,
            "model = \"gpt-5\"\napproval_policy = \"on-request\"\n",
        )
        .expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        apply_runtime_configs(
            std::slice::from_ref(&agent),
            &sample_permissions(),
            &sample_codex_config(),
        );

        let disabled = AgentPermissionsConfig {
            codex_auto_full_access: false,
            claude_auto_full_access: false,
        };
        apply_runtime_configs(&[agent], &disabled, &sample_codex_config());

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("approval_policy = \"on-request\""));
        assert!(!value.contains("sandbox_mode = \"danger-full-access\""));
        assert!(!codex_permission_state_path().exists());
    });
}

