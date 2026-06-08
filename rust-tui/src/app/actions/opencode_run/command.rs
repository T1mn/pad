use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::Command;

pub(super) fn run_opencode_prompt(
    prompt: &str,
    session_id: Option<&str>,
    cwd: &Path,
    command: &OsString,
) -> io::Result<()> {
    let status = Command::new("tmux")
        .args(["new-window", "-c"])
        .arg(cwd)
        .arg(run_command(prompt, session_id, command))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "tmux new-window exited with {status}"
        )))
    }
}

pub(in crate::app::actions) fn run_command(
    prompt: &str,
    session_id: Option<&str>,
    command: &OsString,
) -> String {
    let mut command_line = crate::codex_runtime::shell_single_quote(&command.to_string_lossy());
    command_line.push_str(" run");
    if let Some(session_id) = session_id {
        command_line.push_str(" --session ");
        command_line.push_str(&crate::codex_runtime::shell_single_quote(session_id));
    }
    command_line.push_str(" -- ");
    command_line.push_str(&crate::codex_runtime::shell_single_quote(prompt));
    command_line
}
