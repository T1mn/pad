use super::super::*;
use super::support::with_temp_home;

fn save_and_reload(api_key: &str, bot_token: &str) -> Config {
    let mut config = Config::default();
    config.telegram.bot_token = bot_token.to_string();
    let claude = config
        .agents
        .iter_mut()
        .find(|agent| agent.name == "claude")
        .expect("claude agent");
    claude.providers.push(ProviderConfig {
        label: "Relay".into(),
        base_url: "https://relay.example/v1".into(),
        api_key: api_key.into(),
        env_key: String::new(),
        wire_api: "responses".into(),
        provider_key: "relay".into(),
        npm_package: String::new(),
        disable_thinking: false,
        models: Vec::new(),
        test_status: None,
        test_http_status: None,
        test_latency_ms: None,
        test_result: None,
    });
    claude.active_provider = Some(0);
    config.save().expect("save config");
    Config::load()
}

fn loaded_api_key(config: &Config) -> String {
    config
        .agents
        .iter()
        .find(|agent| agent.name == "claude")
        .and_then(|agent| agent.providers.first())
        .map(|provider| provider.api_key.clone())
        .expect("claude provider survived the round trip")
}

#[test]
fn api_key_with_backslashes_survives_round_trip() {
    with_temp_home("backslash-api-key", || {
        let loaded = save_and_reload(r"C:\path\to\key", r"123:AA\bb");
        assert_eq!(loaded_api_key(&loaded), r"C:\path\to\key");
        assert_eq!(loaded.telegram.bot_token, r"123:AA\bb");
    });
}

#[test]
fn value_ending_with_backslash_does_not_break_the_file() {
    with_temp_home("trailing-backslash", || {
        let loaded = save_and_reload(r"secret\", "plain");
        assert_eq!(loaded_api_key(&loaded), r"secret\");
        // 结构没被吃掉：后面的 agent 仍然都在。
        assert_eq!(loaded.agents.len(), Config::default().agents.len());
    });
}

#[test]
fn multiline_and_control_character_values_survive_round_trip() {
    with_temp_home("multiline-value", || {
        let loaded = save_and_reload("line1\nline2\n", "tab\tand\r\ncrlf");
        assert_eq!(loaded_api_key(&loaded), "line1\nline2\n");
        assert_eq!(loaded.telegram.bot_token, "tab\tand\r\ncrlf");
    });
}

#[test]
fn broken_config_is_backed_up_before_falling_back_to_defaults() {
    with_temp_home("broken-config-backup", || {
        let path = Config::config_path();
        std::fs::create_dir_all(path.parent().expect("config parent")).expect("create config dir");
        let broken = "[[agents]]\nname = \"claude\"\napi_key = \"C:\\path\"\n";
        std::fs::write(&path, broken).expect("write broken config");

        let report = Config::load_reported();
        let recovery = report.recovery.expect("parse failure must be reported");
        let backup = recovery.backup.expect("broken config must be backed up");

        assert_eq!(
            std::fs::read_to_string(&backup).expect("read backup"),
            broken,
            "original bytes must be recoverable after the default fallback"
        );
        assert!(recovery.error.contains("parse"));

        // 回退默认值后再保存一次（模拟用户随手改个设置），备份仍然在。
        report.config.save().expect("save defaults");
        assert!(backup.exists());
        assert_eq!(
            std::fs::read_to_string(&backup).expect("read backup again"),
            broken
        );
    });
}

#[cfg(unix)]
#[test]
fn saved_config_is_owner_only_readable() {
    use std::os::unix::fs::PermissionsExt;

    with_temp_home("config-permissions", || {
        let path = Config::config_path();
        std::fs::create_dir_all(path.parent().expect("config parent")).expect("create config dir");
        std::fs::write(&path, "theme = \"default\"\n").expect("seed config");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .expect("loosen perms");

        Config::default().save().expect("save config");

        let mode = std::fs::metadata(&path)
            .expect("stat config")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "config.toml stores plaintext secrets");
    });
}
