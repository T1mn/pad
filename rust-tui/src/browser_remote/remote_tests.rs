use super::{remote_ssh_command, RemoteCommandRequest};

#[test]
fn remote_command_cd_quotes_cwd() {
    let cmd = remote_ssh_command(&RemoteCommandRequest {
        host: "devbox".into(),
        cwd: Some("/tmp/my app".into()),
        command: "npm test".into(),
    });
    assert_eq!(cmd[0], "ssh");
    assert_eq!(cmd[1], "devbox");
    assert_eq!(cmd[2], "cd '/tmp/my app' && npm test");
}

#[test]
fn remote_command_cd_escapes_single_quotes() {
    let cmd = remote_ssh_command(&RemoteCommandRequest {
        host: "devbox".into(),
        cwd: Some("/tmp/bob's app".into()),
        command: "npm test".into(),
    });

    assert_eq!(cmd[2], r#"cd '/tmp/bob'\''s app' && npm test"#);
}
