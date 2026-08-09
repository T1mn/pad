pub(crate) mod attach {
    use super::super::opencode_attach::{attach_command, normalize_server_url};

    pub(crate) fn attach_url_accepts_single_http_url_and_strips_quotes() {
        assert_eq!(
            normalize_server_url("'http://localhost:4096/'").unwrap(),
            "http://localhost:4096"
        );
        assert_eq!(
            normalize_server_url("https://example.com/path").unwrap(),
            "https://example.com/path"
        );
    }

    pub(crate) fn attach_url_rejects_multi_line_or_non_http_clipboard() {
        assert!(normalize_server_url("http://a:1\nhttp://b:2").is_err());
        assert!(normalize_server_url("ftp://example.com:21").is_err());
        assert!(normalize_server_url("https://").is_err());
        assert!(normalize_server_url("https://example .com").is_err());
    }

    pub(crate) fn attach_command_preserves_configured_command_and_quotes_url() {
        assert_eq!(
            attach_command("http://localhost:4096/a'b", "/opt/opencode --pure"),
            "/opt/opencode --pure 'attach' 'http://localhost:4096/a'\\''b'"
        );
    }
}

pub(crate) mod cli {
    use super::super::opencode_cli::{command_with_args, safe_filename};

    pub(crate) fn configured_command_keeps_shell_expansion_and_flags() {
        assert_eq!(
            command_with_args("~/.opencode/bin/opencode --pure", ["web"]),
            "~/.opencode/bin/opencode --pure 'web'"
        );
    }

    pub(crate) fn safe_filename_sanitizes_and_falls_back() {
        assert_eq!(safe_filename("ses/../abc def"), "ses_abc_def");
        assert_eq!(safe_filename("***"), "session");
    }

    pub(crate) fn safe_filename_limits_output_length() {
        assert_eq!(safe_filename(&"a".repeat(120)).len(), 96);
    }

    pub(crate) fn safe_filename_keeps_underscore_inside_truncated_output() {
        let value = format!("{} {}", "a".repeat(95), "b");
        let filename = safe_filename(&value);

        assert_eq!(filename.len(), 96);
        assert!(filename.ends_with('_'));
    }
}

pub(crate) mod export {
    use super::super::opencode_export::{opencode_export_path, ExportMode};
    use std::path::Path;

    pub(crate) fn opencode_export_path_sanitizes_session_id() {
        assert_eq!(
            opencode_export_path("ses/../abc def", Path::new("/tmp/out"), ExportMode::Raw),
            Path::new("/tmp/out/ses_abc_def.json")
        );
    }

    pub(crate) fn opencode_sanitized_export_path_uses_distinct_suffix() {
        assert_eq!(
            opencode_export_path("ses_123", Path::new("/tmp/out"), ExportMode::Sanitized),
            Path::new("/tmp/out/ses_123.sanitized.json")
        );
    }
}

pub(crate) mod github {
    use super::super::opencode_github::github_install_command;

    pub(crate) fn github_install_command_preserves_configured_command() {
        assert_eq!(
            github_install_command("'/opt/open code/bin/opencode' --pure"),
            "'/opt/open code/bin/opencode' --pure 'github' 'install'"
        );
    }
}

pub(crate) mod import {
    use super::super::opencode_import::{normalize_import_source, trim_wrapping_quotes};

    pub(crate) fn import_source_accepts_opencode_share_url() {
        assert_eq!(
            normalize_import_source(" https://opencode.ai/s/abc123 \n").unwrap(),
            "https://opencode.ai/s/abc123"
        );
    }

    pub(crate) fn import_source_accepts_json_path_and_strips_quotes() {
        assert_eq!(
            normalize_import_source("'/tmp/session.sanitized.json'").unwrap(),
            "/tmp/session.sanitized.json"
        );
        assert_eq!(trim_wrapping_quotes("\"/tmp/a.json\""), "/tmp/a.json");
    }

    pub(crate) fn import_source_rejects_multi_line_clipboard() {
        assert!(normalize_import_source("/tmp/a.json\n/tmp/b.json").is_err());
    }
}

pub(crate) mod native_launch {
    use crate::app::{App, TerminalPaneId};
    use crate::model::AgentType;
    use crate::terminal_runtime::TerminalSize;

    #[cfg(unix)]
    pub(crate) fn action_launches_opencode_in_native_terminal_and_registry() {
        crate::test_support::with_temp_home("pad-opencode-action", "native-launch", |home| {
            let cwd = home.join("project");
            std::fs::create_dir_all(&cwd).unwrap();
            let marker = home.join("native-action-launched");
            let command = format!(
                "printf native-action > {}",
                crate::codex_runtime::shell_single_quote(&marker.to_string_lossy())
            );

            let mut app = App::new();
            app.start_native_terminal(TerminalSize::new(101, 33))
                .unwrap();
            let initial_pane_id = app.focused_terminal_pane_id().unwrap();
            wait_for_pane_size(&mut app, initial_pane_id, TerminalSize::new(101, 33));
            app.sidebar.show_tree = true;
            let pane_id = app
                .launch_native_agent_action(
                    "OpenCode Test",
                    &command,
                    AgentType::OpenCode,
                    cwd.clone(),
                )
                .unwrap();
            wait_for_pane_size(&mut app, pane_id, TerminalSize::new(101, 33));

            assert!(app.terminal_is_focused());
            assert_eq!(
                app.terminal_pane(pane_id).unwrap().size(),
                Some(TerminalSize::new(101, 33))
            );
            assert_eq!(app.terminal_pane(pane_id).unwrap().cwd(), cwd);
            assert_eq!(app.terminal_pane(pane_id).unwrap().label(), "OpenCode Test");
            assert_eq!(app.panels.len(), 1);
            assert_eq!(app.panels[0].agent_type, AgentType::OpenCode);
            assert_eq!(app.panels[0].working_dir, cwd.to_string_lossy());
            assert!(App::is_native_agent_terminal_id(&app.panels[0].pane_id));
            assert_eq!(
                app.sidebar.selected_sidebar_key.as_deref(),
                Some(format!("live:{}", app.panels[0].pane_id).as_str())
            );
            for _ in 0..100 {
                if marker.exists() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            assert_eq!(std::fs::read_to_string(marker).unwrap(), "native-action");
            app.shutdown_native_terminal().unwrap();
        });
    }

    fn wait_for_pane_size(app: &mut App, pane_id: TerminalPaneId, expected: TerminalSize) {
        for _ in 0..100 {
            app.poll_native_terminal();
            if app.terminal_pane(pane_id).and_then(|pane| pane.size()) == Some(expected) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(
            app.terminal_pane(pane_id).and_then(|pane| pane.size()),
            Some(expected)
        );
    }
}

pub(crate) mod plugin {
    use super::super::opencode_plugin::{normalize_plugin_module, plugin_command};

    pub(crate) fn plugin_module_accepts_npm_names_scope_and_versions() {
        assert_eq!(
            normalize_plugin_module("opencode-foo").unwrap(),
            "opencode-foo"
        );
        assert_eq!(
            normalize_plugin_module("'@scope/opencode-plugin@1.2.3'").unwrap(),
            "@scope/opencode-plugin@1.2.3"
        );
    }

    pub(crate) fn plugin_module_rejects_empty_multiline_flags_and_whitespace() {
        assert!(normalize_plugin_module(" ").is_err());
        assert!(normalize_plugin_module("a\nb").is_err());
        assert!(normalize_plugin_module("--global").is_err());
        assert!(normalize_plugin_module("opencode plugin").is_err());
        assert!(normalize_plugin_module("pkg;rm").is_err());
    }

    pub(crate) fn plugin_command_preserves_configured_command_and_quotes_module() {
        assert_eq!(
            plugin_command(
                "@scope/opencode-plugin@1.2.3",
                "'/opt/open code/bin/opencode' --pure",
            ),
            "'/opt/open code/bin/opencode' --pure 'plugin' '@scope/opencode-plugin@1.2.3'"
        );
    }
}

pub(crate) mod pr {
    use super::super::opencode_pr::{normalize_pr_number, pr_command};

    pub(crate) fn pr_number_accepts_plain_hash_and_github_url() {
        assert_eq!(normalize_pr_number("123").unwrap(), "123");
        assert_eq!(normalize_pr_number("#456").unwrap(), "456");
        assert_eq!(
            normalize_pr_number("https://github.com/acme/repo/pull/789/files").unwrap(),
            "789"
        );
    }

    pub(crate) fn pr_number_rejects_empty_zero_multiline_and_non_pr_url() {
        assert!(normalize_pr_number(" ").is_err());
        assert!(normalize_pr_number("0").is_err());
        assert!(normalize_pr_number("1\n2").is_err());
        assert!(normalize_pr_number("https://github.com/acme/repo/issues/12").is_err());
        assert!(normalize_pr_number("abc123").is_err());
    }

    pub(crate) fn pr_command_preserves_configured_command() {
        assert_eq!(
            pr_command("123", "'/opt/open code/bin/opencode' --pure"),
            "'/opt/open code/bin/opencode' --pure 'pr' '123'"
        );
    }
}

pub(crate) mod run {
    use super::super::opencode_run::{normalize_prompt, prompt_preview, run_command};

    pub(crate) fn run_prompt_trims_outer_blank_space_but_keeps_multiline_body() {
        assert_eq!(
            normalize_prompt("\n  fix this\nkeep context  \n").unwrap(),
            "fix this\nkeep context"
        );
        assert!(normalize_prompt(" \n\t ").is_err());
    }

    pub(crate) fn run_command_quotes_prompt_and_resumes_opencode_session() {
        assert_eq!(
            run_command(
                "fix Bob's bug\nnow",
                Some("ses'sion"),
                "'/opt/open code/bin/opencode' --pure",
            ),
            "'/opt/open code/bin/opencode' --pure run --session 'ses'\\''sion' -- \"$(printf '%b' 'fix Bob'\\''s bug\\0012now')\""
        );
    }

    pub(crate) fn run_command_can_start_new_session_without_selected_opencode_thread() {
        assert_eq!(
            run_command("hello", None, "opencode"),
            "opencode run -- \"$(printf '%b' 'hello')\""
        );
    }

    pub(crate) fn run_command_is_one_shell_line_and_preserves_backslashes() {
        let command = run_command("first\\path\t\r\nsecond", None, "opencode");

        assert!(!command.contains(['\r', '\n']));
        assert_eq!(
            command,
            "opencode run -- \"$(printf '%b' 'first\\\\path\\0011\\0015\\0012second')\""
        );
    }

    pub(crate) fn run_prompt_preview_uses_first_non_empty_line() {
        assert_eq!(prompt_preview("\nfirst\nsecond"), "first");
    }
}

pub(crate) mod serve {
    use super::super::opencode_serve::serve_command;

    pub(crate) fn serve_command_stays_local_and_uses_random_port() {
        assert_eq!(
            serve_command("'/opt/open code/bin/opencode' --pure"),
            "'/opt/open code/bin/opencode' --pure 'serve' '--hostname' '127.0.0.1' '--port' '0'"
        );
    }
}

pub(crate) mod stats {
    use super::super::opencode_stats::opencode_stats_path;
    use std::path::Path;

    pub(crate) fn opencode_stats_path_sanitizes_project() {
        assert_eq!(
            opencode_stats_path("/Users/tim/my repo", Path::new("/tmp/stats"), 42),
            Path::new("/tmp/stats/Users_tim_my_repo-42.txt")
        );
    }
}

pub(crate) mod web {
    use super::super::opencode_web::web_command;

    pub(crate) fn web_command_preserves_configured_opencode_command() {
        assert_eq!(
            web_command("~/.opencode/bin/opencode --pure"),
            "~/.opencode/bin/opencode --pure 'web'"
        );
    }
}
