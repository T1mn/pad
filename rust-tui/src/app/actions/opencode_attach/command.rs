use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::Command;

pub(super) fn attach_opencode_server(url: &str, cwd: &Path, command: &OsString) -> io::Result<()> {
    let status = Command::new("tmux")
        .args(["new-window", "-c"])
        .arg(cwd)
        .arg(attach_command(url, command))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "tmux new-window exited with {status}"
        )))
    }
}

pub(in crate::app::actions) fn attach_command(url: &str, command: &OsString) -> String {
    format!(
        "{} attach {}",
        crate::codex_runtime::shell_single_quote(&command.to_string_lossy()),
        crate::codex_runtime::shell_single_quote(url)
    )
}
