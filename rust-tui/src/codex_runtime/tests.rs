use super::*;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_home(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("pad-codex-runtime-{name}-{stamp}"))
}

fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("lock codex runtime tests");
    let home = temp_home(name);
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create temp home");

    let prev_home = std::env::var_os("HOME");
    let prev_openai_api_key = std::env::var_os(auth::TEST_OPENAI_API_KEY_ENV);
    std::env::set_var("HOME", &home);
    std::env::remove_var(auth::TEST_OPENAI_API_KEY_ENV);

    let result = f(&home);

    if let Some(prev) = prev_home {
        std::env::set_var("HOME", prev);
    } else {
        std::env::remove_var("HOME");
    }
    if let Some(prev) = prev_openai_api_key {
        std::env::set_var(auth::TEST_OPENAI_API_KEY_ENV, prev);
    } else {
        std::env::remove_var(auth::TEST_OPENAI_API_KEY_ENV);
    }
    let _ = std::fs::remove_dir_all(&home);

    result
}

#[test]
fn codex_agent_command_uses_pad_profile_without_codex_home() {
    with_temp_home("profile", |_home| {
        let command = with_pad_codex_runtime("codex --model gpt-5");
        assert!(command.starts_with("'"));
        assert!(command.contains("/.pad/scripts/pad-codex'"));
        assert!(command.ends_with(" --model gpt-5"));
        assert!(!command.contains("OPENAI_API_KEY="));
    });
}

#[test]
fn codex_agent_command_replaces_existing_profile_with_pad() {
    with_temp_home("replace-profile", |_home| {
        let command = with_pad_codex_runtime("codex --profile work --model gpt-5");

        assert!(command.contains("/.pad/scripts/pad-codex' --model gpt-5"));
        assert!(!command.contains("--profile work"));
    });
}

#[test]
fn codex_agent_command_uses_wrapper_instead_of_inlining_auth() {
    with_temp_home("auth", |_home| {
        let auth_path = crate::paths::pad_codex_auth_path();
        std::fs::create_dir_all(auth_path.parent().expect("auth parent"))
            .expect("create auth parent");
        std::fs::write(&auth_path, r#"{"OPENAI_API_KEY":"sk-test'1"}"#).expect("write auth");

        let command = with_pad_codex_runtime("codex");

        assert!(command.contains("/.pad/scripts/pad-codex'"));
        assert!(!command.contains("sk-test"));
    });
}

#[test]
fn codex_prepare_fails_when_pad_provider_requires_auth_but_key_is_missing() {
    with_temp_home("missing-auth", |_home| {
        let config_path = crate::paths::pad_codex_config_path();
        std::fs::create_dir_all(config_path.parent().expect("config parent"))
            .expect("create config parent");
        std::fs::write(
            &config_path,
            r#"
model_provider = "local"

[model_providers.local]
base_url = "http://localhost:8317/v1"
name = "local"
requires_openai_auth = true
"#,
        )
        .expect("write config");

        let err = prepare_agent_command("codex", "codex").expect_err("missing key");
        assert!(err.to_string().contains("needs relay auth"));
    });
}

#[test]
fn codex_prepare_allows_required_auth_when_pad_key_exists() {
    with_temp_home("ready-auth", |_home| {
        let config_path = crate::paths::pad_codex_config_path();
        std::fs::create_dir_all(config_path.parent().expect("config parent"))
            .expect("create config parent");
        std::fs::write(
            &config_path,
            r#"
model_provider = "local"

[model_providers.local]
base_url = "http://localhost:8317/v1"
name = "local"
requires_openai_auth = true
"#,
        )
        .expect("write config");
        let auth_path = crate::paths::pad_codex_auth_path();
        std::fs::create_dir_all(auth_path.parent().expect("auth parent"))
            .expect("create auth parent");
        std::fs::write(&auth_path, r#"{"OPENAI_API_KEY":"local-key"}"#).expect("write auth");

        let command = prepare_agent_command("codex", "codex --model gpt-5").expect("ready");
        assert!(command.contains("/.pad/scripts/pad-codex' --model gpt-5"));
    });
}

#[test]
fn first_command_token_accepts_absolute_codex_path() {
    assert_eq!(
        command::first_command_token("/opt/bin/codex --version"),
        Some("codex")
    );
}

#[test]
fn non_codex_agent_is_not_wrapped() {
    assert!(!command::is_codex_agent("claude", "claude"));
}
