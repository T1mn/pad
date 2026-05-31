#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteCommandRequest {
    pub host: String,
    pub cwd: Option<String>,
    pub command: String,
}

pub fn remote_ssh_command(request: &RemoteCommandRequest) -> Vec<String> {
    let remote = match request.cwd.as_deref().filter(|cwd| !cwd.trim().is_empty()) {
        Some(cwd) => format!("cd {} && {}", shell_quote(cwd), request.command),
        None => request.command.clone(),
    };
    vec!["ssh".into(), request.host.clone(), remote]
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\\''"#))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
