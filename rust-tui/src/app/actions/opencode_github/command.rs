use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::Command;

pub(super) fn install_github_agent(cwd: &Path, command: &OsString) -> io::Result<()> {
    let status = Command::new("tmux")
        .args(["new-window", "-c"])
        .arg(cwd)
        .arg(github_install_command(command))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "tmux new-window exited with {status}"
        )))
    }
}

pub(in crate::app::actions) fn github_install_command(command: &OsString) -> String {
    format!(
        "{} github install",
        crate::codex_runtime::shell_single_quote(&command.to_string_lossy())
    )
}
