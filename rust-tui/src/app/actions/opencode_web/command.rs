use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::Command;

pub(super) fn open_opencode_web(cwd: &Path, command: &OsString) -> io::Result<()> {
    let status = Command::new("tmux")
        .args(["new-window", "-c"])
        .arg(cwd)
        .arg(web_command(command))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "tmux new-window exited with {status}"
        )))
    }
}

pub(in crate::app::actions) fn web_command(command: &OsString) -> String {
    format!(
        "{} web",
        crate::codex_runtime::shell_single_quote(&command.to_string_lossy())
    )
}
