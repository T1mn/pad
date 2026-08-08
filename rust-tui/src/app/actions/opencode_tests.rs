mod attach {
    use super::super::opencode_attach::{attach_command, normalize_server_url};
    use std::ffi::OsString;

    #[test]
    fn attach_url_accepts_single_http_url_and_strips_quotes() {
        assert_eq!(
            normalize_server_url("'http://localhost:4096/'").unwrap(),
            "http://localhost:4096"
        );
        assert_eq!(
            normalize_server_url("https://example.com/path").unwrap(),
            "https://example.com/path"
        );
    }

    #[test]
    fn attach_url_rejects_multi_line_or_non_http_clipboard() {
        assert!(normalize_server_url("http://a:1\nhttp://b:2").is_err());
        assert!(normalize_server_url("ftp://example.com:21").is_err());
        assert!(normalize_server_url("https://").is_err());
        assert!(normalize_server_url("https://example .com").is_err());
    }

    #[test]
    fn attach_command_quotes_url_and_command() {
        assert_eq!(
            attach_command(
                "http://localhost:4096/a'b",
                &OsString::from("/opt/opencode")
            ),
            "'/opt/opencode' attach 'http://localhost:4096/a'\\''b'"
        );
    }
}

mod cli {
    use super::super::opencode_cli::{first_command_token, safe_filename};

    #[test]
    fn opencode_command_uses_first_configured_token() {
        assert_eq!(
            first_command_token("/opt/bin/opencode --pure"),
            "/opt/bin/opencode"
        );
    }

    #[test]
    fn safe_filename_sanitizes_and_falls_back() {
        assert_eq!(safe_filename("ses/../abc def"), "ses_abc_def");
        assert_eq!(safe_filename("***"), "session");
    }

    #[test]
    fn safe_filename_limits_output_length() {
        assert_eq!(safe_filename(&"a".repeat(120)).len(), 96);
    }

    #[test]
    fn safe_filename_keeps_underscore_inside_truncated_output() {
        let value = format!("{} {}", "a".repeat(95), "b");
        let filename = safe_filename(&value);

        assert_eq!(filename.len(), 96);
        assert!(filename.ends_with('_'));
    }
}

mod export {
    use super::super::opencode_export::{opencode_export_path, ExportMode};
    use std::path::Path;

    #[test]
    fn opencode_export_path_sanitizes_session_id() {
        assert_eq!(
            opencode_export_path("ses/../abc def", Path::new("/tmp/out"), ExportMode::Raw),
            Path::new("/tmp/out/ses_abc_def.json")
        );
    }

    #[test]
    fn opencode_sanitized_export_path_uses_distinct_suffix() {
        assert_eq!(
            opencode_export_path("ses_123", Path::new("/tmp/out"), ExportMode::Sanitized),
            Path::new("/tmp/out/ses_123.sanitized.json")
        );
    }
}

mod github {
    use super::super::opencode_github::github_install_command;
    use std::ffi::OsString;

    #[test]
    fn github_install_command_quotes_configured_command() {
        assert_eq!(
            github_install_command(&OsString::from("/opt/open code/bin/opencode")),
            "'/opt/open code/bin/opencode' github install"
        );
    }
}

mod import {
    use super::super::opencode_import::{normalize_import_source, trim_wrapping_quotes};

    #[test]
    fn import_source_accepts_opencode_share_url() {
        assert_eq!(
            normalize_import_source(" https://opencode.ai/s/abc123 \n").unwrap(),
            "https://opencode.ai/s/abc123"
        );
    }

    #[test]
    fn import_source_accepts_json_path_and_strips_quotes() {
        assert_eq!(
            normalize_import_source("'/tmp/session.sanitized.json'").unwrap(),
            "/tmp/session.sanitized.json"
        );
        assert_eq!(trim_wrapping_quotes("\"/tmp/a.json\""), "/tmp/a.json");
    }

    #[test]
    fn import_source_rejects_multi_line_clipboard() {
        assert!(normalize_import_source("/tmp/a.json\n/tmp/b.json").is_err());
    }
}

mod plugin {
    use super::super::opencode_plugin::{normalize_plugin_module, plugin_command};
    use std::ffi::OsString;

    #[test]
    fn plugin_module_accepts_npm_names_scope_and_versions() {
        assert_eq!(
            normalize_plugin_module("opencode-foo").unwrap(),
            "opencode-foo"
        );
        assert_eq!(
            normalize_plugin_module("'@scope/opencode-plugin@1.2.3'").unwrap(),
            "@scope/opencode-plugin@1.2.3"
        );
    }

    #[test]
    fn plugin_module_rejects_empty_multiline_flags_and_whitespace() {
        assert!(normalize_plugin_module(" ").is_err());
        assert!(normalize_plugin_module("a\nb").is_err());
        assert!(normalize_plugin_module("--global").is_err());
        assert!(normalize_plugin_module("opencode plugin").is_err());
        assert!(normalize_plugin_module("pkg;rm").is_err());
    }

    #[test]
    fn plugin_command_quotes_configured_command_and_module() {
        assert_eq!(
            plugin_command(
                "@scope/opencode-plugin@1.2.3",
                &OsString::from("/opt/open code/bin/opencode"),
            ),
            "'/opt/open code/bin/opencode' plugin '@scope/opencode-plugin@1.2.3'"
        );
    }
}

mod pr {
    use super::super::opencode_pr::{normalize_pr_number, pr_command};
    use std::ffi::OsString;

    #[test]
    fn pr_number_accepts_plain_hash_and_github_url() {
        assert_eq!(normalize_pr_number("123").unwrap(), "123");
        assert_eq!(normalize_pr_number("#456").unwrap(), "456");
        assert_eq!(
            normalize_pr_number("https://github.com/acme/repo/pull/789/files").unwrap(),
            "789"
        );
    }

    #[test]
    fn pr_number_rejects_empty_zero_multiline_and_non_pr_url() {
        assert!(normalize_pr_number(" ").is_err());
        assert!(normalize_pr_number("0").is_err());
        assert!(normalize_pr_number("1\n2").is_err());
        assert!(normalize_pr_number("https://github.com/acme/repo/issues/12").is_err());
        assert!(normalize_pr_number("abc123").is_err());
    }

    #[test]
    fn pr_command_quotes_configured_command() {
        assert_eq!(
            pr_command("123", &OsString::from("/opt/open code/bin/opencode")),
            "'/opt/open code/bin/opencode' pr 123"
        );
    }
}

mod run {
    use super::super::opencode_run::{normalize_prompt, prompt_preview, run_command};
    use std::ffi::OsString;

    #[test]
    fn run_prompt_trims_outer_blank_space_but_keeps_multiline_body() {
        assert_eq!(
            normalize_prompt("\n  fix this\nkeep context  \n").unwrap(),
            "fix this\nkeep context"
        );
        assert!(normalize_prompt(" \n\t ").is_err());
    }

    #[test]
    fn run_command_quotes_prompt_and_resumes_opencode_session() {
        assert_eq!(
            run_command(
                "fix Bob's bug\nnow",
                Some("ses'sion"),
                &OsString::from("/opt/open code/bin/opencode"),
            ),
            "'/opt/open code/bin/opencode' run --session 'ses'\\''sion' -- 'fix Bob'\\''s bug\nnow'"
        );
    }

    #[test]
    fn run_command_can_start_new_session_without_selected_opencode_thread() {
        assert_eq!(
            run_command("hello", None, &OsString::from("opencode")),
            "'opencode' run -- 'hello'"
        );
    }

    #[test]
    fn run_prompt_preview_uses_first_non_empty_line() {
        assert_eq!(prompt_preview("\nfirst\nsecond"), "first");
    }
}

mod serve {
    use super::super::opencode_serve::serve_command;
    use std::ffi::OsString;

    #[test]
    fn serve_command_stays_local_and_uses_random_port() {
        assert_eq!(
            serve_command(&OsString::from("/opt/open code/bin/opencode")),
            "'/opt/open code/bin/opencode' serve --hostname 127.0.0.1 --port 0"
        );
    }
}

mod stats {
    use super::super::opencode_stats::opencode_stats_path;
    use std::path::Path;

    #[test]
    fn opencode_stats_path_sanitizes_project() {
        assert_eq!(
            opencode_stats_path("/Users/tim/my repo", Path::new("/tmp/stats"), 42),
            Path::new("/tmp/stats/Users_tim_my_repo-42.txt")
        );
    }
}

mod web {
    use super::super::opencode_web::web_command;
    use std::ffi::OsString;

    #[test]
    fn web_command_quotes_configured_opencode_command() {
        assert_eq!(
            web_command(&OsString::from("/opt/open code/bin/opencode")),
            "'/opt/open code/bin/opencode' web"
        );
    }
}
