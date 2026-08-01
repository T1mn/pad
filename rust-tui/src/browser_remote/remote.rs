use std::io;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteCommandRequest {
    pub host: String,
    pub cwd: Option<String>,
    pub command: String,
}

/// 足够覆盖 FQDN 与 `user@host:port`,再长的都不是合法 destination。
const MAX_HOST_LEN: usize = 255;

/// 构造远端执行用的 `ssh` argv。
///
/// `ssh` 先解析选项再取 destination,裸传 host 会让 `-oProxyCommand=...` 这类输入
/// 在**本地**执行任意命令。这里两道防线都要留:白名单挡掉畸形 host,`--` 终止选项解析。
pub fn remote_ssh_command(request: &RemoteCommandRequest) -> io::Result<Vec<String>> {
    let host = validate_ssh_host(&request.host)?;
    let remote = match request.cwd.as_deref().filter(|cwd| !cwd.trim().is_empty()) {
        Some(cwd) => format!("cd {} && {}", shell_quote(cwd), request.command),
        None => request.command.clone(),
    };
    Ok(vec!["ssh".into(), "--".into(), host.to_string(), remote])
}

/// 校验 `[user@]host[:port]`,host 也可以是 ssh_config 里的 Host 别名或裸 IPv6 字面量。
///
/// 允许的字符只有字母数字与 `. - _ @ :`,都不是 shell 元字符,因此结果既能直接 exec,
/// 也能被单引号包起来塞进 shell 命令行。
pub fn validate_ssh_host(host: &str) -> io::Result<&str> {
    let host = host.trim();
    if host.is_empty() || host.len() > MAX_HOST_LEN {
        return Err(reject("ssh host must be 1..=255 characters"));
    }
    let (user, target) = match host.split_once('@') {
        Some((user, target)) => (Some(user), target),
        None => (None, host),
    };
    if user.is_some_and(|user| !is_login_name(user)) {
        return Err(reject("ssh host has an invalid user part"));
    }
    if !is_target(target) {
        return Err(reject("ssh host has an invalid hostname part"));
    }
    Ok(host)
}

fn reject(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn is_login_name(user: &str) -> bool {
    !user.is_empty() && !user.starts_with('-') && user.chars().all(is_name_char)
}

fn is_target(target: &str) -> bool {
    match target.split_once(':') {
        // 裸 IPv6 字面量:冒号不止一个,后面也就没有端口后缀可言。
        Some(_) if target.matches(':').count() > 1 => is_ipv6(target),
        Some((name, port)) => is_hostname(name) && is_port(port),
        None => is_hostname(target),
    }
}

/// 主机名/别名:不能以 `-` 开头,否则 `ssh` 仍可能把它当选项(即便有 `--` 兜底)。
fn is_hostname(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('-') && name.chars().all(is_name_char)
}

fn is_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_')
}

fn is_ipv6(addr: &str) -> bool {
    addr.chars()
        .all(|ch| ch.is_ascii_hexdigit() || matches!(ch, ':' | '.'))
}

fn is_port(port: &str) -> bool {
    // `parse::<u16>` 自己会放行 `+80`,所以先卡死只能是数字。
    port.chars().all(|ch| ch.is_ascii_digit()) && port.parse::<u16>().is_ok_and(|port| port > 0)
}

fn shell_quote(value: &str) -> String {
    crate::shell_quote::single_quote(value)
}

#[cfg(test)]
#[path = "remote_tests.rs"]
mod tests;
