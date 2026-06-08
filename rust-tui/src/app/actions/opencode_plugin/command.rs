use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::Command;

pub(super) fn install_opencode_plugin(
    module: &str,
    cwd: &Path,
    command: &OsString,
) -> io::Result<()> {
    let status = Command::new("tmux")
        .args(["new-window", "-c"])
        .arg(cwd)
        .arg(plugin_command(module, command))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "tmux new-window exited with {status}"
        )))
    }
}

pub(in crate::app::actions) fn plugin_command(module: &str, command: &OsString) -> String {
    format!(
        "{} plugin {}",
        crate::codex_runtime::shell_single_quote(&command.to_string_lossy()),
        crate::codex_runtime::shell_single_quote(module)
    )
}
