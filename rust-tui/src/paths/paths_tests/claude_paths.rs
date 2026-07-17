use super::support::with_temp_home;

#[test]
fn claude_paths_follow_config_dir_override() {
    with_temp_home("claude-config-dir", |home| {
        let config_dir = home.join("custom-claude");
        let previous = std::env::var_os("CLAUDE_CONFIG_DIR");
        std::env::set_var("CLAUDE_CONFIG_DIR", &config_dir);

        assert_eq!(super::super::claude_config_dir(), config_dir);
        assert_eq!(
            super::super::claude_projects_dir(),
            config_dir.join("projects")
        );
        assert_eq!(
            super::super::claude_settings_path(),
            config_dir.join("settings.json")
        );

        restore_config_dir(previous);
    });
}

#[test]
fn claude_paths_fall_back_for_missing_or_empty_override() {
    with_temp_home("claude-config-fallback", |home| {
        let previous = std::env::var_os("CLAUDE_CONFIG_DIR");
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        assert_eq!(super::super::claude_config_dir(), home.join(".claude"));

        std::env::set_var("CLAUDE_CONFIG_DIR", "");
        assert_eq!(super::super::claude_config_dir(), home.join(".claude"));

        restore_config_dir(previous);
    });
}

fn restore_config_dir(previous: Option<std::ffi::OsString>) {
    if let Some(previous) = previous {
        std::env::set_var("CLAUDE_CONFIG_DIR", previous);
    } else {
        std::env::remove_var("CLAUDE_CONFIG_DIR");
    }
}
