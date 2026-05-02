#[test]
fn runtime_configs_apply_codex_fast_mode_without_relay_provider() {
    with_temp_home("codex-fast-mode", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
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
    with_temp_home("codex-fast-mode-restore", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(
            &config_path,
            "model = \"gpt-5\"\nservice_tier = \"default\"\n[features]\nfast_mode = false\n",
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
        codex.fast_mode = true;
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        codex.fast_mode = false;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("service_tier = \"default\""));
        assert!(value.contains("fast_mode = false"));
    });
}

#[test]
fn runtime_configs_apply_codex_goals_without_relay_provider() {
    with_temp_home("codex-goals", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
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
        codex.goals = true;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("goals = true"));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_previous_codex_goals_when_disabled() {
    with_temp_home("codex-goals-restore", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(&config_path, "model = \"gpt-5\"\n[features]\ngoals = false\n")
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
        codex.goals = true;
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        codex.goals = false;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("goals = false"));
    });
}

#[test]
fn runtime_configs_apply_codex_multi_agent_without_relay_provider() {
    with_temp_home("codex-multi-agent", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
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
        codex.multi_agent = true;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("multi_agent = true"));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_previous_codex_multi_agent_when_disabled() {
    with_temp_home("codex-multi-agent-restore", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(
            &config_path,
            "model = \"gpt-5\"\n[features]\nmulti_agent = false\n",
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
        codex.multi_agent = true;
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        codex.multi_agent = false;
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("multi_agent = false"));
    });
}

#[test]
fn runtime_configs_apply_codex_web_search_without_relay_provider() {
    with_temp_home("codex-web-search", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
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
        codex.web_search = "live".into();
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("web_search = \"live\""));
        assert!(codex_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_previous_codex_web_search_when_defaulted() {
    with_temp_home("codex-web-search-restore", |home| {
        let codex_dir = home.join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("create codex dir");
        let config_path = codex_dir.join("config.toml");
        std::fs::write(&config_path, "model = \"gpt-5\"\nweb_search = \"cached\"\n")
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
        codex.web_search = "live".into();
        apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

        codex.web_search = "default".into();
        apply_runtime_configs(&[agent], &sample_permissions(), &codex);

        let value = std::fs::read_to_string(&config_path).expect("read codex config");
        assert!(value.contains("web_search = \"cached\""));
    });
}
