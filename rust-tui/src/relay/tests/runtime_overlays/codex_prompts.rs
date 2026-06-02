#[test]
fn runtime_configs_apply_codex_jailbreak_prompt_file_without_relay_provider() {
    with_temp_home("codex-prompt-file", |_home| {
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
            base_url: None,
            api_key: None,
        };

        let mut codex = sample_codex_config();
        codex.jailbreak_prompt_file = true;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        let expected = codex_jailbreak_prompt_file_path()
            .to_string_lossy()
            .to_string();
        assert!(value.contains(&format!("model_instructions_file = \"{expected}\"")));
        assert!(codex_jailbreak_prompt_file_path().is_file());
        assert_eq!(
            std::fs::read_to_string(codex_jailbreak_prompt_file_path()).expect("read prompt file"),
            DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE
        );
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_apply_codex_index_prompt_file_without_relay_provider() {
    with_temp_home("codex-index-prompt-file", |_home| {
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
            base_url: None,
            api_key: None,
        };

        let mut codex = sample_codex_config();
        codex.index_prompt_file = true;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        let expected = codex_index_prompt_file_path().to_string_lossy().to_string();
        assert!(value.contains(&format!("model_instructions_file = \"{expected}\"")));
        assert!(codex_index_prompt_file_path().is_file());
        assert_eq!(
            std::fs::read_to_string(codex_index_prompt_file_path()).expect("read prompt file"),
            DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE
        );
    });
}

#[test]
fn runtime_configs_apply_combined_codex_prompt_candidates_without_relay_provider() {
    with_temp_home("codex-combined-prompt-file", |_home| {
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
            base_url: None,
            api_key: None,
        };

        let mut codex = sample_codex_config();
        codex.jailbreak_prompt_file = true;
        codex.index_prompt_file = true;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        let expected = codex_selected_prompt_file_path()
            .to_string_lossy()
            .to_string();
        assert!(value.contains(&format!("model_instructions_file = \"{expected}\"")));
        let combined =
            std::fs::read_to_string(codex_selected_prompt_file_path()).expect("read combined");
        assert!(combined.contains(DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE));
        assert!(combined.contains(DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE));
    });
}

#[test]
fn runtime_configs_restore_previous_codex_jailbreak_prompt_file_when_disabled() {
    with_temp_home("codex-prompt-file-restore", |_home| {
        let config_path = crate::paths::pad_codex_config_path();
        std::fs::create_dir_all(config_path.parent().expect("codex config parent"))
            .expect("create codex config parent");
        std::fs::write(
            &config_path,
            "model = \"gpt-5\"\nmodel_instructions_file = \"/tmp/original.md\"\n",
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

        let mut codex = sample_codex_config();
        codex.jailbreak_prompt_file = true;
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        codex.jailbreak_prompt_file = false;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("model_instructions_file = \"/tmp/original.md\""));
    });
}

