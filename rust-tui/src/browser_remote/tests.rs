pub(crate) mod browser {
    use super::super::browser::{browser_open_command, validate_browser_url};

    pub(crate) fn validates_safe_browser_urls() {
        assert!(validate_browser_url("http://localhost:3000"));
        assert!(validate_browser_url("https://example.com"));
        assert!(validate_browser_url("file:///tmp/report.html"));
        assert!(!validate_browser_url("javascript:alert(1)"));
    }

    pub(crate) fn rejects_option_shaped_and_control_char_urls() {
        for url in [
            "-a",
            "--args",
            " -e /etc/passwd",
            "http://example.com\nhttps://evil.test",
            "http://example.com\0",
            "http://example.com\u{1b}[2J",
        ] {
            assert!(!validate_browser_url(url), "url {url:?} must be rejected");
        }
    }

    pub(crate) fn open_command_uses_the_trimmed_url_it_validated() {
        let command =
            browser_open_command("  https://example.com  ").expect("trimmed url is valid");
        assert_eq!(command.args, vec!["https://example.com".to_string()]);
    }

    pub(crate) fn open_command_strips_trailing_newline_instead_of_passing_it_on() {
        // 收尾的 CRLF 由 trim 吃掉,所以子进程拿到的和校验过的是同一个串。
        let command =
            browser_open_command("https://example.com\r\n").expect("trailing crlf is valid");
        assert_eq!(command.args, vec!["https://example.com".to_string()]);
    }
}

pub(crate) mod cli {
    use super::super::cli::{format_args, format_command_line};

    pub(crate) fn command_line_format_includes_program_and_args() {
        assert_eq!(
            format_command_line("open", ["https://example.com", "--background"]),
            "open https://example.com --background"
        );
    }

    pub(crate) fn args_format_keeps_space_separated_remote_command() {
        let args = vec!["echo".to_string(), "hello".to_string()];
        assert_eq!(format_args(&args), "echo hello");
    }
}

pub(crate) mod remote {
    use super::super::remote::{remote_ssh_command, validate_ssh_host, RemoteCommandRequest};

    fn request(host: &str) -> RemoteCommandRequest {
        RemoteCommandRequest {
            host: host.into(),
            cwd: None,
            command: "npm test".into(),
        }
    }

    pub(crate) fn remote_command_cd_quotes_cwd() {
        let cmd = remote_ssh_command(&RemoteCommandRequest {
            host: "devbox".into(),
            cwd: Some("/tmp/my app".into()),
            command: "npm test".into(),
        })
        .expect("plain host is accepted");
        assert_eq!(cmd[0], "ssh");
        assert_eq!(cmd[1], "--");
        assert_eq!(cmd[2], "devbox");
        assert_eq!(cmd[3], "cd '/tmp/my app' && npm test");
    }

    pub(crate) fn remote_command_cd_escapes_single_quotes() {
        let cmd = remote_ssh_command(&RemoteCommandRequest {
            host: "devbox".into(),
            cwd: Some("/tmp/bob's app".into()),
            command: "npm test".into(),
        })
        .expect("plain host is accepted");

        assert_eq!(cmd[3], r#"cd '/tmp/bob'\''s app' && npm test"#);
    }

    pub(crate) fn remote_command_puts_separator_before_host() {
        let cmd = remote_ssh_command(&request("devbox")).expect("plain host is accepted");
        // `--` 必须排在 destination 前面,否则 ssh 会继续把 host 当选项解析。
        assert_eq!(cmd, vec!["ssh", "--", "devbox", "npm test"]);
    }

    pub(crate) fn remote_command_rejects_proxy_command_injection() {
        let err = remote_ssh_command(&request("-oProxyCommand=touch /tmp/pwned"))
            .expect_err("option-shaped host must not reach ssh");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    pub(crate) fn remote_command_rejects_option_shaped_hosts() {
        for host in [
            "-oProxyCommand=id",
            "-J",
            "--",
            "-",
            "root@-oProxyCommand=id",
            "-oProxyCommand=id@devbox",
        ] {
            assert!(
                remote_ssh_command(&request(host)).is_err(),
                "host {host:?} must be rejected"
            );
        }
    }

    pub(crate) fn remote_command_rejects_shell_metacharacters() {
        for host in [
            "dev box",
            "devbox;touch /tmp/pwned",
            "devbox`id`",
            "devbox$(id)",
            "devbox|id",
            "devbox&id",
            "dev\nbox",
            "devbox>out",
            "dev'box",
            "dev\"box",
            "devbox*",
            "host/../etc",
            "",
            "   ",
        ] {
            assert!(
                remote_ssh_command(&request(host)).is_err(),
                "host {host:?} must be rejected"
            );
        }
    }

    pub(crate) fn remote_command_accepts_normal_destinations() {
        for host in [
            "devbox",
            "dev-box_1.example.com",
            "root@devbox",
            "deploy.user@10.0.0.5",
            "devbox:2222",
            "root@devbox.example.com:22",
            "::1",
            "fe80::1",
        ] {
            let cmd = remote_ssh_command(&request(host)).unwrap_or_else(|err| {
                panic!("host {host:?} must be accepted, got {err}");
            });
            assert_eq!(cmd[2], host);
        }
    }

    pub(crate) fn validate_ssh_host_trims_surrounding_whitespace() {
        assert_eq!(validate_ssh_host("  devbox  ").expect("trimmed"), "devbox");
        let cmd = remote_ssh_command(&request(" devbox ")).expect("trimmed host is accepted");
        assert_eq!(cmd[2], "devbox");
    }

    pub(crate) fn validate_ssh_host_rejects_bad_ports_and_overlong_input() {
        for host in [
            "devbox:",
            "devbox:0",
            "devbox:+80",
            "devbox:99999",
            "@devbox",
        ] {
            assert!(
                validate_ssh_host(host).is_err(),
                "host {host:?} must be rejected"
            );
        }
        assert!(validate_ssh_host(&"a".repeat(256)).is_err());
        assert!(validate_ssh_host(&"a".repeat(255)).is_ok());
    }
}
