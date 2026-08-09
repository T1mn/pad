mod browser {
    use std::io;
    use std::process::Command;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct BrowserCommand {
        pub program: String,
        pub args: Vec<String>,
    }

    /// scheme 白名单顺带保证了 URL 不可能以 `-` 开头,`open(1)`/`xdg-open(1)` 也就没法
    /// 把它当选项解析。控制字符在 URL 里从来都不合法,一并挡掉。
    pub fn validate_browser_url(url: &str) -> bool {
        let trimmed = url.trim();
        if trimmed.starts_with('-') || trimmed.chars().any(|ch| ch.is_ascii_control()) {
            return false;
        }
        trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("file://")
    }

    pub fn browser_open_command(url: &str) -> io::Result<BrowserCommand> {
        if !validate_browser_url(url) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "browser URL must start with http://, https://, or file://",
            ));
        }
        // 校验的是 trim 之后的串,交给子进程的也必须是同一个串,不然两边看到的东西不一样。
        let url = url.trim().to_string();
        #[cfg(target_os = "macos")]
        {
            Ok(BrowserCommand {
                program: "open".into(),
                args: vec![url],
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(BrowserCommand {
                program: "xdg-open".into(),
                args: vec![url],
            })
        }
    }

    pub fn open_browser_url(url: &str) -> io::Result<()> {
        let command = browser_open_command(url)?;
        let output = Command::new(&command.program)
            .args(&command.args)
            .output()?;
        if output.status.success() {
            return Ok(());
        }
        Err(io::Error::other(format!(
            "browser open failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}
mod cli {
    use super::{browser_open_command, open_browser_url, remote_ssh_command, RemoteCommandRequest};
    use std::error::Error;
    use std::process::Command;

    pub fn run_args(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn Error>> {
        let args: Vec<String> = args.into_iter().collect();
        match args.first().map(String::as_str) {
            Some("browser-open") => run_browser_open(&args[1..]),
            Some("remote-exec") => run_remote_exec(&args[1..]),
            Some(other) => Err(format!("unknown browser-remote command: {other}").into()),
            None => {
                Err("usage: pad __internal browser-remote browser-open <url> [--dry-run]".into())
            }
        }
    }

    fn run_browser_open(args: &[String]) -> Result<(), Box<dyn Error>> {
        let dry_run = args.iter().any(|arg| arg == "--dry-run");
        let url = args
            .iter()
            .find(|arg| !arg.starts_with("--"))
            .ok_or("missing url")?;
        if dry_run {
            let command = browser_open_command(url)?;
            println!(
                "{}",
                format_command_line(&command.program, command.args.iter().map(String::as_str))
            );
            return Ok(());
        }
        open_browser_url(url)?;
        println!("opened {url}");
        Ok(())
    }

    fn run_remote_exec(args: &[String]) -> Result<(), Box<dyn Error>> {
        let dry_run = args.iter().any(|arg| arg == "--dry-run");
        let host = value_after(args, "--host").ok_or("missing --host")?;
        let cwd = value_after(args, "--cwd");
        let sep = args
            .iter()
            .position(|arg| arg == "--")
            .ok_or("missing -- command")?;
        let command = format_args(&args[sep + 1..]);
        if command.trim().is_empty() {
            return Err("missing remote command".into());
        }
        let ssh = remote_ssh_command(&RemoteCommandRequest { host, cwd, command })?;
        if dry_run {
            println!("{}", format_args(&ssh));
            return Ok(());
        }
        let status = Command::new(&ssh[0]).args(&ssh[1..]).status()?;
        if status.success() {
            return Ok(());
        }
        Err(format!("remote exec failed with {status}").into())
    }

    fn value_after(args: &[String], key: &str) -> Option<String> {
        args.windows(2)
            .find(|pair| pair[0] == key)
            .map(|pair| pair[1].clone())
    }

    pub(super) fn format_args(args: &[String]) -> String {
        format_command_line("", args.iter().map(String::as_str))
    }

    pub(super) fn format_command_line<'a>(
        program: &str,
        args: impl IntoIterator<Item = &'a str>,
    ) -> String {
        let mut line = program.to_string();
        for arg in args {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(arg);
        }
        line
    }
}
mod remote {
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
}

pub use browser::{browser_open_command, open_browser_url};
pub use cli::run_args;
pub use remote::{remote_ssh_command, RemoteCommandRequest};

#[cfg(test)]
pub(crate) mod tests;
