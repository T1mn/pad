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
    let quote_count = value.matches('\'').count();
    let mut quoted = String::with_capacity(value.len() + 2 + quote_count * 3);
    quoted.push('\'');
    if quote_count == 0 {
        quoted.push_str(value);
    } else {
        for ch in value.chars() {
            if ch == '\'' {
                quoted.push_str(r#"'\''"#);
            } else {
                quoted.push(ch);
            }
        }
    }
    quoted.push('\'');
    quoted
}

#[cfg(test)]
#[path = "remote_tests.rs"]
mod tests;
