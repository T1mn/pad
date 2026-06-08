#[test]
fn runtime_configs_apply_codex_status_line_without_relay_provider() {
    with_temp_home("codex-status-line", |_home| {
        let config_path = crate::paths::pad_codex_config_path();
        std::fs::create_dir_all(config_path.parent().expect("codex config parent"))
            .expect("create codex config parent");
        std::fs::write(&config_path, "model = \"gpt-5\"\n").expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
        };

        let mut codex = sample_codex_config();
        codex.status_line_model_with_reasoning = true;
        codex.status_line_fast_mode = true;
        codex.status_line_five_hour_limit = true;
        codex.status_line_weekly_limit = true;
        codex.status_line_context_remaining = true;
        codex.status_line_current_dir = true;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("[tui]"));
        assert!(value.contains(
            "status_line = [\"model-with-reasoning\", \"fast-mode\", \"five-hour-limit\", \"weekly-limit\", \"context-remaining\", \"current-dir\"]"
        ));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_apply_partial_codex_status_line_without_relay_provider() {
    with_temp_home("codex-status-line-partial", |_home| {
        let config_path = crate::paths::pad_codex_config_path();
        std::fs::create_dir_all(config_path.parent().expect("codex config parent"))
            .expect("create codex config parent");
        std::fs::write(&config_path, "model = \"gpt-5\"\n").expect("seed codex config");

        let agent = AgentConfig {
            name: "codex".into(),
            cmd: "codex".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
        };

        let mut codex = sample_codex_config();
        codex.status_line_five_hour_limit = true;
        codex.status_line_weekly_limit = true;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("status_line = [\"five-hour-limit\", \"weekly-limit\"]"));
    });
}

#[test]
fn runtime_configs_restore_previous_codex_status_line_when_disabled() {
    with_temp_home("codex-status-line-restore", |_home| {
        let config_path = crate::paths::pad_codex_config_path();
        std::fs::create_dir_all(config_path.parent().expect("codex config parent"))
            .expect("create codex config parent");
        std::fs::write(
            &config_path,
            "model = \"gpt-5\"\n[tui]\nstatus_line = [\"project\", \"git-branch\"]\n",
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
        codex.status_line_model_with_reasoning = true;
        codex.status_line_fast_mode = true;
        codex.status_line_five_hour_limit = true;
        codex.status_line_weekly_limit = true;
        codex.status_line_context_remaining = true;
        codex.status_line_current_dir = true;
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        codex.status_line_model_with_reasoning = false;
        codex.status_line_fast_mode = false;
        codex.status_line_five_hour_limit = false;
        codex.status_line_weekly_limit = false;
        codex.status_line_context_remaining = false;
        codex.status_line_current_dir = false;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("status_line = [\"project\", \"git-branch\"]"));
    });
}
