mod codex_permissions {
    use super::*;
    #[test]
    fn runtime_configs_apply_codex_full_access_without_relay_provider() {
        with_temp_home("codex-permissions", |_home| {
            let config_path = crate::paths::pad_codex_config_path();
            std::fs::create_dir_all(config_path.parent().expect("codex config parent"))
                .expect("create codex config parent");
            std::fs::write(
                &config_path,
                "model = \"gpt-5\"\napproval_policy = \"on-request\"\n",
            )
            .expect("seed codex config");

            let agent = AgentConfig {
                name: "codex".into(),
                cmd: "codex".into(),
                providers: Vec::new(),
                active_provider: None,
                default_model: String::new(),
                small_model: String::new(),
            };

            apply_runtime_configs(&[agent], &sample_permissions(), &sample_codex_config());

            let value = std::fs::read_to_string(&config_path).expect("read codex config");
            assert!(value.contains("approval_policy = \"never\""));
            assert!(value.contains("sandbox_mode = \"danger-full-access\""));
            assert!(codex_permission_state_path().exists());
        });
    }

    #[test]
    fn runtime_configs_restore_previous_codex_permission_fields_when_disabled() {
        with_temp_home("codex-permissions-restore", |_home| {
            let config_path = crate::paths::pad_codex_config_path();
            std::fs::create_dir_all(config_path.parent().expect("codex config parent"))
                .expect("create codex config parent");
            std::fs::write(
                &config_path,
                "model = \"gpt-5\"\napproval_policy = \"on-request\"\n",
            )
            .expect("seed codex config");

            let agent = AgentConfig {
                name: "codex".into(),
                cmd: "codex".into(),
                providers: Vec::new(),
                active_provider: None,
                default_model: String::new(),
                small_model: String::new(),
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

            let value = std::fs::read_to_string(&config_path).expect("read codex config");
            assert!(value.contains("approval_policy = \"on-request\""));
            assert!(!value.contains("sandbox_mode = \"danger-full-access\""));
            assert!(!codex_permission_state_path().exists());
        });
    }
}

mod codex_features {
    use super::*;
    mod fast_mode {
        use super::*;
        use super::support::{codex_agent_without_relay_provider, seed_pad_codex_config};

        #[test]
        fn runtime_configs_apply_codex_fast_mode_without_relay_provider() {
            with_temp_home("codex-fast-mode", |_home| {
                let config_path = seed_pad_codex_config("model = \"gpt-5\"\n");
                let agent = codex_agent_without_relay_provider();

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
            with_temp_home("codex-fast-mode-restore", |_home| {
                let config_path = seed_pad_codex_config(
                    "model = \"gpt-5\"\nservice_tier = \"default\"\n[features]\nfast_mode = false\n",
                );
                let agent = codex_agent_without_relay_provider();

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
    }

    mod goals {
        use super::*;
        use super::support::{codex_agent_without_relay_provider, seed_pad_codex_config};

        #[test]
        fn runtime_configs_apply_codex_goals_without_relay_provider() {
            with_temp_home("codex-goals", |_home| {
                let config_path = seed_pad_codex_config("model = \"gpt-5\"\n");
                let agent = codex_agent_without_relay_provider();

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
            with_temp_home("codex-goals-restore", |_home| {
                let config_path = seed_pad_codex_config("model = \"gpt-5\"\n[features]\ngoals = false\n");
                let agent = codex_agent_without_relay_provider();

                let mut codex = sample_codex_config();
                codex.goals = true;
                apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

                codex.goals = false;
                apply_runtime_configs(&[agent], &sample_permissions(), &codex);

                let value = std::fs::read_to_string(&config_path).expect("read codex config");
                assert!(value.contains("goals = false"));
            });
        }
    }

    mod multi_agent {
        use super::*;
        use super::support::{codex_agent_without_relay_provider, seed_pad_codex_config};

        #[test]
        fn runtime_configs_apply_codex_multi_agent_without_relay_provider() {
            with_temp_home("codex-multi-agent", |_home| {
                let config_path = seed_pad_codex_config("model = \"gpt-5\"\n");
                let agent = codex_agent_without_relay_provider();

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
            with_temp_home("codex-multi-agent-restore", |_home| {
                let config_path = seed_pad_codex_config(
                    "model = \"gpt-5\"\n[features]\nmulti_agent = false\n",
                );
                let agent = codex_agent_without_relay_provider();

                let mut codex = sample_codex_config();
                codex.multi_agent = true;
                apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

                codex.multi_agent = false;
                apply_runtime_configs(&[agent], &sample_permissions(), &codex);

                let value = std::fs::read_to_string(&config_path).expect("read codex config");
                assert!(value.contains("multi_agent = false"));
            });
        }
    }

    mod support {
        use super::*;
        pub(super) fn seed_pad_codex_config(contents: &str) -> std::path::PathBuf {
            let config_path = crate::paths::pad_codex_config_path();
            std::fs::create_dir_all(config_path.parent().expect("codex config parent"))
                .expect("create codex config parent");
            std::fs::write(&config_path, contents).expect("seed codex config");
            config_path
        }

        pub(super) fn codex_agent_without_relay_provider() -> AgentConfig {
            AgentConfig {
                name: "codex".into(),
                cmd: "codex".into(),
                providers: Vec::new(),
                active_provider: None,
                default_model: String::new(),
                small_model: String::new(),
            }
        }
    }

    mod web_search {
        use super::*;
        use super::support::{codex_agent_without_relay_provider, seed_pad_codex_config};

        #[test]
        fn runtime_configs_apply_codex_web_search_without_relay_provider() {
            with_temp_home("codex-web-search", |_home| {
                let config_path = seed_pad_codex_config("model = \"gpt-5\"\n");
                let agent = codex_agent_without_relay_provider();

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
            with_temp_home("codex-web-search-restore", |_home| {
                let config_path = seed_pad_codex_config("model = \"gpt-5\"\nweb_search = \"cached\"\n");
                let agent = codex_agent_without_relay_provider();

                let mut codex = sample_codex_config();
                codex.web_search = "live".into();
                apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

                codex.web_search = "default".into();
                apply_runtime_configs(&[agent], &sample_permissions(), &codex);

                let value = std::fs::read_to_string(&config_path).expect("read codex config");
                assert!(value.contains("web_search = \"cached\""));
            });
        }
    }
}

mod codex_status_line {
    use super::*;
    #[test]
    fn runtime_configs_apply_codex_status_line_without_relay_provider() {
        with_temp_home("codex-status-line", |_home| {
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
            };

            let mut codex = sample_codex_config();
            codex.status_line_model_with_reasoning = true;
            codex.status_line_fast_mode = true;
            codex.status_line_five_hour_limit = true;
            codex.status_line_weekly_limit = true;
            codex.status_line_context_remaining = true;
            codex.status_line_current_dir = true;
            apply_runtime_configs(&[agent], &sample_permissions(), &codex);

            let value = std::fs::read_to_string(&config_path).expect("read codex config");
            assert!(value.contains("[tui]"));
            assert!(value.contains(
                "status_line = [\"model-with-reasoning\", \"fast-mode\", \"five-hour-limit\", \"weekly-limit\", \"context-remaining\", \"current-dir\"]"
            ));
            assert!(codex_permission_state_path().exists());
        });
    }

    #[test]
    fn runtime_configs_apply_partial_codex_status_line_without_relay_provider() {
        with_temp_home("codex-status-line-partial", |_home| {
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
            };

            let mut codex = sample_codex_config();
            codex.status_line_five_hour_limit = true;
            codex.status_line_weekly_limit = true;
            apply_runtime_configs(&[agent], &sample_permissions(), &codex);

            let value = std::fs::read_to_string(&config_path).expect("read codex config");
            assert!(value.contains("status_line = [\"five-hour-limit\", \"weekly-limit\"]"));
        });
    }

    #[test]
    fn runtime_configs_restore_previous_codex_status_line_when_disabled() {
        with_temp_home("codex-status-line-restore", |_home| {
            let config_path = crate::paths::pad_codex_config_path();
            std::fs::create_dir_all(config_path.parent().expect("codex config parent"))
                .expect("create codex config parent");
            std::fs::write(
                &config_path,
                "model = \"gpt-5\"\n[tui]\nstatus_line = [\"project\", \"git-branch\"]\n",
            )
            .expect("seed codex config");

            let agent = AgentConfig {
                name: "codex".into(),
                cmd: "codex".into(),
                providers: Vec::new(),
                active_provider: None,
                default_model: String::new(),
                small_model: String::new(),
            };

            let mut codex = sample_codex_config();
            codex.status_line_model_with_reasoning = true;
            codex.status_line_fast_mode = true;
            codex.status_line_five_hour_limit = true;
            codex.status_line_weekly_limit = true;
            codex.status_line_context_remaining = true;
            codex.status_line_current_dir = true;
            apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

            codex.status_line_model_with_reasoning = false;
            codex.status_line_fast_mode = false;
            codex.status_line_five_hour_limit = false;
            codex.status_line_weekly_limit = false;
            codex.status_line_context_remaining = false;
            codex.status_line_current_dir = false;
            apply_runtime_configs(&[agent], &sample_permissions(), &codex);

            let value = std::fs::read_to_string(&config_path).expect("read codex config");
            assert!(value.contains("status_line = [\"project\", \"git-branch\"]"));
        });
    }
}

mod codex_prompts {
    use super::*;
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
}

mod combined_overlays {
    use super::*;
    #[test]
    fn runtime_configs_apply_combined_codex_overlays_together() {
        with_temp_home("codex-combined-overlays", |_home| {
            let config_path = crate::paths::pad_codex_config_path();
            std::fs::create_dir_all(config_path.parent().expect("codex config parent"))
                .expect("create codex config parent");
            std::fs::write(
                &config_path,
                "model = \"gpt-5\"\napproval_policy = \"on-request\"\nservice_tier = \"default\"\nweb_search = \"cached\"\n[features]\nfast_mode = false\nmulti_agent = false\n",
            )
            .expect("seed codex config");

            let agent = AgentConfig {
                name: "codex".into(),
                cmd: "codex".into(),
                providers: Vec::new(),
                active_provider: None,
                default_model: String::new(),
                small_model: String::new(),
            };

            let mut codex = sample_codex_config();
            codex.fast_mode = true;
            codex.multi_agent = true;
            codex.web_search = "live".into();
            codex.status_line_model_with_reasoning = true;
            codex.status_line_fast_mode = true;
            codex.status_line_five_hour_limit = true;
            codex.status_line_weekly_limit = true;
            codex.status_line_context_remaining = true;
            codex.status_line_current_dir = true;
            apply_runtime_configs(&[agent], &sample_permissions(), &codex);

            let value = std::fs::read_to_string(&config_path).expect("read codex config");
            assert!(value.contains("approval_policy = \"never\""));
            assert!(value.contains("sandbox_mode = \"danger-full-access\""));
            assert!(value.contains("service_tier = \"fast\""));
            assert!(value.contains("web_search = \"live\""));
            assert!(value.contains("fast_mode = true"));
            assert!(value.contains("multi_agent = true"));
            assert!(value.contains(
                "status_line = [\"model-with-reasoning\", \"fast-mode\", \"five-hour-limit\", \"weekly-limit\", \"context-remaining\", \"current-dir\"]"
            ));
            assert!(codex_permission_state_path().exists());
        });
    }

    #[test]
    fn runtime_configs_restore_combined_codex_overlays_to_original_values() {
        with_temp_home("codex-combined-restore", |_home| {
            let config_path = crate::paths::pad_codex_config_path();
            std::fs::create_dir_all(config_path.parent().expect("codex config parent"))
                .expect("create codex config parent");
            std::fs::write(
                &config_path,
                "model = \"gpt-5\"\napproval_policy = \"on-request\"\nservice_tier = \"default\"\nweb_search = \"cached\"\n[features]\nfast_mode = false\nmulti_agent = false\n",
            )
            .expect("seed codex config");

            let agent = AgentConfig {
                name: "codex".into(),
                cmd: "codex".into(),
                providers: Vec::new(),
                active_provider: None,
                default_model: String::new(),
                small_model: String::new(),
            };

            let mut codex = sample_codex_config();
            codex.fast_mode = true;
            codex.multi_agent = true;
            codex.web_search = "live".into();
            codex.status_line_model_with_reasoning = true;
            codex.status_line_fast_mode = true;
            codex.status_line_five_hour_limit = true;
            codex.status_line_weekly_limit = true;
            codex.status_line_context_remaining = true;
            codex.status_line_current_dir = true;
            apply_runtime_configs(std::slice::from_ref(&agent), &sample_permissions(), &codex);

            let disabled = AgentPermissionsConfig {
                codex_auto_full_access: false,
                claude_auto_full_access: true,
            };
            codex.fast_mode = false;
            codex.multi_agent = false;
            codex.web_search = "default".into();
            codex.status_line_model_with_reasoning = false;
            codex.status_line_fast_mode = false;
            codex.status_line_five_hour_limit = false;
            codex.status_line_weekly_limit = false;
            codex.status_line_context_remaining = false;
            codex.status_line_current_dir = false;
            apply_runtime_configs(&[agent], &disabled, &codex);

            let value = std::fs::read_to_string(&config_path).expect("read codex config");
            assert!(value.contains("approval_policy = \"on-request\""));
            assert!(!value.contains("sandbox_mode = \"danger-full-access\""));
            assert!(value.contains("service_tier = \"default\""));
            assert!(value.contains("web_search = \"cached\""));
            assert!(value.contains("fast_mode = false"));
            assert!(value.contains("multi_agent = false"));
            assert!(!value.contains(
                "status_line = [\"model-with-reasoning\", \"fast-mode\", \"five-hour-limit\", \"weekly-limit\", \"context-remaining\", \"current-dir\"]"
            ));
            assert!(!codex_permission_state_path().exists());
        });
    }
}

mod claude_permissions {
    use super::*;
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
}
