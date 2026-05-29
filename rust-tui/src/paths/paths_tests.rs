use super::*;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_home(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("pad-paths-{name}-{stamp}"))
}

fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("lock paths tests");
    let home = temp_home(name);
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create temp home");

    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    let result = f(&home);

    if let Some(prev) = prev_home {
        std::env::set_var("HOME", prev);
    } else {
        std::env::remove_var("HOME");
    }
    let _ = std::fs::remove_dir_all(&home);

    result
}

#[test]
fn claude_bridge_template_stays_minimal_and_forwards_turn_id() {
    let template = claude_hook_bridge_template();
    assert!(template.contains(&format!("# pad-bridge-version: {}", CLAUDE_BRIDGE_VERSION)));
    assert!(template.contains("\"turn_id\": payload.get(\"turn_id\")"));
    assert!(!template.contains("def silence_stdio():"));
    assert!(!template.contains("def load_payload():"));
    assert!(!template.contains("stderr=subprocess.DEVNULL"));
}

#[test]
fn codex_bridge_template_keeps_required_stdin_and_turn_id_handling() {
    let template = codex_hook_bridge_template();
    assert!(template.contains(&format!("# pad-bridge-version: {}", CODEX_BRIDGE_VERSION)));
    assert!(template.contains("\"turn_id\": payload.get(\"turn_id\")"));
    assert!(template.contains("def silence_stdio():"));
    assert!(template.contains("def load_payload():"));
    assert!(template.contains("stderr=subprocess.DEVNULL"));
    assert!(template.contains("payload.get(\"hook_event_name\") or hook_type"));
}

#[test]
fn codex_hooks_feature_key_switches_at_0130() {
    assert_eq!(
        codex_hooks_feature_key_for_version(Some("0.129.9")),
        "codex_hooks"
    );
    assert_eq!(
        codex_hooks_feature_key_for_version(Some("0.130.0")),
        "hooks"
    );
    assert_eq!(
        codex_hooks_feature_key_for_version(Some("codex 0.130.0")),
        "codex_hooks"
    );
    assert_eq!(codex_hooks_feature_key_for_version(None), "codex_hooks");
}

#[test]
fn parse_codex_cli_version_accepts_plain_and_prefixed_versions() {
    assert_eq!(parse_codex_cli_version("0.130.0"), Some((0, 130, 0)));
    assert_eq!(parse_codex_cli_version("v0.130.1"), Some((0, 130, 1)));
    assert_eq!(parse_codex_cli_version("0.130.0-beta"), Some((0, 130, 0)));
    assert_eq!(parse_codex_cli_version("codex 0.130.0"), None);
}

#[test]
fn set_toml_bool_in_section_writes_new_hooks_key() {
    let updated = set_toml_bool_in_section(
        "[features]\ncodex_hooks = true\n",
        "features",
        "hooks",
        true,
    );

    assert!(updated.contains("[features]\n"));
    assert!(updated.contains("codex_hooks = true\n"));
    assert!(updated.contains("hooks = true\n"));
}

#[test]
fn remove_toml_key_in_section_removes_legacy_codex_hooks_key() {
    let updated = remove_toml_key_in_section(
        "[features]\ncodex_hooks = true\nhooks = true\n",
        "features",
        "codex_hooks",
    );

    assert!(updated.contains("[features]\n"));
    assert!(updated.contains("hooks = true\n"));
    assert!(!updated.contains("codex_hooks = true\n"));
}

#[test]
fn ensure_runtime_layout_creates_codex_jailbreak_prompt_file() {
    with_temp_home("runtime-layout", |_home| {
        ensure_runtime_layout().expect("ensure runtime layout");
        assert!(prompts_dir().is_dir());
        assert!(codex_jailbreak_prompt_file_path().is_file());
        assert_eq!(
            std::fs::read_to_string(codex_jailbreak_prompt_file_path()).expect("read prompt file"),
            DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE
        );
        assert!(codex_index_prompt_file_path().is_file());
        assert_eq!(
            std::fs::read_to_string(codex_index_prompt_file_path()).expect("read prompt file"),
            DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE
        );
    });
}

#[test]
fn ensure_pad_codex_home_layout_copies_config_but_not_auth() {
    with_temp_home("pad-codex-home-config", |home| {
        let canonical = home.join(".codex");
        fs::create_dir_all(&canonical).expect("create canonical codex home");
        fs::write(canonical.join("config.toml"), "model_provider = \"cpa\"\n")
            .expect("seed canonical config");
        fs::write(
            canonical.join("auth.json"),
            "{\"OPENAI_API_KEY\":\"sk-live\"}\n",
        )
        .expect("seed canonical auth");

        ensure_pad_codex_home_layout().expect("ensure pad codex home");

        assert_eq!(
            fs::read_to_string(pad_codex_config_path()).expect("read pad config"),
            "model_provider = \"cpa\"\n"
        );
        assert!(!pad_codex_auth_path().exists());
    });
}

#[cfg(unix)]
#[test]
fn ensure_pad_codex_home_layout_links_sessions_to_canonical_home() {
    with_temp_home("pad-codex-home-sessions", |_home| {
        ensure_pad_codex_home_layout().expect("ensure pad codex home");

        let sessions_meta =
            fs::symlink_metadata(pad_codex_home_dir().join("sessions")).expect("sessions link");
        let db_meta = fs::symlink_metadata(pad_codex_home_dir().join("state_5.sqlite"))
            .expect("state db link");
        let wal_meta = fs::symlink_metadata(pad_codex_home_dir().join("state_5.sqlite-wal"))
            .expect("state wal link");
        assert!(sessions_meta.file_type().is_symlink());
        assert!(db_meta.file_type().is_symlink());
        assert!(wal_meta.file_type().is_symlink());
        assert!(canonical_codex_home_dir().join("sessions").is_dir());
    });
}

#[test]
fn write_codex_selected_prompt_file_combines_selected_candidates() {
    with_temp_home("selected-prompt-combine", |_home| {
        fs::create_dir_all(prompts_dir()).expect("create prompt dir");

        let selected = write_codex_selected_prompt_file(true, true).expect("write selected prompt");

        let selected_path = codex_selected_prompt_file_path();
        assert_eq!(selected.as_deref(), Some(selected_path.as_path()));
        let content = fs::read_to_string(codex_selected_prompt_file_path()).expect("read combined");
        assert!(content.contains(DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE));
        assert!(content.contains(DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE));
    });
}

#[test]
fn write_codex_selected_prompt_file_returns_single_candidate_directly() {
    with_temp_home("selected-prompt-single", |_home| {
        fs::create_dir_all(prompts_dir()).expect("create prompt dir");

        let selected =
            write_codex_selected_prompt_file(false, true).expect("write selected prompt");

        let index_path = codex_index_prompt_file_path();
        assert_eq!(selected.as_deref(), Some(index_path.as_path()));
        assert!(!codex_selected_prompt_file_path().exists());
    });
}

#[test]
fn ensure_runtime_layout_reseeds_empty_codex_jailbreak_prompt_file() {
    with_temp_home("runtime-layout-empty-prompt", |_home| {
        fs::create_dir_all(prompts_dir()).expect("create prompt dir");
        fs::write(codex_jailbreak_prompt_file_path(), "\n\n").expect("seed empty prompt file");

        ensure_runtime_layout().expect("ensure runtime layout");

        assert_eq!(
            std::fs::read_to_string(codex_jailbreak_prompt_file_path()).expect("read prompt file"),
            DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE
        );
    });
}

#[test]
fn ensure_runtime_layout_tracks_current_codex_jailbreak_prompt_version() {
    with_temp_home("runtime-layout-codex-prompt-version", |_home| {
        fs::create_dir_all(prompts_dir()).expect("create prompt dir");
        fs::write(
            codex_jailbreak_prompt_file_path(),
            DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE,
        )
        .expect("seed prompt file");

        ensure_runtime_layout().expect("ensure runtime layout");

        let state = read_managed_prompt_state(&codex_jailbreak_prompt_state_path())
            .expect("read prompt state")
            .expect("managed prompt state");
        assert_eq!(state.version, CODEX_JAILBREAK_PROMPT_VERSION);
        assert_eq!(
            state.content_md5,
            prompt_md5(DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE)
        );
    });
}

#[test]
fn ensure_runtime_layout_refreshes_outdated_managed_codex_jailbreak_prompt() {
    with_temp_home("runtime-layout-refresh-managed-prompt", |_home| {
        let old_prompt = "legacy managed prompt";
        fs::create_dir_all(prompts_dir()).expect("create prompt dir");
        fs::write(codex_jailbreak_prompt_file_path(), old_prompt).expect("seed old prompt");
        write_managed_prompt_state(
            &codex_jailbreak_prompt_state_path(),
            &ManagedPromptState {
                version: "codex-jailbreak-prompt-2026-04-20.1".into(),
                content_md5: prompt_md5(old_prompt),
            },
        )
        .expect("seed prompt state");

        ensure_runtime_layout().expect("ensure runtime layout");

        assert_eq!(
            fs::read_to_string(codex_jailbreak_prompt_file_path()).expect("read prompt file"),
            DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE
        );
        let state = read_managed_prompt_state(&codex_jailbreak_prompt_state_path())
            .expect("read prompt state")
            .expect("managed prompt state");
        assert_eq!(state.version, CODEX_JAILBREAK_PROMPT_VERSION);
        assert_eq!(
            state.content_md5,
            prompt_md5(DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE)
        );
    });
}

#[test]
fn ensure_runtime_layout_preserves_custom_codex_jailbreak_prompt_edits() {
    with_temp_home("runtime-layout-preserve-custom-prompt", |_home| {
        let custom_prompt = "custom operator prompt";
        fs::create_dir_all(prompts_dir()).expect("create prompt dir");
        fs::write(codex_jailbreak_prompt_file_path(), custom_prompt).expect("seed custom prompt");
        write_managed_prompt_state(
            &codex_jailbreak_prompt_state_path(),
            &ManagedPromptState {
                version: "codex-jailbreak-prompt-2026-04-20.1".into(),
                content_md5: prompt_md5("legacy managed prompt"),
            },
        )
        .expect("seed prompt state");

        ensure_runtime_layout().expect("ensure runtime layout");

        assert_eq!(
            fs::read_to_string(codex_jailbreak_prompt_file_path()).expect("read prompt file"),
            custom_prompt
        );
    });
}

#[test]
fn ensure_runtime_layout_migrates_custom_legacy_codex_prompt_to_jailbreak_name() {
    with_temp_home("runtime-layout-migrate-legacy-prompt", |_home| {
        let custom_prompt = "legacy custom jailbreak prompt";
        fs::create_dir_all(prompts_dir()).expect("create prompt dir");
        fs::write(legacy_codex_prompt_file_path(), custom_prompt).expect("seed legacy prompt");

        ensure_runtime_layout().expect("ensure runtime layout");

        assert_eq!(
            fs::read_to_string(codex_jailbreak_prompt_file_path()).expect("read prompt file"),
            custom_prompt
        );
    });
}
