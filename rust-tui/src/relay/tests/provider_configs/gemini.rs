#[test]
fn gemini_provider_writes_env_and_preserves_settings_json() {
    with_temp_home("gemini-write", |home| {
        let gemini_dir = home.join(".gemini");
        std::fs::create_dir_all(&gemini_dir).expect("create gemini dir");
        let settings_path = gemini_dir.join("settings.json");
        let env_path = gemini_dir.join(".env");
        std::fs::write(
            &settings_path,
            r#"{"mcpServers":{"echo":{"command":"echo"}},"apiUrl":"old","apiKey":"old"}"#,
        )
        .expect("seed gemini settings");
        std::fs::write(&env_path, "KEEP_ME=1\n").expect("seed gemini env");

        let agent = AgentConfig {
            name: "gemini".into(),
            cmd: "gemini".into(),
            providers: vec![sample_provider("https://gemini-relay.example", "gm-test")],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
        };
        apply_relay_configs(&[agent]);

        let env = parse_env_file(&std::fs::read_to_string(&env_path).expect("read env"));
        assert_eq!(
            env.get("GOOGLE_GEMINI_BASE_URL").map(String::as_str),
            Some("https://gemini-relay.example")
        );
        assert_eq!(
            env.get("GEMINI_API_KEY").map(String::as_str),
            Some("gm-test")
        );
        assert_eq!(env.get("KEEP_ME").map(String::as_str), Some("1"));

        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings_path).expect("read"))
                .expect("parse");
        assert_eq!(
            value
                .pointer("/security/auth/selectedType")
                .and_then(|v| v.as_str()),
            Some("apiKey")
        );
        assert!(value.get("mcpServers").is_some());
        assert!(value.get("apiUrl").is_none());
        assert!(value.get("apiKey").is_none());
    });
}

#[test]
fn incomplete_gemini_provider_restores_original_files() {
    with_temp_home("gemini-restore", |home| {
        let gemini_dir = home.join(".gemini");
        std::fs::create_dir_all(&gemini_dir).expect("create gemini dir");
        let settings_path = gemini_dir.join("settings.json");
        let env_path = gemini_dir.join(".env");
        std::fs::write(
            &settings_path,
            r#"{"mcpServers":{"echo":{"command":"echo"}}}"#,
        )
        .expect("seed settings");
        std::fs::write(&env_path, "KEEP_ME=1\n").expect("seed env");

        let complete = AgentConfig {
            name: "gemini".into(),
            cmd: "gemini".into(),
            providers: vec![sample_provider("https://gemini-relay.example", "gm-test")],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
        };
        apply_relay_configs(&[complete]);

        let incomplete = AgentConfig {
            name: "gemini".into(),
            cmd: "gemini".into(),
            providers: vec![sample_provider("https://gemini-relay.example", "")],
            active_provider: Some(0),
            default_model: String::new(),
            small_model: String::new(),
        };
        apply_relay_configs(&[incomplete]);

        assert_eq!(
            std::fs::read_to_string(&env_path).expect("read restored env"),
            "KEEP_ME=1\n"
        );
        let restored: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings_path).expect("read"))
                .expect("parse");
        assert!(restored.pointer("/mcpServers/echo").is_some());
        assert!(restored.pointer("/security/auth").is_none());
    });
}
