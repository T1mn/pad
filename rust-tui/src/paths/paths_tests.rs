use super::*;

pub(crate) mod bridge_hooks {
    use std::io::Write;
    use std::process::{Command, Stdio};

    use super::*;

    fn assert_valid_python(script: &str, label: &str) {
        let mut child = Command::new("python3")
            .args([
                "-c",
                "import sys; compile(sys.stdin.read(), sys.argv[1], 'exec')",
                label,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn python3 syntax check");
        child
            .stdin
            .take()
            .expect("python stdin")
            .write_all(script.as_bytes())
            .expect("write bridge template to python");
        let output = child
            .wait_with_output()
            .expect("wait for python syntax check");

        assert!(
            output.status.success(),
            "{label} is not valid Python: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    pub(crate) fn claude_bridge_template_stays_minimal_and_forwards_turn_id() {
        let template = claude_hook_bridge_template();
        assert_valid_python(&template, "claude_hook_bridge.py");
        assert!(template.contains(&format!("# pad-bridge-version: {}", CLAUDE_BRIDGE_VERSION)));
        assert!(template.contains("\"turn_id\": payload.get(\"turn_id\")"));
        assert!(!template.contains("def silence_stdio():"));
        assert!(!template.contains("def load_payload():"));
        assert!(!template.contains("stderr=subprocess.DEVNULL"));
    }

    pub(crate) fn codex_bridge_template_keeps_required_stdin_and_turn_id_handling() {
        let template = codex_hook_bridge_template();
        assert_valid_python(&template, "codex_hook_bridge.py");
        assert!(template.contains(&format!("# pad-bridge-version: {}", CODEX_BRIDGE_VERSION)));
        assert!(template.contains("\"turn_id\": payload.get(\"turn_id\")"));
        assert!(template.contains("def silence_stdio():"));
        assert!(template.contains("def load_payload():"));
        assert!(template.contains("stderr=subprocess.DEVNULL"));
        assert!(template.contains("payload.get(\"hook_event_name\") or hook_type"));
        assert!(template.contains("def pad_codex_hooks_enabled():"));
        assert!(template.contains("PAD_CODEX_HOOKS"));
        assert!(template.contains("__internal\", \"codex-turn-diff\", \"hook\""));
        assert!(template.contains("record_codex_turn_diff(message)"));
    }

    pub(crate) fn codex_hooks_feature_key_switches_at_0130() {
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

    pub(crate) fn parse_codex_cli_version_accepts_plain_and_prefixed_versions() {
        assert_eq!(parse_codex_cli_version("0.130.0"), Some((0, 130, 0)));
        assert_eq!(parse_codex_cli_version("v0.130.1"), Some((0, 130, 1)));
        assert_eq!(parse_codex_cli_version("0.130.0-beta"), Some((0, 130, 0)));
        assert_eq!(parse_codex_cli_version("codex 0.130.0"), None);
    }

    pub(crate) fn set_toml_bool_in_section_writes_new_hooks_key() {
        let updated = set_toml_bool_in_section(
            "[features]\ncodex_hooks = true\n",
            "features",
            "hooks",
            true,
        );

        assert_eq!(updated, "[features]\ncodex_hooks = true\n\nhooks = true\n");
    }

    pub(crate) fn set_toml_bool_in_section_preserves_leading_blank_line() {
        let updated =
            set_toml_bool_in_section("\n[features]\nhooks = false\n", "features", "hooks", true);

        assert_eq!(updated, "\n[features]\nhooks = true\n");
    }

    pub(crate) fn set_toml_bool_in_section_updates_compact_assignment_only() {
        let updated = set_toml_bool_in_section(
            "[features]\nhooks=false\nhooks_extra=false\n",
            "features",
            "hooks",
            true,
        );
        let parsed = updated
            .parse::<toml::Value>()
            .expect("updated config must remain valid TOML");
        let features = parsed
            .get("features")
            .and_then(toml::Value::as_table)
            .expect("features table");

        assert_eq!(
            features.get("hooks").and_then(toml::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            features.get("hooks_extra").and_then(toml::Value::as_bool),
            Some(false)
        );
    }

    pub(crate) fn remove_toml_key_in_section_removes_legacy_codex_hooks_key() {
        let updated = remove_toml_key_in_section(
            "[features]\ncodex_hooks = true\nhooks = true\n",
            "features",
            "codex_hooks",
        );

        assert_eq!(updated, "[features]\nhooks = true\n");
    }

    pub(crate) fn remove_toml_key_in_section_removes_compact_assignment_only() {
        let updated = remove_toml_key_in_section(
            "[features]\ncodex_hooks=false\ncodex_hooks_extra=false\nhooks=true\n",
            "features",
            "codex_hooks",
        );
        let parsed = updated.parse::<toml::Value>().expect("valid TOML");
        let features = parsed
            .get("features")
            .and_then(toml::Value::as_table)
            .expect("features table");

        assert!(!features.contains_key("codex_hooks"));
        assert_eq!(
            features
                .get("codex_hooks_extra")
                .and_then(toml::Value::as_bool),
            Some(false)
        );
        assert_eq!(
            features.get("hooks").and_then(toml::Value::as_bool),
            Some(true)
        );
    }
}
pub(crate) mod claude_paths {
    use super::support::with_temp_home;

    pub(crate) fn claude_paths_follow_config_dir_override() {
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

    pub(crate) fn claude_paths_fall_back_for_missing_or_empty_override() {
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
}
pub(crate) mod codex_home {
    use super::support::with_temp_home;
    use super::*;

    pub(crate) fn ensure_pad_codex_home_layout_copies_config_to_profile_but_not_auth() {
        with_temp_home("pad-codex-profile-config", |home| {
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
            assert_eq!(
                pad_codex_config_path(),
                pad_codex_home_dir().join("pad.config.toml")
            );
            assert!(!pad_codex_auth_path().exists());
        });
    }

    pub(crate) fn ensure_pad_codex_home_layout_does_not_create_session_or_db_links() {
        with_temp_home("pad-codex-profile-no-links", |_home| {
            ensure_pad_codex_home_layout().expect("ensure pad codex home");

            assert_eq!(
                pad_codex_config_path(),
                pad_codex_home_dir().join("pad.config.toml")
            );
            assert!(!pad_codex_home_dir().join("sessions").exists());
            assert!(!pad_codex_home_dir().join("state_5.sqlite").exists());
            assert!(!pad_codex_home_dir().join("state_5.sqlite-wal").exists());
        });
    }

    #[cfg(unix)]
    pub(crate) fn ensure_pad_codex_home_layout_unlinks_legacy_shared_state_symlinks() {
        with_temp_home("pad-codex-profile-unlink-legacy", |home| {
            use std::os::unix::fs::symlink;

            let canonical = home.join(".codex");
            let canonical_sessions = canonical.join("sessions");
            let canonical_archived = canonical.join("archived_sessions");
            let canonical_db = canonical.join("state_5.sqlite");
            fs::create_dir_all(&canonical_sessions).expect("create canonical sessions");
            fs::create_dir_all(&canonical_archived).expect("create canonical archived");
            fs::write(&canonical_db, "db").expect("write canonical db");

            fs::create_dir_all(pad_codex_home_dir()).expect("create pad codex home");
            symlink(&canonical_sessions, pad_codex_home_dir().join("sessions"))
                .expect("symlink sessions");
            symlink(
                &canonical_archived,
                pad_codex_home_dir().join("archived_sessions"),
            )
            .expect("symlink archived");
            symlink(&canonical_db, pad_codex_home_dir().join("state_5.sqlite"))
                .expect("symlink db");

            ensure_pad_codex_home_layout().expect("ensure pad codex home");

            assert!(!pad_codex_home_dir().join("sessions").exists());
            assert!(!pad_codex_home_dir().join("archived_sessions").exists());
            assert!(!pad_codex_home_dir().join("state_5.sqlite").exists());
            assert!(canonical_sessions.is_dir());
            assert!(canonical_archived.is_dir());
            assert!(canonical_db.is_file());
        });
    }
}
pub(crate) mod prompts {
    use super::support::with_temp_home;
    use super::*;

    pub(crate) fn write_codex_selected_prompt_file_combines_selected_candidates() {
        with_temp_home("selected-prompt-combine", |_home| {
            fs::create_dir_all(prompts_dir()).expect("create prompt dir");

            let selected =
                write_codex_selected_prompt_file(true, true).expect("write selected prompt");

            let selected_path = codex_selected_prompt_file_path();
            assert_eq!(selected.as_deref(), Some(selected_path.as_path()));
            let content =
                fs::read_to_string(codex_selected_prompt_file_path()).expect("read combined");
            assert!(content.contains(DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE));
            assert!(content.contains(DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE));
        });
    }

    pub(crate) fn write_codex_selected_prompt_file_returns_single_candidate_directly() {
        with_temp_home("selected-prompt-single", |_home| {
            fs::create_dir_all(prompts_dir()).expect("create prompt dir");

            let selected =
                write_codex_selected_prompt_file(false, true).expect("write selected prompt");

            let index_path = codex_index_prompt_file_path();
            assert_eq!(selected.as_deref(), Some(index_path.as_path()));
            assert!(!codex_selected_prompt_file_path().exists());
        });
    }

    pub(crate) fn ensure_runtime_layout_reseeds_empty_codex_jailbreak_prompt_file() {
        with_temp_home("runtime-layout-empty-prompt", |_home| {
            fs::create_dir_all(prompts_dir()).expect("create prompt dir");
            fs::write(codex_jailbreak_prompt_file_path(), "\n\n").expect("seed empty prompt file");

            ensure_runtime_layout().expect("ensure runtime layout");

            assert_eq!(
                std::fs::read_to_string(codex_jailbreak_prompt_file_path())
                    .expect("read prompt file"),
                DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE
            );
        });
    }

    pub(crate) fn ensure_runtime_layout_tracks_current_codex_jailbreak_prompt_version() {
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

    pub(crate) fn ensure_runtime_layout_refreshes_outdated_managed_codex_jailbreak_prompt() {
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

    pub(crate) fn ensure_runtime_layout_preserves_custom_codex_jailbreak_prompt_edits() {
        with_temp_home("runtime-layout-preserve-custom-prompt", |_home| {
            let custom_prompt = "custom operator prompt";
            fs::create_dir_all(prompts_dir()).expect("create prompt dir");
            fs::write(codex_jailbreak_prompt_file_path(), custom_prompt)
                .expect("seed custom prompt");
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

    pub(crate) fn ensure_runtime_layout_migrates_custom_legacy_codex_prompt_to_jailbreak_name() {
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
}
pub(crate) mod runtime_layout {
    use super::support::with_temp_home;
    use super::*;

    pub(crate) fn ensure_runtime_layout_creates_codex_jailbreak_prompt_file() {
        with_temp_home("runtime-layout", |_home| {
            ensure_runtime_layout().expect("ensure runtime layout");
            assert!(prompts_dir().is_dir());
            assert!(codex_jailbreak_prompt_file_path().is_file());
            assert_eq!(
                std::fs::read_to_string(codex_jailbreak_prompt_file_path())
                    .expect("read prompt file"),
                DEFAULT_CODEX_JAILBREAK_PROMPT_TEMPLATE
            );
            assert!(codex_index_prompt_file_path().is_file());
            assert_eq!(
                std::fs::read_to_string(codex_index_prompt_file_path()).expect("read prompt file"),
                DEFAULT_CODEX_INDEX_PROMPT_TEMPLATE
            );
            assert!(pad_codex_wrapper_path().is_file());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = fs::metadata(hook_events_path())
                    .expect("hook journal metadata")
                    .permissions()
                    .mode()
                    & 0o777;
                assert_eq!(mode, 0o600);
            }
        });
    }

    pub(crate) fn ensure_runtime_layout_installs_executable_pad_codex_wrapper() {
        with_temp_home("runtime-layout-wrapper", |_home| {
            ensure_runtime_layout().expect("ensure runtime layout");

            let wrapper = pad_codex_wrapper_path();
            let content = fs::read_to_string(&wrapper).expect("read wrapper");
            assert!(content.contains(".pad/codex-home/auth.json"));
            assert!(content.contains("exec \"$CODEX_BIN\" --profile pad \"$@\""));

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = fs::metadata(&wrapper)
                    .expect("wrapper metadata")
                    .permissions()
                    .mode();
                assert_ne!(mode & 0o111, 0);
            }
        });
    }

    pub(crate) fn ensure_runtime_layout_enables_codex_hooks_in_pad_profile_only() {
        with_temp_home("runtime-layout-codex-profile-hooks", |home| {
            let canonical_config = home.join(".codex").join("config.toml");
            fs::create_dir_all(canonical_config.parent().expect("canonical config parent"))
                .expect("create canonical config parent");
            fs::write(&canonical_config, "model = \"gpt-5\"\n").expect("seed canonical config");

            ensure_runtime_layout().expect("ensure runtime layout");

            let canonical = fs::read_to_string(&canonical_config).expect("read canonical config");
            let profile = fs::read_to_string(pad_codex_config_path()).expect("read pad profile");

            assert_eq!(canonical, "model = \"gpt-5\"\n");
            assert!(profile.contains("model = \"gpt-5\""));
            assert!(profile.contains("[features]"));
            assert!(profile.contains("codex_hooks = true") || profile.contains("hooks = true"));
            assert_eq!(
                pad_codex_hooks_path(),
                pad_codex_home_dir().join("hooks.json")
            );
            assert!(pad_codex_hooks_path().exists());
        });
    }
}
mod support {
    use std::path::Path;

    pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
        crate::test_support::with_temp_home("pad-paths", name, f)
    }
}
