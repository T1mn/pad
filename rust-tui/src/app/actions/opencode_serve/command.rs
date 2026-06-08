use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::Command;

pub(super) fn serve_opencode(cwd: &Path, command: &OsString) -> io::Result<()> {
    let status = Command::new("tmux")
        .args(["new-window", "-c"])
        .arg(cwd)
        .arg(serve_command(command))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "tmux new-window exited with {status}"
        )))
    }
}

pub(in crate::app::actions) fn serve_command(command: &OsString) -> String {
    format!(
        "{} serve --hostname 127.0.0.1 --port 0",
        crate::codex_runtime::shell_single_quote(&command.to_string_lossy())
    )
}
