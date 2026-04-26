#[test]
fn runtime_configs_apply_claude_full_access_without_relay_provider() {
    with_temp_home("claude-permissions", |home| {
        let claude_dir = home.join(".claude");
        std::fs::create_dir_all(&claude_dir).expect("create claude dir");
        let settings_path = claude_dir.join("settings.json");
        std::fs::write(
            &settings_path,
            r#"{"permissions":{"defaultMode":"ask"},"sandbox":{"enabled":true},"mcpServers":{"echo":{"command":"echo"}}}"#,
        )
        .expect("seed claude settings");

        let agent = AgentConfig {
            name: "claude".into(),
            cmd: "claude".into(),
            providers: Vec::new(),
            active_provider: None,
            default_model: String::new(),
            small_model: String::new(),
            base_url: None,
            api_key: None,
        };

        apply_runtime_configs(&[agent], &sample_permissions(), &sample_codex_config());

        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings_path).expect("read"))
                .expect("parse");
        assert_eq!(
            value
                .pointer("/permissions/defaultMode")
                .and_then(|v| v.as_str()),
            Some("bypassPermissions")
        );
        assert_eq!(
            value.pointer("/sandbox/enabled").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert!(value.pointer("/mcpServers/echo").is_some());
        assert!(claude_permission_state_path().exists());
    });
}

#[test]
fn runtime_configs_restore_previous_claude_permission_fields_when_disabled() {
    with_temp_home("claude-permissions-restore", |home| {
        let claude_dir = home.join(".claude");
        std::fs::create_dir_all(&claude_dir).expect("create claude dir");
        let settings_path = claude_dir.join("settings.json");
        std::fs::write(
            &settings_path,
            r#"{"permissions":{"defaultMode":"ask"},"sandbox":{"enabled":true},"env":{"KEEP":"1"}}"#,
        )
        .expect("seed claude settings");

        let agent = AgentConfig {
            name: "claude".into(),
            cmd: "claude".into(),
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

        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings_path).expect("read"))
                .expect("parse");
        assert_eq!(
            value
                .pointer("/permissions/defaultMode")
                .and_then(|v| v.as_str()),
            Some("ask")
        );
        assert_eq!(
            value.pointer("/sandbox/enabled").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            value.pointer("/env/KEEP").and_then(|v| v.as_str()),
            Some("1")
        );
        assert!(!claude_permission_state_path().exists());
    });
}
