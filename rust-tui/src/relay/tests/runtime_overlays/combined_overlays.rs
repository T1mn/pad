#[test]
fn runtime_configs_apply_combined_codex_overlays_together() {
    with_temp_home("codex-combined-overlays", |_home| {
        let config_path = crate::paths::pad_codex_config_path();
        std::fs::create_dir_all(config_path.parent().expect("codex config parent"))
            .expect("create codex config parent");
        std::fs::write(
            &config_path,
            "model = \"gpt-5\"\napproval_policy = \"on-request\"\nservice_tier = \"default\"\nweb_search = \"cached\"\n[features]\nfast_mode = false\nmulti_agent = false\n",
        )
        .expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
        };

        let mut codex = sample_codex_config();
        codex.fast_mode = true;
        codex.multi_agent = true;
        codex.web_search = "live".into();
        codex.status_line_model_with_reasoning = true;
        codex.status_line_fast_mode = true;
        codex.status_line_five_hour_limit = true;
        codex.status_line_weekly_limit = true;
        codex.status_line_context_remaining = true;
        codex.status_line_current_dir = true;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("approval_policy = \"never\""));
        assert!(value.contains("sandbox_mode = \"danger-full-access\""));
        assert!(value.contains("service_tier = \"fast\""));
        assert!(value.contains("web_search = \"live\""));
        assert!(value.contains("fast_mode = true"));
        assert!(value.contains("multi_agent = true"));
        assert!(value.contains(
            "status_line = [\"model-with-reasoning\", \"fast-mode\", \"five-hour-limit\", \"weekly-limit\", \"context-remaining\", \"current-dir\"]"
        ));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_combined_codex_overlays_to_original_values() {
    with_temp_home("codex-combined-restore", |_home| {
        let config_path = crate::paths::pad_codex_config_path();
        std::fs::create_dir_all(config_path.parent().expect("codex config parent"))
            .expect("create codex config parent");
        std::fs::write(
            &config_path,
            "model = \"gpt-5\"\napproval_policy = \"on-request\"\nservice_tier = \"default\"\nweb_search = \"cached\"\n[features]\nfast_mode = false\nmulti_agent = false\n",
        )
        .expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
        };

        let mut codex = sample_codex_config();
        codex.fast_mode = true;
        codex.multi_agent = true;
        codex.web_search = "live".into();
        codex.status_line_model_with_reasoning = true;
        codex.status_line_fast_mode = true;
        codex.status_line_five_hour_limit = true;
        codex.status_line_weekly_limit = true;
        codex.status_line_context_remaining = true;
        codex.status_line_current_dir = true;
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        let disabled = AgentPermissionsConfig {
            codex_auto_full_access: false,
            claude_auto_full_access: true,
        };
        codex.fast_mode = false;
        codex.multi_agent = false;
        codex.web_search = "default".into();
        codex.status_line_model_with_reasoning = false;
        codex.status_line_fast_mode = false;
        codex.status_line_five_hour_limit = false;
        codex.status_line_weekly_limit = false;
        codex.status_line_context_remaining = false;
        codex.status_line_current_dir = false;
        apply_runtime_configs(&[agent], &disabled, &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("approval_policy = \"on-request\""));
        assert!(!value.contains("sandbox_mode = \"danger-full-access\""));
        assert!(value.contains("service_tier = \"default\""));
        assert!(value.contains("web_search = \"cached\""));
        assert!(value.contains("fast_mode = false"));
        assert!(value.contains("multi_agent = false"));
        assert!(!value.contains(
            "status_line = [\"model-with-reasoning\", \"fast-mode\", \"five-hour-limit\", \"weekly-limit\", \"context-remaining\", \"current-dir\"]"
        ));
        assert!(!codex_permission_state_path().exists());
    });
}
