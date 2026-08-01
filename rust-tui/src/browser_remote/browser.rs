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

#[cfg(test)]
#[path = "browser_tests.rs"]
mod tests;
