use std::io;
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserCommand {
    pub program: String,
    pub args: Vec<String>,
}

pub fn validate_browser_url(url: &str) -> bool {
    let trimmed = url.trim();
    trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("file://")
        || trimmed.starts_with("http://localhost:")
        || trimmed.starts_with("http://127.0.0.1:")
}

pub fn browser_open_command(url: &str) -> io::Result<BrowserCommand> {
    if !validate_browser_url(url) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "browser URL must start with http://, https://, or file://",
        ));
    }
    #[cfg(target_os = "macos")]
    {
        Ok(BrowserCommand {
            program: "open".into(),
            args: vec![url.to_string()],
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(BrowserCommand {
            program: "xdg-open".into(),
            args: vec![url.to_string()],
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
