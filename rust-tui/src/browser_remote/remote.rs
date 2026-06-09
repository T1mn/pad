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
    crate::shell_quote::single_quote(value)
}

#[cfg(test)]
#[path = "remote_tests.rs"]
mod tests;
