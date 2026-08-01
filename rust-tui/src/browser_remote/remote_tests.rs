use super::{remote_ssh_command, validate_ssh_host, RemoteCommandRequest};

fn request(host: &str) -> RemoteCommandRequest {
    RemoteCommandRequest {
        host: host.into(),
        cwd: None,
        command: "npm test".into(),
    }
}

#[test]
fn remote_command_cd_quotes_cwd() {
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

#[test]
fn remote_command_cd_escapes_single_quotes() {
    let cmd = remote_ssh_command(&RemoteCommandRequest {
        host: "devbox".into(),
        cwd: Some("/tmp/bob's app".into()),
        command: "npm test".into(),
    })
    .expect("plain host is accepted");

    assert_eq!(cmd[3], r#"cd '/tmp/bob'\''s app' && npm test"#);
}

#[test]
fn remote_command_puts_separator_before_host() {
    let cmd = remote_ssh_command(&request("devbox")).expect("plain host is accepted");
    // `--` 必须排在 destination 前面,否则 ssh 会继续把 host 当选项解析。
    assert_eq!(cmd, vec!["ssh", "--", "devbox", "npm test"]);
}

#[test]
fn remote_command_rejects_proxy_command_injection() {
    let err = remote_ssh_command(&request("-oProxyCommand=touch /tmp/pwned"))
        .expect_err("option-shaped host must not reach ssh");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn remote_command_rejects_option_shaped_hosts() {
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

#[test]
fn remote_command_rejects_shell_metacharacters() {
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

#[test]
fn remote_command_accepts_normal_destinations() {
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

#[test]
fn validate_ssh_host_trims_surrounding_whitespace() {
    assert_eq!(validate_ssh_host("  devbox  ").expect("trimmed"), "devbox");
    let cmd = remote_ssh_command(&request(" devbox ")).expect("trimmed host is accepted");
    assert_eq!(cmd[2], "devbox");
}

#[test]
fn validate_ssh_host_rejects_bad_ports_and_overlong_input() {
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
