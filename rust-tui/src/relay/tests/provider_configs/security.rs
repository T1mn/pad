#[cfg(unix)]
fn assert_private(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mode = std::fs::metadata(path)
        .expect("stat provider file")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "provider file must be owner-only: {path:?}");
}

#[cfg(unix)]
#[test]
fn relay_provider_files_are_private() {
    with_temp_home("private-provider-files", |home| {
        let agents = [
            AgentConfig {
                name: "claude".into(),
                cmd: "claude".into(),
                providers: vec![sample_provider("https://claude.example", "claude-secret")],
                active_provider: Some(0),
                default_model: String::new(),
                small_model: String::new(),
            },
            AgentConfig {
                name: "codex".into(),
                cmd: "codex".into(),
                providers: vec![sample_provider("https://codex.example", "codex-secret")],
                active_provider: Some(0),
                default_model: String::new(),
                small_model: String::new(),
            },
        ];
        apply_relay_configs(&agents);

        for path in [
            home.join(".claude/settings.json"),
            crate::paths::pad_codex_config_path(),
            crate::paths::pad_codex_auth_path(),
            crate::paths::pad_home_dir().join("claude-settings.pre-pad.json"),
            crate::paths::pad_home_dir().join("codex-config.pre-pad.toml"),
            crate::paths::pad_home_dir().join("codex-auth.pre-pad.json"),
        ] {
            assert_private(&path);
        }
    });
}
